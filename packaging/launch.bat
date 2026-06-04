@echo off
cd /d "%~dp0"

if not exist "backend\.venv\Scripts\pythonw.exe" (
    echo Backend is not set up yet. Please run setup.bat first.
    echo.
    pause
    exit /b 1
)

REM Start the backend with pythonw.exe = no console window. It logs to
REM logs\scanner.log. Then launch the overlay and exit immediately so no
REM command window lingers. All status shows on the overlay itself.
start "" "backend\.venv\Scripts\pythonw.exe" "backend\main.py"
start "" "SC Ore Scanner.exe"
exit
