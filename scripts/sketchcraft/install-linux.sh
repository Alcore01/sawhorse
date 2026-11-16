#!/usr/bin/env bash
# sketchcraft E-24: installation sous Linux, depuis ce dossier de code (un clone de la
# branche sketchcraft).
#
#   bash scripts/sketchcraft/install-linux.sh            installe ou met à jour
#   bash scripts/sketchcraft/install-linux.sh --retirer  retire le lanceur et les icônes
#
# Ce qu'il fait, dans l'ordre (rien n'est installé sans le dire, rien n'est demandé à sudo) :
#   1. vérifie Node (22 ou plus) ; sans lui, dit comment l'installer et s'arrête ;
#   2. vérifie Rust (rustup), la cible wasm32 et wasm-pack ; propose d'installer ce qui manque ;
#   3. installe les dépendances (npm ci), compile le noyau (npm run wasm), construit l'appli
#      (npm run build) ;
#   4. crée le lanceur ~/.local/bin/sketchcraft, l'entrée du menu des applications et l'icône
#      du bureau. Le lanceur reconstruit l'appli d'elle-même après une mise à jour du code
#      (git pull), puis l'ouvre dans le navigateur.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN="$HOME/.local/bin"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/128x128/apps"
DESKTOP_DIR="$(xdg-user-dir DESKTOP 2>/dev/null || echo "$HOME/Desktop")"
STAMP="dist/.sketchcraft-build"

say() { printf '\n== %s\n' "$*"; }
fail() { printf '\nERREUR : %s\n' "$*" >&2; exit 1; }
ask() {
  local a
  read -r -p "$1 [o/N] " a || true
  [[ "$a" =~ ^[oOyY] ]]
}

if [[ "${1:-}" == "--retirer" ]]; then
  say "Retrait du lanceur sketchcraft (le code et vos modèles ne sont pas touchés)"
  for f in "$BIN/sketchcraft" "$APPS/sketchcraft.desktop" "$ICONS/sketchcraft.png" "$DESKTOP_DIR/sketchcraft.desktop"; do
    if [[ -e "$f" ]]; then rm -f -- "$f" && echo "retiré : $f"; fi
  done
  exit 0
fi

[[ -f "$REPO/package.json" && -d "$REPO/crates" ]] || fail "lancez ce script depuis le dossier de code de sketchcraft ($REPO ne l'est pas)."
cd "$REPO"

# ---- 1. Node --------------------------------------------------------------------------
say "1/4 Node"
if ! command -v node >/dev/null; then
  fail "Node n'est pas installé. Installez Node 22 ou plus (par ex. 'sudo apt install nodejs npm', ou https://nodejs.org), puis relancez."
fi
NODE_MAJOR="$(node -p 'process.versions.node.split(".")[0]')"
(( NODE_MAJOR >= 22 )) || fail "Node $(node -v) est trop ancien : il faut la version 22 ou plus."
echo "Node $(node -v) : ok"

# ---- 2. Rust, wasm32, wasm-pack --------------------------------------------------------
say "2/4 Rust et wasm-pack (pour compiler le noyau)"
# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
if ! command -v cargo >/dev/null || ! command -v rustup >/dev/null; then
  echo "Rust (rustup) n'est pas installé."
  ask "Installer Rust avec l'installateur officiel (https://sh.rustup.rs), dans votre dossier personnel ?" || fail "Rust est nécessaire pour compiler le noyau."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
command -v cc >/dev/null || fail "il manque le compilateur C (l'éditeur de liens). Installez-le avec 'sudo apt install build-essential', puis relancez."
if ! rustup target list --installed | grep -qx wasm32-unknown-unknown; then
  echo "Ajout de la cible wasm32…"
  rustup target add wasm32-unknown-unknown
fi
if ! command -v wasm-pack >/dev/null; then
  ask "wasm-pack manque. L'installer (cargo install wasm-pack, quelques minutes) ?" || fail "wasm-pack est nécessaire pour compiler le noyau."
  cargo install wasm-pack --locked
fi
echo "Rust $(rustc --version | cut -d' ' -f2), wasm-pack $(wasm-pack --version | cut -d' ' -f2) : ok"

# ---- 3. Dépendances, noyau, application ------------------------------------------------
say "3/4 Construction (dépendances, noyau, application)"
npm ci --no-audit --no-fund
npm run wasm
npm run build
git rev-parse HEAD > "$STAMP" 2>/dev/null || echo "sans-git" > "$STAMP"
[[ -f dist/index.html ]] || fail "la construction n'a pas produit dist/index.html."

# ---- 4. Lanceur, menu, bureau ----------------------------------------------------------
say "4/4 Lanceur et icônes"
mkdir -p "$BIN" "$APPS" "$ICONS"
NEW="$BIN/.sketchcraft.nouveau"
cat > "$NEW" <<LAUNCHER
#!/usr/bin/env bash
# Lanceur de sketchcraft (créé par scripts/sketchcraft/install-linux.sh).
REPO="$REPO"
cd "\$REPO" || { echo "sketchcraft : dossier introuvable : \$REPO"; read -r; exit 1; }
# shellcheck disable=SC1091
[[ -f "\$HOME/.cargo/env" ]] && source "\$HOME/.cargo/env"
want="\$(git rev-parse HEAD 2>/dev/null || echo sans-git)"
have="\$(cat $STAMP 2>/dev/null || true)"
if [[ ! -f dist/index.html || "\$want" != "\$have" ]]; then
  echo "sketchcraft : le code a changé, reconstruction (une à deux minutes)…"
  if [[ ! -d node_modules || package-lock.json -nt node_modules/.package-lock.json ]]; then
    npm ci --no-audit --no-fund || { echo "Échec de npm ci. Appuyez sur Entrée."; read -r; exit 1; }
  fi
  { npm run wasm && npm run build; } || { echo "Échec de la construction. Appuyez sur Entrée."; read -r; exit 1; }
  echo "\$want" > $STAMP
fi
exec node scripts/sketchcraft/serve.mjs "\$REPO/dist"
LAUNCHER
chmod 755 "$NEW"
[[ -f "$BIN/sketchcraft" ]] && cp -p "$BIN/sketchcraft" "$BIN/.sketchcraft.$(date +%Y%m%d-%H%M%S)"
mv -f "$NEW" "$BIN/sketchcraft"
echo "lanceur : $BIN/sketchcraft"

cp "$REPO/src-tauri/icons/128x128.png" "$ICONS/sketchcraft.png"

DESKTOP_FILE="$APPS/sketchcraft.desktop"
cat > "$DESKTOP_FILE" <<DESKTOP
[Desktop Entry]
Type=Application
Name=sketchcraft
Comment=Modeleur 3D pour les volumétries de bâtiments
Exec=$BIN/sketchcraft
Icon=$ICONS/sketchcraft.png
Terminal=true
Categories=Graphics;3DGraphics;Engineering;
StartupNotify=false
DESKTOP
chmod 755 "$DESKTOP_FILE"
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" >/dev/null 2>&1 || true
echo "menu des applications : $DESKTOP_FILE"

if [[ -d "$DESKTOP_DIR" ]]; then
  cp "$DESKTOP_FILE" "$DESKTOP_DIR/sketchcraft.desktop"
  chmod 755 "$DESKTOP_DIR/sketchcraft.desktop"
  # GNOME : un lanceur du bureau doit être marqué « de confiance » pour se lancer d'un clic.
  command -v gio >/dev/null && gio set "$DESKTOP_DIR/sketchcraft.desktop" metadata::trusted true 2>/dev/null || true
  echo "bureau : $DESKTOP_DIR/sketchcraft.desktop"
fi

say "Terminé"
echo "Lancez sketchcraft depuis le menu des applications, l'icône du bureau, ou la commande 'sketchcraft'."
echo "Une fenêtre de terminal s'ouvre avec le serveur local, et l'application s'ouvre dans votre navigateur."
echo "Fermer cette fenêtre arrête sketchcraft. Après un 'git pull', le lanceur reconstruit tout seul."
[[ ":$PATH:" == *":$BIN:"* ]] || echo "Note : $BIN n'est pas dans votre PATH ; la commande 'sketchcraft' n'y sera qu'après une reconnexion."
