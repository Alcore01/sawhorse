# sketchcraft E-24 : installation sous Windows, depuis le paquet prepare sous Linux
# (scripts/sketchcraft/package-windows.sh). Messages sans accents : PowerShell 5 lit ce
# fichier dans le codage de Windows.
#
#   installer.cmd              installe ou met a jour
#   installer.cmd -Retirer     retire les raccourcis (l'application installee reste)
#
# Ce qu'il fait :
#   1. verifie Node (22 ou plus) ; s'il manque, propose de l'installer avec winget ;
#   2. copie l'application dans %LOCALAPPDATA%\sketchcraft (la version precedente est gardee
#      dans sauvegardes\), puis verifie chaque fichier copie avec EMPREINTES.txt ;
#   3. cree le lanceur sketchcraft.cmd et les raccourcis du Bureau et du menu Demarrer.
param([switch]$Retirer)
$ErrorActionPreference = 'Stop'

$Src = Split-Path -Parent $MyInvocation.MyCommand.Path
$Dest = Join-Path $env:LOCALAPPDATA 'sketchcraft'
$Shortcuts = @(
  (Join-Path ([Environment]::GetFolderPath('Desktop')) 'sketchcraft.lnk'),
  (Join-Path ([Environment]::GetFolderPath('Programs')) 'sketchcraft.lnk')
)

function Say($t) { Write-Host ''; Write-Host "== $t" -ForegroundColor Cyan }
function Fail($t) { Write-Host ''; Write-Host "ERREUR : $t" -ForegroundColor Red; exit 1 }

if ($Retirer) {
  Say 'Retrait des raccourcis sketchcraft (vos modeles ne sont pas touches)'
  foreach ($s in $Shortcuts) { if (Test-Path $s) { Remove-Item $s; Write-Host "retire : $s" } }
  Write-Host "L'application reste dans $Dest ; supprimez ce dossier vous-meme si vous le souhaitez."
  exit 0
}

if (-not (Test-Path (Join-Path $Src 'dist\index.html'))) {
  Fail "pas d'application construite dans $Src\dist. Utilisez le paquet sketchcraft-windows-*.zip prepare sous Linux."
}

# ---- 1. Node ------------------------------------------------------------------------
Say '1/3 Node'
$node = Get-Command node -ErrorAction SilentlyContinue
if (-not $node) {
  Write-Host "Node n'est pas installe."
  $winget = Get-Command winget -ErrorAction SilentlyContinue
  if (-not $winget) { Fail "installez Node 22 ou plus depuis https://nodejs.org, puis relancez installer.cmd." }
  $a = Read-Host "Installer Node (version LTS) avec winget ? [o/N]"
  if ($a -notmatch '^[oOyY]') { Fail 'Node est necessaire pour lancer sketchcraft.' }
  winget install --id OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements
  $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
  $node = Get-Command node -ErrorAction SilentlyContinue
  if (-not $node) { Fail "Node vient d'etre installe mais n'est pas encore visible : fermez cette fenetre et relancez installer.cmd." }
}
$major = [int](& $node.Source -p "process.versions.node.split('.')[0]")
if ($major -lt 22) { Fail "Node $(& $node.Source -v) est trop ancien : il faut la version 22 ou plus." }
Write-Host "Node $(& $node.Source -v) : ok ($($node.Source))"

# ---- 2. Copie et verification ---------------------------------------------------------
Say "2/3 Copie dans $Dest"
New-Item -ItemType Directory -Force -Path $Dest | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
if (Test-Path (Join-Path $Dest 'dist')) {
  $keep = Join-Path $Dest "sauvegardes\$stamp"
  New-Item -ItemType Directory -Force -Path $keep | Out-Null
  foreach ($old in @('dist', 'serve.mjs', 'VERSION.txt', 'EMPREINTES.txt', 'sketchcraft.cmd')) {
    $p = Join-Path $Dest $old
    if (Test-Path $p) { Move-Item $p $keep }
  }
  Write-Host "version precedente gardee dans $keep"
}
Copy-Item -Recurse (Join-Path $Src 'dist') (Join-Path $Dest 'dist')
foreach ($f in @('serve.mjs', 'sketchcraft.ico', 'VERSION.txt', 'EMPREINTES.txt', 'LISEZMOI.txt')) {
  Copy-Item -Force (Join-Path $Src $f) (Join-Path $Dest $f)
}
$bad = 0; $n = 0
foreach ($line in Get-Content (Join-Path $Dest 'EMPREINTES.txt')) {
  if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { continue }
  $want = $Matches[1]; $rel = $Matches[2]
  if ($rel -in @('install-windows.ps1', 'installer.cmd')) { continue }
  $p = Join-Path $Dest ($rel -replace '/', '\')
  $n++
  if (-not (Test-Path $p) -or (Get-FileHash -Algorithm SHA256 $p).Hash.ToLower() -ne $want) {
    Write-Host "different ou absent : $rel" -ForegroundColor Red; $bad++
  }
}
if ($bad -gt 0) { Fail "$bad fichier(s) mal copie(s) sur $n : relancez installer.cmd." }
Write-Host "$n fichiers copies et verifies (empreintes SHA-256)."

# ---- 3. Lanceur et raccourcis ---------------------------------------------------------
Say '3/3 Lanceur et raccourcis'
$launcher = Join-Path $Dest 'sketchcraft.cmd'
$lines = @(
  '@echo off',
  'title sketchcraft',
  "`"$($node.Source)`" `"%~dp0serve.mjs`" `"%~dp0dist`"",
  'if errorlevel 1 pause'
)
Set-Content -Path $launcher -Value $lines -Encoding ASCII
Write-Host "lanceur : $launcher"

$sh = New-Object -ComObject WScript.Shell
foreach ($s in $Shortcuts) {
  $l = $sh.CreateShortcut($s)
  $l.TargetPath = $launcher
  $l.WorkingDirectory = $Dest
  $l.IconLocation = (Join-Path $Dest 'sketchcraft.ico') + ',0'
  $l.Description = 'VolumeCraft : modeleur 3D pour les volumetries de batiments'
  $l.WindowStyle = 7
  $l.Save()
  Write-Host "raccourci : $s"
}

Say 'Termine'
Write-Host "Lancez sketchcraft avec l'icone du Bureau ou du menu Demarrer."
Write-Host "Une petite fenetre (reduite) garde le serveur local ; l'application s'ouvre dans votre navigateur."
Write-Host "Fermer cette fenetre arrete sketchcraft."
