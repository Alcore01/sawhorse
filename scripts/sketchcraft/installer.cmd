@echo off
rem sketchcraft E-24 : double-cliquez ici pour installer sketchcraft sous Windows.
rem Lance install-windows.ps1 sans changer la politique d'execution de PowerShell.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install-windows.ps1" %*
echo.
pause
