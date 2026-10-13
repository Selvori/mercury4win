@echo off
setlocal EnableExtensions DisableDelayedExpansion
chcp 65001 >nul
title mercury4win - Development

pushd "%~dp0" >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Cannot open the project directory.
    goto :failure
)
set "PROJECT_DIR_PUSHED=1"

if not exist "package.json" (
    echo [ERROR] package.json is missing from the project directory.
    goto :failure
)

for %%T in (node.exe npm.cmd cargo.exe rustc.exe) do (
    where %%T >nul 2>&1
    if errorlevel 1 (
        echo [ERROR] %%T is not available in PATH.
        echo Install Node.js and Rust, then open a new terminal.
        goto :failure
    )
)

if not exist "node_modules\.bin\tauri.cmd" (
    echo [ERROR] Project dependencies are missing.
    echo Run: npm.cmd ci --legacy-peer-deps
    goto :failure
)

echo Starting mercury4win in desktop development mode...
echo First-time Rust compilation may take a few minutes.
echo Press Ctrl+C in this window to stop.
echo.
call npm.cmd run tauri -- dev %*
set "START_EXIT_CODE=%ERRORLEVEL%"

if not "%START_EXIT_CODE%"=="0" (
    echo.
    echo [ERROR] Startup stopped with exit code %START_EXIT_CODE%.
    pause
)
popd
exit /b %START_EXIT_CODE%

:failure
if defined PROJECT_DIR_PUSHED popd
pause
exit /b 1
