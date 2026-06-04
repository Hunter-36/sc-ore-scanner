@echo off
setlocal
cd /d "%~dp0"

echo ============================================================
echo   SC Ore Scanner - Setup  (run this once)
echo ============================================================
echo.
echo This installs the scanner's Python backend. It needs an
echo internet connection and downloads ~150 MB the first time.
echo.

REM --- Ensure uv (the Python installer/manager) is available ---
where uv >nul 2>nul
if %errorlevel%==0 (
    set "UV=uv"
) else (
    echo Installing uv ...
    powershell -ExecutionPolicy Bypass -Command "irm https://astral.sh/uv/install.ps1 | iex"
    set "UV=%USERPROFILE%\.local\bin\uv.exe"
)

echo.
echo Installing Python 3.11 + backend dependencies ...
cd backend
"%UV%" python install 3.11
"%UV%" venv --python 3.11
"%UV%" pip install -r requirements.txt
if errorlevel 1 (
    echo.
    echo [ERROR] Dependency installation failed. Check your internet
    echo         connection and the messages above, then re-run setup.bat
    echo.
    pause
    exit /b 1
)

echo.
echo ============================================================
echo   Calibration - opening the region selector...
echo ============================================================
echo   A full-screen window will open. Click and drag a box over
echo   the spot on your mining HUD where the RS number appears
echo   (the teal "10,620"-style readout), then release.
echo   Leave a little margin around the number. (ESC to skip.)
echo.
".venv\Scripts\python.exe" calibrate.py

echo.
echo Setup complete - start the app anytime with launch.bat
echo (This window closes automatically.)
timeout /t 4 /nobreak >nul
