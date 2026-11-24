@echo off
rem VolumeCraft: double-click to start (sketchcraft E-60). Runs VolumeCraft.ps1 beside it.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0VolumeCraft.ps1" %*
