param(
    [ValidateRange(1, 3600)]
    [int]$DurationSeconds = 600,
    [int[]]$Seeds = @(42, 7, 101),
    [ValidateRange(1, 100000)]
    [int]$DiagnosticInterval = 100
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$outputDirectory = Join-Path $repoRoot ".evosim-experiments\environment-chemistry-baseline-$runId"

New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
Push-Location $repoRoot
try {
    foreach ($seed in $Seeds) {
        $diagnosticsPath = Join-Path $outputDirectory "seed-$seed.jsonl"
        $consolePath = Join-Path $outputDirectory "seed-$seed.console.log"
        Write-Output "Starting standard environment chemistry run: seed=$seed duration=${DurationSeconds}s"
        Write-Output "Diagnostics: $diagnosticsPath"

        & cargo run --locked -- `
            --headless `
            --environment-chemistry-experiment `
            --seconds $DurationSeconds `
            --seed $seed `
            --diagnostics $diagnosticsPath `
            --diagnostic-interval $DiagnosticInterval 2>&1 |
            Tee-Object -FilePath $consolePath

        if ($LASTEXITCODE -ne 0) {
            throw "Chemistry baseline failed for seed $seed (exit code $LASTEXITCODE). See $consolePath"
        }
    }
}
finally {
    Pop-Location
}

Write-Output "Completed independent baseline runs. Artifacts: $outputDirectory"
