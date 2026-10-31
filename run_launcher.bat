@echo off
setlocal enabledelayedexpansion
title Horizon Launcher

cd /d "%~dp0"

echo ========================================================
echo               Horizon Launcher (Clean & Free)
echo ========================================================
echo.

:: 1. Locate precompiled binary if already built or downloaded
set "EXE_PATH="
if exist "%~dp0horizon-launcher.exe" set "EXE_PATH=%~dp0horizon-launcher.exe"
if exist "launcher\target\release\horizon-launcher.exe" set "EXE_PATH=launcher\target\release\horizon-launcher.exe"
if exist "target\release\horizon-launcher.exe" set "EXE_PATH=target\release\horizon-launcher.exe"

if defined EXE_PATH (
    if not "%~1"=="--rebuild" (
        echo [INFO] Starting Horizon Launcher from !EXE_PATH!...
        start "" "!EXE_PATH!"
        exit /b 0
    )
)

:: 2. Add cargo to PATH if installed in user profile
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

:: 3. Check if Rust / Cargo is available (only needed to compile from source)
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust / Cargo is not found!
    echo.
    echo To build Horizon Launcher from source, Rust is required:
    echo   winget install Rustlang.Rustup
    echo.
    echo Or download a precompiled release (.zip / .exe) from:
    echo   https://github.com/YaroslavSamchuk/CleanHorizon/releases
    echo.
    pause
    exit /b 1
)

:: Compile launcher in release mode
echo [INFO] Building Horizon Launcher (Release mode)...
echo [INFO] This may take a few minutes on the first run. Please wait...
echo.

cargo build --release --manifest-path launcher/Cargo.toml

if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Build failed. Please check the compiler errors above.
    pause
    exit /b %errorlevel%
)

:: Find the newly built executable
if exist "launcher\target\release\horizon-launcher.exe" (
    set "EXE_PATH=launcher\target\release\horizon-launcher.exe"
) else if exist "target\release\horizon-launcher.exe" (
    set "EXE_PATH=target\release\horizon-launcher.exe"
)

if defined EXE_PATH (
    echo.
    echo [INFO] Build completed successfully! Starting Horizon Launcher...
    start "" "!EXE_PATH!"
) else (
    echo [ERROR] Executable not found after build.
    pause
)
