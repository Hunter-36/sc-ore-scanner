@echo off
cd /d "%~dp0"

if not exist "backend\.venv\Scripts\python.exe" (
    echo Backend is not set up yet.
    echo Please run  setup.bat  first.
    echo.
    pause
    exit /b 1
)

echo Starting SC Ore Scanner...

REM Start the Python backend in its own window
start "SC Ore Scanner - Backend" cmd /k "cd backend && .venv\Scripts\python.exe main.py"

REM Give the backend a moment to come up, then launch the overlay
timeout /t 4 /nobreak >nul
start "" "SC Ore Scanner.exe"

echo.
echo Backend and overlay launched in separate windows.
echo - The overlay sits top-right, always on top of the game.
echo - If it says OFFLINE, give the backend ~10s to load, then it
echo   will switch to READY / SCANNING.
echo - To stop: close the backend window and the overlay.
echo.
echo You can close THIS window now.
timeout /t 6 /nobreak >nul
