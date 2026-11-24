#!/usr/bin/env bash
# VolumeCraft install for Linux, from this checkout (sketchcraft E-60).
#
#   install.sh              add VolumeCraft to the applications menu, with its icon, and a
#                           `volumecraft` command in ~/.local/bin
#   install.sh --uninstall  take them away again (moved aside, not deleted: see below)
#
# For this user only; no sudo. What it writes:
#   ~/.local/share/applications/volumecraft.desktop     the menu entry (Start, Stop)
#   ~/.local/share/icons/hicolor/128x128/apps/volumecraft.png
#   ~/.local/bin/volumecraft                            a link to packaging/linux/volumecraft.sh
# A file already there is first copied to ~/.local/state/volumecraft/backup-<date>/, never
# overwritten without a copy; --uninstall moves the files there too.
set -euo pipefail

SELF="$(readlink -f "${BASH_SOURCE[0]}")"
HERE="$(dirname "$SELF")"
ROOT="$(cd "$HERE/../.." && pwd)"
LAUNCHER="$HERE/volumecraft.sh"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/128x128/apps"
BIN="$HOME/.local/bin"
BACKUP="${XDG_STATE_HOME:-$HOME/.local/state}/volumecraft/backup-$(date +%Y%m%d-%H%M%S)"
DESKTOP="$APPS/volumecraft.desktop"
ICON="$ICONS/volumecraft.png"
LINK="$BIN/volumecraft"

keep() { # copy an existing file (or link) aside before it changes
  if [ -e "$1" ] || [ -L "$1" ]; then
    mkdir -p "$BACKUP"
    cp -P "$1" "$BACKUP/"
    echo "  kept a copy of $1 in $BACKUP"
  fi
}

if [ "${1:-}" = "--uninstall" ]; then
  for f in "$DESKTOP" "$ICON" "$LINK"; do
    if [ -e "$f" ] || [ -L "$f" ]; then
      mkdir -p "$BACKUP"
      mv "$f" "$BACKUP/"
      echo "  moved $f to $BACKUP"
    fi
  done
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" 2>/dev/null || true
  echo "VolumeCraft is off the menu. The checkout ($ROOT) is untouched; delete $BACKUP when you like."
  exit 0
fi

[ -x "$LAUNCHER" ] || chmod +x "$LAUNCHER"
mkdir -p "$APPS" "$ICONS" "$BIN"

keep "$ICON"
cp "$ROOT/src-tauri/icons/128x128.png" "$ICON"

keep "$LINK"
ln -sfn "$LAUNCHER" "$LINK"

keep "$DESKTOP"
cat >"$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=VolumeCraft
GenericName=Building massing modeler
Comment=Model building volumes (push/pull), measure areas and volumes
Exec="$LAUNCHER"
Icon=volumecraft
Terminal=false
Categories=Graphics;3DGraphics;Engineering;
Keywords=3D;volume;building;PEB;SketchUp;
Actions=stop;

[Desktop Action stop]
Name=Stop VolumeCraft
Exec="$LAUNCHER" --stop
EOF
chmod +x "$DESKTOP"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APPS" 2>/dev/null || true

echo "VolumeCraft is in the applications menu (and 'volumecraft' in a terminal if ~/.local/bin is on your PATH)."
echo "The first start installs packages and builds the app: a few minutes; the next ones take seconds."
