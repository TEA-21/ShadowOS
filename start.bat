@echo off
setlocal
title ShadowOS Platform Launcher
cd /d "%~dp0"

echo =======================================================================
echo  Starting ShadowOS Full-Stack Platform...
echo =======================================================================

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0start.ps1"

if %ERRORLEVEL% neq 0 (
    echo.
    echo [!] Launcher exited with error code %ERRORLEVEL%.
    pause
)
