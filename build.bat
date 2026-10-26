@echo off
setlocal enabledelayedexpansion

cd /d "%~dp0"

echo ========================================================
echo   Building M-RustHub (MCHOSE G3 V2) Clean Release
echo ========================================================
echo.

echo [1/3] Compiling release binary with Cargo...
cargo build --release
if errorlevel 1 (
    echo.
    echo [ERROR] Build failed! Check compiler output above.
    pause
    exit /b 1
)

echo [2/3] Preparing release folder...
if not exist "release" mkdir "release"

echo [3/3] Copying clean executable...
copy /y "target\release\m_rusthub.exe" "release\m_rusthub.exe" >nul
if errorlevel 1 (
    echo.
    echo [ERROR] Failed to copy binary to release\
    pause
    exit /b 1
)

for %%I in ("release\m_rusthub.exe") do set size=%%~zI

echo.
echo ========================================================
echo   SUCCESS! Clean release is ready:
echo   File: release\m_rusthub.exe
echo   Size: !size! bytes
echo ========================================================
echo.
pause
