# VolumeCraft launcher for Windows (sketchcraft E-60).
#
# Starts VolumeCraft from this checkout and opens it in the browser:
#   1. checks Node.js (20 or later);
#   2. installs the JavaScript packages when they are missing or package-lock.json changed;
#   3. builds the app (npm run build) when it is missing or the sources are newer
#      (the kernel, src\wasm\pkg, must have been built: npm run wasm, or copied from Linux);
#   4. serves it on http://127.0.0.1:4173 (-Port to change), hidden, and opens the browser.
# If VolumeCraft is already running, it only opens the browser.
#
#   VolumeCraft.ps1          start (or show) VolumeCraft
#   VolumeCraft.ps1 -Stop    stop the server this launcher started
#
# The log is in %LOCALAPPDATA%\VolumeCraft\launcher.log.
param([switch]$Stop, [int]$Port = 4173)
$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$Url = "http://127.0.0.1:$Port/"
$State = Join-Path $env:LOCALAPPDATA "VolumeCraft"
$Log = Join-Path $State "launcher.log"
$PidFile = Join-Path $State "server.pid"
New-Item -ItemType Directory -Force -Path $State | Out-Null

function Say([string]$Text) {
  Write-Host "VolumeCraft: $Text"
  Add-Content -Path $Log -Value ("{0} {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $Text)
}
function Fail([string]$Text) {
  Say $Text
  Add-Type -AssemblyName PresentationFramework
  [System.Windows.MessageBox]::Show($Text, "VolumeCraft") | Out-Null
  exit 1
}
function Up {
  try { Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2 | Out-Null; return $true } catch { return $false }
}
function Newer([string]$Than, [string[]]$Paths) {
  $t = (Get-Item $Than).LastWriteTime
  foreach ($p in $Paths) {
    $full = Join-Path $Root $p
    if (Test-Path $full) {
      $hit = Get-ChildItem -Path $full -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTime -gt $t } | Select-Object -First 1
      if ($hit) { return $true }
    }
  }
  return $false
}

if ($Stop) {
  if (Test-Path $PidFile) {
    $id = Get-Content $PidFile
    & taskkill.exe /PID $id /T /F 2>$null | Out-Null
    Remove-Item $PidFile
    Say "stopped."
  } else { Say "not running (from this launcher)." }
  exit 0
}

if (Up) { Say "already running: opening $Url"; Start-Process $Url; exit 0 }

if (-not (Get-Command node -ErrorAction SilentlyContinue)) { Fail "Node.js 20 or later is needed: https://nodejs.org" }
$major = [int](& node -p "process.versions.node.split('.')[0]")
if ($major -lt 20) { Fail "Node.js $major is too old: 20 or later is needed." }

Set-Location $Root
if (-not (Test-Path "package.json")) { Fail "no package.json in ${Root}: is this launcher inside the VolumeCraft checkout?" }

# 2. Packages.
$lock = "node_modules\.package-lock.json"
if (-not (Test-Path "node_modules") -or -not (Test-Path $lock) -or ((Get-Item "package-lock.json").LastWriteTime -gt (Get-Item $lock).LastWriteTime)) {
  Say "installing the JavaScript packages (npm ci)..."
  & cmd.exe /c "npm ci >> `"$Log`" 2>&1"
  if ($LASTEXITCODE -ne 0) { Fail "npm ci failed: see $Log" }
}

# 3. App (the kernel must be there).
$Wasm = "src\wasm\pkg\sawhorse_wasm_bg.wasm"
if (-not (Test-Path $Wasm)) {
  Fail "the kernel is not built (src\wasm\pkg). Build it with 'npm run wasm' (Rust and wasm-pack), or copy src\wasm\pkg from the Linux machine."
}
if (-not (Test-Path "dist\index.html") -or ((Get-Item $Wasm).LastWriteTime -gt (Get-Item "dist\index.html").LastWriteTime) -or (Newer "dist\index.html" @("src", "public", "index.html", "package.json"))) {
  Say "building the app (npm run build)..."
  & cmd.exe /c "npm run build >> `"$Log`" 2>&1"
  if ($LASTEXITCODE -ne 0) { Fail "npm run build failed: see $Log" }
}

# 4. Serve (hidden) and open.
Say "starting at $Url"
$p = Start-Process -FilePath "cmd.exe" -ArgumentList "/c npx vite preview --host 127.0.0.1 --port $Port --strictPort >> `"$Log`" 2>&1" -WorkingDirectory $Root -WindowStyle Hidden -PassThru
Set-Content -Path $PidFile -Value $p.Id
for ($i = 0; $i -lt 60; $i++) {
  if (Up) { break }
  if ($p.HasExited) { Fail "the server stopped while starting: see $Log" }
  Start-Sleep -Milliseconds 500
}
if (-not (Up)) { Fail "the server did not answer at ${Url}: see $Log" }
Start-Process $Url
Say "open at $Url. The 'Stop VolumeCraft' shortcut stops it."
