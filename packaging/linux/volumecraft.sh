#!/usr/bin/env bash
# VolumeCraft launcher for Linux (sketchcraft E-60).
#
# Starts VolumeCraft from this checkout and opens it in the browser:
#   1. checks Node.js (20 or later);
#   2. installs the JavaScript packages when they are missing or package-lock.json changed;
#   3. builds the kernel (npm run wasm) when it is missing or the Rust sources are newer,
#      if Rust and wasm-pack are installed (otherwise says what to do);
#   4. builds the app (npm run build) when it is missing or the sources are newer;
#   5. serves it on http://127.0.0.1:4173 (VOLUMECRAFT_PORT to change) and opens the browser.
# If VolumeCraft is already running, it only opens the browser.
#
#   volumecraft.sh           start (or show) VolumeCraft
#   volumecraft.sh --stop    stop the server this launcher started
#   volumecraft.sh --status  say whether it runs
#
# Nothing in the checkout is overwritten except what the build makes (dist/, src/wasm/pkg/),
# which are build outputs. The log is in ~/.local/state/volumecraft/launcher.log.
set -euo pipefail

SELF="$(readlink -f "${BASH_SOURCE[0]}")"
ROOT="$(cd "$(dirname "$SELF")/../.." && pwd)"
PORT="${VOLUMECRAFT_PORT:-4173}"
URL="http://127.0.0.1:${PORT}/"
STATE="${XDG_STATE_HOME:-$HOME/.local/state}/volumecraft"
LOG="$STATE/launcher.log"
PIDFILE="$STATE/server.pid"
mkdir -p "$STATE"

say() { printf 'VolumeCraft: %s\n' "$*"; printf '%s %s\n' "$(date '+%F %T')" "$*" >>"$LOG"; }
fail() {
  say "$*"
  command -v notify-send >/dev/null 2>&1 && notify-send "VolumeCraft" "$*" || true
  exit 1
}
up() { curl -fsS -o /dev/null --max-time 2 "$URL" 2>/dev/null; }
open_browser() {
  if [ -n "${BROWSER:-}" ]; then "$BROWSER" "$URL" >/dev/null 2>&1 &
  elif command -v xdg-open >/dev/null 2>&1; then xdg-open "$URL" >/dev/null 2>&1 &
  elif command -v firefox >/dev/null 2>&1; then firefox "$URL" >/dev/null 2>&1 &
  else say "Open $URL in your browser."
  fi
}

case "${1:-}" in
  --stop)
    if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
      kill -- "-$(cat "$PIDFILE")" 2>/dev/null || kill "$(cat "$PIDFILE")"
      rm -f "$PIDFILE"
      say "stopped."
    else
      say "not running (from this launcher)."
    fi
    exit 0
    ;;
  --status)
    if up; then say "running at $URL"; else say "not running."; fi
    exit 0
    ;;
  "" ) ;;
  *) fail "unknown option $1 (use --stop or --status)" ;;
esac

command -v curl >/dev/null 2>&1 || fail "curl is needed (sudo apt install curl)."

if up; then
  say "already running: opening $URL"
  open_browser
  exit 0
fi

command -v node >/dev/null 2>&1 || fail "Node.js 20 or later is needed: https://nodejs.org (or sudo apt install nodejs npm)."
NODE_MAJOR="$(node -p 'process.versions.node.split(".")[0]')"
[ "$NODE_MAJOR" -ge 20 ] || fail "Node.js $NODE_MAJOR is too old: 20 or later is needed."

cd "$ROOT"
[ -f package.json ] || fail "no package.json in $ROOT: is this launcher inside the VolumeCraft checkout?"

# 2. Packages.
if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  say "installing the JavaScript packages (npm ci)…"
  npm ci >>"$LOG" 2>&1 || fail "npm ci failed: see $LOG"
fi

# 3. Kernel.
WASM="src/wasm/pkg/sawhorse_wasm_bg.wasm"
if [ ! -f "$WASM" ] || [ -n "$(find crates -name '*.rs' -newer "$WASM" -print -quit 2>/dev/null)" ]; then
  FAILED="$STATE/kernel-build-failed"
  # A rebuild that failed is not tried again on every start, only when the sources change
  # again (a later successful build leaves the kernel newer than this mark).
  if [ -f "$WASM" ] && [ -f "$FAILED" ] && [ "$FAILED" -nt "$WASM" ] \
    && [ -z "$(find crates -name '*.rs' -newer "$FAILED" -print -quit 2>/dev/null)" ]; then
    say "the kernel did not rebuild last time and its sources have not changed since: using the kernel already built."
  elif command -v cargo >/dev/null 2>&1 && command -v wasm-pack >/dev/null 2>&1; then
    say "building the kernel (npm run wasm, a few minutes)…"
    if ! npm run wasm >>"$LOG" 2>&1; then
      [ -f "$WASM" ] || fail "npm run wasm failed: see $LOG"
      say "rebuilding the kernel failed (see $LOG): using the kernel already built."
      touch "$FAILED"
    fi
  elif [ -f "$WASM" ]; then
    say "the kernel sources changed but Rust or wasm-pack is missing: using the kernel already built."
  else
    fail "the kernel is not built and Rust or wasm-pack is missing (see CONTRIBUTING.md), or copy src/wasm/pkg from a machine that built it."
  fi
fi

# 4. App.
if [ ! -f dist/index.html ] || [ "$WASM" -nt dist/index.html ] \
  || [ -n "$(find src public index.html package.json -newer dist/index.html -print -quit 2>/dev/null)" ]; then
  say "building the app (npm run build)…"
  npm run build >>"$LOG" 2>&1 || fail "npm run build failed: see $LOG"
fi

# 5. Serve and open. The server runs in its own process group so --stop ends all of it.
say "starting at $URL"
setsid npx vite preview --host 127.0.0.1 --port "$PORT" --strictPort >>"$LOG" 2>&1 &
echo $! >"$PIDFILE"
for _ in $(seq 1 60); do
  up && break
  kill -0 "$(cat "$PIDFILE")" 2>/dev/null || fail "the server stopped while starting: see $LOG"
  sleep 0.5
done
up || fail "the server did not answer at $URL: see $LOG"
open_browser
say "open at $URL. volumecraft.sh --stop (or the menu's Stop VolumeCraft) stops it."
