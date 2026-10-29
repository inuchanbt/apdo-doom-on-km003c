$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\env.ps1"
. "$PSScriptRoot\local-config.ps1"
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $taskCli = '.\vendor\km003c-firmware\target\release\km003c-host-cli.exe'
    foreach ($taskRequired in @($taskCli, (Join-Path $taskDeviceConfig.stock_backup 'stock-restore.mencrypt'), (Join-Path $taskDeviceConfig.external_backup 'external-4MiB.bin'), (Join-Path $taskDeviceConfig.external_backup 'verified.json'))) {
        if (-not (Test-Path -LiteralPath $taskRequired)) { throw "Missing local recovery file: $taskRequired" }
    }
    & $taskCli recovery-loader
    if ($LASTEXITCODE -ne 0) { throw 'Could not enter native DFU; hold Exit during power-on and run the recovery steps in README.ja.md' }
    & $taskCli start
    if ($LASTEXITCODE -ne 0) { throw 'Game service did not restart' }
    $taskStamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    & $taskCli game-restore $taskDeviceConfig.external_backup ("artifacts\restore-nor-" + $taskStamp)
    if ($LASTEXITCODE -ne 0) { throw 'NOR restore failed; stock firmware has not been installed' }
    & $taskCli flash (Join-Path $taskDeviceConfig.stock_backup 'stock-restore.mencrypt') --evidence ("artifacts\restore-stock-" + $taskStamp)
    if ($LASTEXITCODE -ne 0) { throw 'Stock application restore failed; device remains in native DFU' }
    & $taskCli stock-start
    if ($LASTEXITCODE -ne 0) { throw 'Stock startup verification failed' }
} finally { Pop-Location }
