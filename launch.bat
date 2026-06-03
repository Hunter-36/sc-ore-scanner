@echo off
echo ============================================================
echo SC ORE SCANNER - Launcher
echo ============================================================
echo.

REM Check if Python backend exists
if not exist "backend\main.py" (
    echo ERROR: Backend not found!
    echo Please ensure you're running this from the sc-ore-scanner directory.
    pause
    exit /b 1
)

REM Check if frontend exists
if not exist "frontend\package.json" (
    echo ERROR: Frontend not found!
    echo Please ensure you're running this from the sc-ore-scanner directory.
    pause
    exit /b 1
)

REM Start backend in new window
echo [1/2] Starting Python backend...
start "SC Ore Scanner - Backend" cmd /k "cd backend && .venv\Scripts\python.exe main.py"

REM Wait a moment for backend to initialize
timeout /t 3 /nobreak > nul

REM Start frontend in new window
echo [2/2] Starting Tauri frontend...
start "SC Ore Scanner - Frontend" cmd /k "cd frontend && npm run tauri dev"

echo.
echo ============================================================
echo LAUNCHED!
echo ============================================================
echo.
echo Two windows will open:
echo   1. Backend (Python FastAPI server)
echo   2. Frontend (Tauri overlay app)
echo.
echo Close this window or press any key to exit launcher.
echo (Note: Backend and frontend will continue running)
echo.
pause > nul
