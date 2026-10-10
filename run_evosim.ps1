# EvoSim persistent local runner.
# Owns the running binary, watches origin/main, and only swaps to a
# newly built version after the build succeeds.

$ErrorActionPreference = "Stop"

# PowerShell 5.1 can surface native stderr as a terminating error even when
# stderr is redirected. Git legitimately writes progress messages to stderr,
# so the Git helpers temporarily allow that native stream while checking the
# actual process exit code themselves.
$Repo = $PSScriptRoot
$RunnerRoot = Join-Path $Repo ".evosim-runner"
$CurrentDir = Join-Path $RunnerRoot "current"
$StagingRoot = Join-Path $RunnerRoot "staging"
$CurrentExe = Join-Path $CurrentDir "evosim.exe"
$CurrentVersion = Join-Path $CurrentDir "version.txt"
$StagingWorktree = Join-Path $StagingRoot "source"
$StagingTarget = Join-Path $StagingRoot "target"
$PollSeconds = 10

New-Item -ItemType Directory -Force -Path $CurrentDir, $StagingRoot | Out-Null

$mutex = New-Object System.Threading.Mutex($false, "Local\EvoSimAutoRunner")
if (-not $mutex.WaitOne(0)) {
    Write-Host "EvoSim is already running through the automatic runner."
    exit 1
}

$script:serverProcess = $null
$script:CargoWorkingDirectory = $Repo
$script:knownGoodVersion = ""
$script:rollbackExe = $null
$script:rejectedVersion = ""
$script:restartAttempted = $false

function Invoke-Git([string[]]$Arguments) {
    $stderrPath = [System.IO.Path]::GetTempFileName()
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = & git @Arguments 2> $stderrPath
        $exitCode = $LASTEXITCODE
        if ($exitCode -ne 0) {
            $stdout = (($output | Out-String).Trim())
            $stderr = (Get-Content -Raw $stderrPath).Trim()
            $detail = (($stdout, $stderr | Where-Object { $_ }) -join [Environment]::NewLine).Trim()
            if ($detail) {
                throw "git $($Arguments -join ' ') failed with exit code ${exitCode}: $detail"
            }
            throw "git $($Arguments -join ' ') failed with exit code ${exitCode}"
        }
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
        Remove-Item -Force $stderrPath -ErrorAction SilentlyContinue
    }
}

function Get-GitOutput([string[]]$Arguments) {
    $stderrPath = [System.IO.Path]::GetTempFileName()
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "Continue"
        $output = & git @Arguments 2> $stderrPath
        $exitCode = $LASTEXITCODE
        if ($exitCode -ne 0) {
            $stdout = (($output | Out-String).Trim())
            $stderr = (Get-Content -Raw $stderrPath).Trim()
            $detail = (($stdout, $stderr | Where-Object { $_ }) -join [Environment]::NewLine).Trim()
            if ($detail) {
                throw "git $($Arguments -join ' ') failed with exit code ${exitCode}: $detail"
            }
            throw "git $($Arguments -join ' ') failed with exit code ${exitCode}"
        }
        return (($output | Out-String).Trim())
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
        Remove-Item -Force $stderrPath -ErrorAction SilentlyContinue
    }
}

function Test-WorktreeClean {
    $status = Get-GitOutput @("-C", $Repo, "status", "--porcelain", "--untracked-files=all")
    $unexpected = @($status -split "`r?`n" | Where-Object {
        $_ -and $_ -ne "?? Cargo.lock"
    })
    return $unexpected.Count -eq 0
}

function Get-RemoteMain {
    return Get-GitOutput @("-C", $Repo, "rev-parse", "origin/main")
}

function Get-CurrentVersion {
    if (Test-Path $CurrentVersion) {
        return (Get-Content -Raw $CurrentVersion).Trim()
    }
    return ""
}

function Remove-StagingWorktree {
    if (Test-Path (Join-Path $StagingWorktree ".git")) {
        try {
            Invoke-Git @("-C", $Repo, "worktree", "remove", "--force", $StagingWorktree)
        } catch {}
    } elseif (Test-Path $StagingWorktree) {
        Remove-Item -Recurse -Force $StagingWorktree
    }
    try {
        Invoke-Git @("-C", $Repo, "worktree", "prune")
    } catch {}
    if (Test-Path $StagingTarget) {
        Remove-Item -Recurse -Force $StagingTarget
    }
}

function Invoke-Cargo([string[]]$Arguments) {
    $stdoutPath = [System.IO.Path]::GetTempFileName()
    $stderrPath = [System.IO.Path]::GetTempFileName()
    try {
        $process = Start-Process -FilePath "cargo.exe" -ArgumentList $Arguments -WorkingDirectory $script:CargoWorkingDirectory -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru -Wait
        $process.Refresh()
        $exitCode = $process.ExitCode

        $stdout = Get-Content -Raw $stdoutPath -ErrorAction SilentlyContinue
        $stderr = Get-Content -Raw $stderrPath -ErrorAction SilentlyContinue
        if ($stdout) { Write-Host $stdout.TrimEnd() }
        if ($stderr) { Write-Host $stderr.TrimEnd() }

        if ($exitCode -ne 0) {
            $detail = (($stdout, $stderr | Where-Object { $_ }) -join [Environment]::NewLine).Trim()
            if ($detail) {
                throw "cargo $($Arguments -join " ") failed with exit code ${exitCode}: $detail"
            }
            throw "cargo $($Arguments -join " ") failed with exit code $exitCode"
        }
    }
    finally {
        Remove-Item -Force $stdoutPath, $stderrPath -ErrorAction SilentlyContinue
    }
}

function Build-Version([string]$Sha) {
    Write-Host "Building EvoSim $Sha ..."
    Remove-StagingWorktree

    Invoke-Git @("-C", $Repo, "worktree", "add", "--detach", $StagingWorktree, $Sha)
    try {
        $script:CargoWorkingDirectory = $StagingWorktree
        # Invoke Cargo without PowerShell's native-stderr handling. PowerShell 5.1
        # can turn ordinary Cargo progress on stderr into NativeCommandError records.
        Invoke-Cargo @("build", "--release", "--target-dir", $StagingTarget)

        Write-Host "Running Rust tests for $Sha ..."
        Invoke-Cargo @("test", "--all-targets", "--release", "--target-dir", $StagingTarget)

        $builtExe = Join-Path $StagingTarget "release\evosim.exe"
        if (-not (Test-Path $builtExe)) {
            throw "cargo build succeeded but $builtExe was not produced"
        }

        $nextExe = Join-Path $StagingRoot "evosim-next.exe"
        if (Test-Path $nextExe) {
            Remove-Item -Force $nextExe
        }
        Copy-Item $builtExe $nextExe
        return $nextExe
    }
    finally {
        $script:CargoWorkingDirectory = $Repo
        Remove-StagingWorktree
    }
}

function Start-Server {
    if (-not (Test-Path $CurrentExe)) {
        throw "No known-good EvoSim binary exists."
    }

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $CurrentExe
    $startInfo.WorkingDirectory = $Repo
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $script:serverProcess = New-Object System.Diagnostics.Process
    $script:serverProcess.StartInfo = $startInfo
    [void]$script:serverProcess.Start()

    $version = Get-CurrentVersion
    if ([string]::IsNullOrWhiteSpace($version)) {
        $version = "unknown"
    }
    $shortVersion = $version.Substring(0, [Math]::Min(12, $version.Length))
    Write-Host "Running EvoSim $shortVersion (PID $($script:serverProcess.Id))."
}

function Wait-ForServer {
    for ($i = 0; $i -lt 30; $i++) {
        if ($null -ne $script:serverProcess) {
            $script:serverProcess.Refresh()
            if ($script:serverProcess.HasExited) {
                Write-Host "EvoSim process $($script:serverProcess.Id) exited before becoming ready (exit code $($script:serverProcess.ExitCode))."
                return $false
            }
        }

        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:3000/observation/status" -TimeoutSec 1
            if ($response.StatusCode -eq 200) {
                $status = $response.Content | ConvertFrom-Json
                if ($null -ne $status.tick -and $null -ne $status.session_id) {
                    $script:serverProcess.Refresh()
                    if (-not $script:serverProcess.HasExited) {
                        return $true
                    }
                    Write-Host "EvoSim answered health check but exited immediately afterward (exit code $($script:serverProcess.ExitCode))."
                    return $false
                }
            }
        } catch {}
        Start-Sleep -Seconds 1
    }
    return $false
}

function Stop-ProcessTree([int]$ProcessId) {
    $taskkill = Start-Process -FilePath "taskkill.exe" -ArgumentList @("/PID", $ProcessId.ToString(), "/T", "/F") -PassThru -Wait
    $taskkill.Refresh()
    if ($taskkill.ExitCode -ne 0) {
        throw "taskkill failed for EvoSim process $ProcessId with exit code $($taskkill.ExitCode)"
    }
}

function Stop-OrphanedEvoSimProcesses {
    if (-not (Test-Path $CurrentExe)) {
        return
    }

    $processes = @(Get-CimInstance Win32_Process -Filter "Name = 'evosim.exe'" | Where-Object {
        $_.ExecutablePath -and [string]::Equals($_.ExecutablePath, $CurrentExe, [System.StringComparison]::OrdinalIgnoreCase)
    })

    foreach ($process in $processes) {
        Write-Host "Stopping orphaned EvoSim process $($process.ProcessId) holding the runner binary (PID $($process.ProcessId))."
        try {
            Stop-ProcessTree ([int]$process.ProcessId)
        } catch {
            Write-Host "Could not stop orphaned EvoSim process $($process.ProcessId): $($_.Exception.Message)"
        }
    }
}

function Stop-Server {
    if ($null -ne $script:serverProcess) {
        $script:serverProcess.Refresh()
        if (-not $script:serverProcess.HasExited) {
            Write-Host "Stopping EvoSim process $($script:serverProcess.Id) ..."
            Stop-ProcessTree $script:serverProcess.Id
            $script:serverProcess.WaitForExit()
        }
        $script:serverProcess.Refresh()
        if ($script:serverProcess.HasExited) {
            Write-Host "EvoSim process $($script:serverProcess.Id) stopped (exit code $($script:serverProcess.ExitCode))."
        }
    }
    $script:serverProcess = $null
}

function Install-Version([string]$Sha, [string]$NextExe, [string]$PreviousSha) {
    if (-not (Test-Path $NextExe)) {
        throw "Candidate binary does not exist: $NextExe"
    }

    $shortPreviousSha = if ([string]::IsNullOrWhiteSpace($PreviousSha)) { "initial" } else { $PreviousSha.Substring(0, [Math]::Min(12, $PreviousSha.Length)) }
    $backupExe = Join-Path $RunnerRoot ("previous-$shortPreviousSha.exe")
    if (Test-Path $backupExe) {
        Remove-Item -Force $backupExe
    }

    if (Test-Path $CurrentExe) {
        Move-Item -Force $CurrentExe $backupExe
    }

    try {
        Move-Item -Force $NextExe $CurrentExe
        Set-Content -Path $CurrentVersion -Value $Sha -NoNewline
        $script:rollbackExe = $backupExe
    }
    catch {
        if (Test-Path $CurrentExe) {
            Remove-Item -Force $CurrentExe
        }
        if ((-not (Test-Path $CurrentExe)) -and (Test-Path $backupExe)) {
            Move-Item -Force $backupExe $CurrentExe
        }
        throw
    }
}

function Confirm-InstalledVersion {
    if ($null -eq $script:rollbackExe) {
        return
    }

    if (Test-Path $script:rollbackExe) {
        try {
            Remove-Item -Force $script:rollbackExe
        } catch {
            Write-Host "Previous binary could not be deleted yet; leaving it in place: $script:rollbackExe"
        }
    }
    $script:rollbackExe = $null
}

try {
    Set-Location $Repo

    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        throw "Git is required for EvoSim automatic updates."
    }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Cargo/Rust is required to build EvoSim updates."
    }

    Write-Host "EvoSim automatic runner starting."
    Write-Host "Repository: $Repo"
    Set-Location $Repo

    $clean = Test-WorktreeClean
    if (-not $clean) {
        Write-Host "WARNING: local source changes are present. Automatic source updates are paused until the worktree is clean."
    }

    try {
        Invoke-Git @("-C", $Repo, "fetch", "origin", "main", "--prune")
    } catch {
        Write-Host "Initial GitHub fetch failed; the last known-good version will be used if available."
    }

    $remoteSha = $null
    try { $remoteSha = Get-RemoteMain } catch {}

    if (-not (Test-Path $CurrentExe)) {
        if (-not $remoteSha) {
            $remoteSha = Get-GitOutput @("-C", $Repo, "rev-parse", "HEAD")
        }
        $nextExe = Build-Version $remoteSha
        Install-Version $remoteSha $nextExe (Get-CurrentVersion)
    }

    $script:knownGoodVersion = Get-CurrentVersion
    Stop-OrphanedEvoSimProcesses
    Start-Server
    if (-not (Wait-ForServer)) {
        Stop-Server
        throw "EvoSim server did not become ready on http://127.0.0.1:3000/"
    }

    Start-Process "http://127.0.0.1:3000/"
    Write-Host "Viewer is open. Leave this runner window open; GitHub updates will be checked every $PollSeconds seconds."
    Write-Host "The browser will reconnect automatically when a new validated build replaces the running version."

    while ($true) {
        Start-Sleep -Seconds $PollSeconds

        if ($null -ne $script:serverProcess) {
            $script:serverProcess.Refresh()
            if ($script:serverProcess.HasExited) {
                $exitCode = $script:serverProcess.ExitCode
                Write-Host "EvoSim server stopped unexpectedly (exit code $exitCode)."
                $script:serverProcess = $null
                Stop-OrphanedEvoSimProcesses

                if ($script:restartAttempted) {
                    Write-Host "EvoSim has already required an automatic restart during this runner session; automatic restart is now paused until the runner is restarted."
                    Start-Sleep -Seconds $PollSeconds
                    continue
                }

                $script:restartAttempted = $true
                try {
                    Start-Server
                    if (-not (Wait-ForServer)) {
                        Stop-Server
                        Write-Host "Known-good server restart failed; automatic restart is paused until the runner is restarted."
                    } else {
                        Write-Host "Known-good server restart succeeded."
                    }
                } catch {
                    Write-Host "Known-good server restart failed: $($_.Exception.Message)"
                }
            }
        }

        try {
            Invoke-Git @("-C", $Repo, "fetch", "origin", "main", "--prune")
        } catch {
            Write-Host "GitHub check failed; keeping the current version."
            continue
        }

        if (-not (Test-WorktreeClean)) {
            Write-Host "Local source changes detected; update skipped."
            continue
        }

        try {
            $remoteSha = Get-RemoteMain
            $runningSha = Get-CurrentVersion
        } catch {
            Write-Host "Could not determine source version; keeping the current version."
            continue
        }

        if ([string]::IsNullOrWhiteSpace($remoteSha) -or $remoteSha -eq $runningSha) {
            continue
        }

        Write-Host "New main commit detected: $remoteSha"
        try {
            $nextExe = Build-Version $remoteSha

            # Re-check before interrupting the running simulation. If main moved
            # during the build, keep the current process and build the newer SHA
            # on the next poll instead.
            Invoke-Git @("-C", $Repo, "fetch", "origin", "main", "--prune")
            $latestSha = Get-RemoteMain
            if ($latestSha -ne $remoteSha) {
                Write-Host "main changed during build; keeping the current version and retrying with $latestSha."
                Remove-Item -Force $nextExe
                continue
            }

            Stop-Server
            Install-Version $remoteSha $nextExe $runningSha

            try {
                Start-Server
                if (-not (Wait-ForServer)) {
                    throw "new version did not become ready"
                }

                # The candidate is not known-good until its own process has
                # passed the health check and is still alive.
                $script:knownGoodVersion = $remoteSha
                $script:rejectedVersion = ""
                $script:restartAttempted = $false
                Confirm-InstalledVersion
                Write-Host "Updated EvoSim to $remoteSha. Browser connections can reconnect now; press Reset in the viewer to begin the new run."
            } catch {
                Write-Host "New version failed to start or remain healthy: $($_.Exception.Message)"
                Write-Host "Restoring the previous known-good binary."
                Stop-Server

                if ($null -eq $script:rollbackExe -or -not (Test-Path $script:rollbackExe)) {
                    throw "rollback binary is unavailable"
                }

                if (Test-Path $CurrentExe) {
                    Remove-Item -Force $CurrentExe
                }
                Move-Item -Force $script:rollbackExe $CurrentExe
                $script:rollbackExe = $null
                Set-Content -Path $CurrentVersion -Value $runningSha -NoNewline
                $script:knownGoodVersion = $runningSha
                $script:rejectedVersion = $remoteSha
                $script:restartAttempted = $false

                Start-Server
                if (-not (Wait-ForServer)) {
                    Stop-Server
                    throw "rollback version also failed to become ready"
                }
                Write-Host "Rollback succeeded. Candidate $remoteSha is rejected for this runner session and will not be retried."
                continue
            }
        } catch {
            Write-Host "Update failed: $($_.Exception.Message)"
            Write-Host "The current running version will remain in service."
            if ($null -ne $script:rollbackExe -and (Test-Path $script:rollbackExe)) {
                Write-Host "A rollback binary remains at $script:rollbackExe."
            }
            try {
                Remove-StagingWorktree
            } catch {}
        }
    }
}
finally {
    if ($null -ne $mutex) {
        try { $mutex.ReleaseMutex() } catch {}
        $mutex.Dispose()
    }
}
