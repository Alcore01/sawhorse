#!/usr/bin/env bash
# sketchcraft E-24: prépare sous Linux le paquet d'installation pour Windows.
#
#   bash scripts/sketchcraft/package-windows.sh [dossier de sortie]
#
# Le noyau (WebAssembly) et l'application construite sont les mêmes sous Windows : on les
# construit ici, et le PC Windows n'a besoin que de Node (pas de Rust, pas de Visual Studio).
# Le paquet sketchcraft-windows-<version>.zip contient le dossier sketchcraft\ : dist\
# (l'application), serve.mjs (le serveur local), installer.cmd et install-windows.ps1,
# l'icône, LISEZMOI.txt et VERSION.txt. Sortie par défaut : le dossier parent du code.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="${1:-$(dirname "$REPO")}"
cd "$REPO"
STAMP="dist/.sketchcraft-build"
want="$(git rev-parse HEAD 2>/dev/null || echo sans-git)"
have="$(cat "$STAMP" 2>/dev/null || true)"

if [[ ! -f dist/index.html || "$want" != "$have" ]]; then
  echo "== Construction de l'application (version $want)"
  # shellcheck disable=SC1091
  [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
  npm run wasm
  npm run build
  echo "$want" > "$STAMP"
fi

short="$(git rev-parse --short HEAD 2>/dev/null || echo local)"
dirty=""
git diff --quiet HEAD -- 2>/dev/null || dirty="-modifie"
ZIP="$OUT/sketchcraft-windows-$short$dirty.zip"
[[ -e "$ZIP" ]] && mv "$ZIP" "$ZIP.$(date +%Y%m%d-%H%M%S)"

echo "== Paquet : $ZIP"
python3 - "$REPO" "$ZIP" "$want$dirty" <<'PY'
import os, sys, zipfile, hashlib
repo, zpath, version = sys.argv[1:4]
here = os.path.join(repo, "scripts", "sketchcraft")
crlf = lambda s: s.replace("\r\n", "\n").replace("\n", "\r\n").encode("utf-8")
files = {}
for base, _, names in os.walk(os.path.join(repo, "dist")):
    for n in names:
        if n == ".sketchcraft-build":
            continue
        full = os.path.join(base, n)
        files["sketchcraft/dist/" + os.path.relpath(full, os.path.join(repo, "dist")).replace(os.sep, "/")] = open(full, "rb").read()
files["sketchcraft/serve.mjs"] = open(os.path.join(here, "serve.mjs"), "rb").read()
files["sketchcraft/install-windows.ps1"] = crlf(open(os.path.join(here, "install-windows.ps1"), encoding="utf-8").read())
files["sketchcraft/installer.cmd"] = crlf(open(os.path.join(here, "installer.cmd"), encoding="utf-8").read())
files["sketchcraft/LISEZMOI.txt"] = crlf(open(os.path.join(here, "LISEZMOI-windows.txt"), encoding="utf-8").read())
files["sketchcraft/sketchcraft.ico"] = open(os.path.join(repo, "src-tauri", "icons", "icon.ico"), "rb").read()
files["sketchcraft/VERSION.txt"] = crlf(version + "\n")
# The list of every file and its SHA-256: the installer checks the copy against it.
manifest = "".join(f"{hashlib.sha256(d).hexdigest()}  {n[len('sketchcraft/'):]}\n" for n, d in sorted(files.items()))
files["sketchcraft/EMPREINTES.txt"] = crlf(manifest)
with zipfile.ZipFile(zpath, "w", zipfile.ZIP_DEFLATED) as z:
    for n, d in sorted(files.items()):
        z.writestr(n, d)
print(f"{len(files)} fichiers")
PY
sha256sum "$ZIP"
echo "Copiez ce .zip sur le PC Windows, décompressez-le (par ex. sur le Bureau), puis double-cliquez sur sketchcraft\\installer.cmd."
