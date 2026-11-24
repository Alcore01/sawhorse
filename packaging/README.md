# Starting VolumeCraft from a checkout (sketchcraft E-60)

Launchers and a per-user install for Linux and Windows, for running VolumeCraft from this
source checkout without typing npm commands. (The desktop installers, .msi / .exe / .deb /
.AppImage, are built by `.github/workflows/release.yml`.)

Each launcher: checks Node.js (20 or later), installs the packages when needed, builds what
is out of date, serves VolumeCraft on http://127.0.0.1:4173 and opens the browser. Started
again while it runs, it only opens the browser. The first start takes a few minutes (packages,
build); the next ones take seconds.

## Linux

```sh
packaging/linux/install.sh          # menu entry (VolumeCraft, Stop VolumeCraft), icon, ~/.local/bin/volumecraft
packaging/linux/volumecraft.sh      # or start it directly
packaging/linux/volumecraft.sh --stop
packaging/linux/install.sh --uninstall
```

On Linux the launcher also rebuilds the kernel (`npm run wasm`) when the Rust sources are
newer than it, if Rust and wasm-pack are installed; if that fails it keeps the kernel already
built and says so (and does not retry until the sources change again).

## Windows

Double-click `packaging\windows\Install VolumeCraft.cmd` (Start menu and desktop shortcuts:
VolumeCraft, Stop VolumeCraft), or `packaging\windows\VolumeCraft.cmd` to start it directly.
`install.ps1 -Uninstall` takes the shortcuts away.

The Windows launcher does not build the kernel: `src\wasm\pkg` must exist (`npm run wasm` with
Rust and wasm-pack, or copied from a Linux machine that built it).

## What they write

Nothing in the checkout but the build outputs (`dist/`, `src/wasm/pkg/`). Per user: the log and
the server's process id (`~/.local/state/volumecraft/`, `%LOCALAPPDATA%\VolumeCraft\`), the menu
entry, icon and link (Linux) or the shortcuts (Windows). A file already in place is first copied
to a dated `backup-…` folder there; uninstalling moves the files to such a folder instead of
deleting them.
