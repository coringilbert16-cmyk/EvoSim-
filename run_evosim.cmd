@echo off
setlocal
cd /d "%~dp0"

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0run_evosim.ps1"
set "EXIT_CODE=%ERRORLEVEL%"

if not "%EXIT_CODE%"=="0" (
    echo.
    echo EvoSim runner stopped with exit code %EXIT_CODE%.
    pause
)

endlocal
exit /b %EXIT_CODE%
