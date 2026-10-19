@echo off
rem Entry point for cmd and PowerShell on Windows: runs bin\redline.ps1 (see there).
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%~dp0redline.ps1" %*
