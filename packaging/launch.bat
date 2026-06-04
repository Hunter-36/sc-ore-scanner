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
echo - FIRST START: the backend loads its OCR models, which can take
echo   ~15-20s. The backend window shows "OCR engine ready" when it's
echo   set; detection won't work until then. This is normal.
echo - To stop: click the X on the overlay, then close the backend window.
echo.
echo You can close THIS window now.
timeout /t 6 /nobreak >nul
