@echo off
setlocal
cd /d "%~dp0"

powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "$shell = New-Object -ComObject WScript.Shell; $shortcut = $shell.CreateShortcut([Environment]::GetFolderPath('Desktop') + '\EvoSim.lnk'); $shortcut.TargetPath = Join-Path $PWD 'run_evosim.cmd'; $shortcut.WorkingDirectory = $PWD; $shortcut.Description = 'Launch EvoSim'; $shortcut.Save()"

if errorlevel 1 (
    echo Failed to create the EvoSim desktop shortcut.
    pause
    exit /b 1
)

echo EvoSim desktop shortcut created successfully.
pause
endlocal
