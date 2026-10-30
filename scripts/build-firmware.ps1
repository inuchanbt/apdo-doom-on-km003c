param(
    [string]$Python = '',
    [switch]$Square,
    [switch]$SafeNor,
    [ValidateSet(0,8)][int]$EngineCacheKiB = 8,
    [ValidateSet('Oz','O2','O3')][string]$EngineOpt = 'O2',
    [ValidateSet('sram','sramh')][string]$EngineArena = 'sramh',
    [ValidatePattern('^[A-Za-z0-9_-]+$')][string]$Name = ('km003c-doom-' + (Get-Date -Format yyyyMMdd-HHmmss))
)
$ErrorActionPreference = 'Stop'
if ($EngineArena -eq 'sram' -and ($Square -or $EngineCacheKiB -ne 0)) {
    throw 'Ordinary SRAM comparisons require -EngineCacheKiB 0 and no -Square'
}
. "$PSScriptRoot\env.ps1"
if (-not $Python) {
    . "$PSScriptRoot\local-config.ps1"
    $Python = if ($taskDeviceConfig.python) { $taskDeviceConfig.python } else { 'python' }
}
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskBinary = Join-Path $taskRoot "artifacts\$Name.bin"
$taskPackage = Join-Path $taskRoot "artifacts\$Name.mencrypt"
if ((Test-Path -LiteralPath $taskBinary) -or (Test-Path -LiteralPath $taskPackage)) { throw 'Output already exists; choose another Name' }
$taskEngineArgs = @("$PSScriptRoot\build-engine.py", "--opt", $EngineOpt, "--arena", $EngineArena, "--cache-kib", $EngineCacheKiB)
if ($Square) {
    if ($EngineArena -ne "sramh") { throw "The square preview requires the SRAMH arena" }
    $taskEngineArgs += "--square"
}
& $Python @taskEngineArgs
if ($LASTEXITCODE -ne 0) { throw 'C engine build failed' }
Push-Location "$taskRoot\vendor\km003c-firmware\firmware"
$taskOldSafeNor = $env:APDO_SAFE_NOR
$env:APDO_SAFE_NOR = if ($SafeNor) { '1' } else { '0' }
try {
    cargo build --release --locked --features doom
    if ($LASTEXITCODE -ne 0) { throw 'Firmware build failed' }
    llvm-objcopy -O binary target\thumbv7em-none-eabihf\release\km003c-firmware $taskBinary
    if ($LASTEXITCODE -ne 0) { throw 'Binary conversion failed' }
} finally { $env:APDO_SAFE_NOR = $taskOldSafeNor; Pop-Location }
& "$taskRoot\vendor\km003c-firmware\target\release\km003c-host-cli.exe" package $taskBinary $taskPackage
if ($LASTEXITCODE -ne 0) { throw 'Package validation failed' }
Write-Output "Created $taskPackage (device unchanged)"
