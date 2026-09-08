@echo off
setlocal
cd /d "%~dp0"

start "EvoSim Server" cmd /c cargo run --release

for /L %%I in (1,1,30) do (
    curl.exe -fsS http://127.0.0.1:3000/ >nul 2>&1
    if not errorlevel 1 goto ready
    timeout /t 1 /nobreak >nul
)

echo EvoSim server did not become ready on http://127.0.0.1:3000/
pause
exit /b 1

:ready
start "" http://127.0.0.1:3000/
endlocal
