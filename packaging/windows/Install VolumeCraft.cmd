@echo off
rem VolumeCraft: double-click to add the shortcuts (sketchcraft E-60). Runs install.ps1 beside it.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*
pause
