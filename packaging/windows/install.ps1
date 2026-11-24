# VolumeCraft install for Windows, from this checkout (sketchcraft E-60).
#
#   install.ps1              shortcuts in the Start menu and on the desktop: VolumeCraft and
#                            Stop VolumeCraft, with the app's icon
#   install.ps1 -Uninstall   take them away again (moved aside, not deleted)
#
# For this user only; no administrator rights. A shortcut already there is first copied to
# %LOCALAPPDATA%\VolumeCraft\backup-<date>\, never overwritten without a copy; -Uninstall
# moves the shortcuts there too.
param([switch]$Uninstall)
$ErrorActionPreference = "Stop"

$Here = $PSScriptRoot
$Root = (Resolve-Path (Join-Path $Here "..\..")).Path
$Launcher = Join-Path $Here "VolumeCraft.ps1"
$Icon = Join-Path $Root "src-tauri\icons\icon.ico"
$StartMenu = Join-Path ([Environment]::GetFolderPath("Programs")) "VolumeCraft"
$Desktop = [Environment]::GetFolderPath("Desktop")
$Backup = Join-Path $env:LOCALAPPDATA ("VolumeCraft\backup-" + (Get-Date -Format "yyyyMMdd-HHmmss"))
$Links = @(
  @{ Path = Join-Path $StartMenu "VolumeCraft.lnk"; Args = ""; Name = "VolumeCraft" },
  @{ Path = Join-Path $StartMenu "Stop VolumeCraft.lnk"; Args = " -Stop"; Name = "Stop VolumeCraft" },
  @{ Path = Join-Path $Desktop "VolumeCraft.lnk"; Args = ""; Name = "VolumeCraft" }
)

function Keep([string]$Path, [switch]$Move) {
  if (Test-Path $Path) {
    New-Item -ItemType Directory -Force -Path $Backup | Out-Null
    if ($Move) { Move-Item $Path $Backup } else { Copy-Item $Path $Backup }
    Write-Host "  $(if ($Move) { 'moved' } else { 'kept a copy of' }) $Path -> $Backup"
  }
}

if ($Uninstall) {
  foreach ($l in $Links) { Keep $l.Path -Move }
  Write-Host "VolumeCraft's shortcuts are off. The checkout ($Root) is untouched; delete $Backup when you like."
  exit 0
}

New-Item -ItemType Directory -Force -Path $StartMenu | Out-Null
$shell = New-Object -ComObject WScript.Shell
foreach ($l in $Links) {
  Keep $l.Path
  $s = $shell.CreateShortcut($l.Path)
  $s.TargetPath = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
  $s.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$Launcher`"$($l.Args)"
  $s.WorkingDirectory = $Root
  $s.Description = $l.Name
  if (Test-Path $Icon) { $s.IconLocation = "$Icon,0" }
  $s.Save()
}
Write-Host "VolumeCraft is in the Start menu and on the desktop."
Write-Host "The first start installs packages and builds the app: a few minutes; the next ones take seconds."
