param(
    [string]$Python = '',
    [ValidatePattern('^[A-Za-z0-9_-]+$')][string]$Name = ('km003c-doom-' + (Get-Date -Format yyyyMMdd-HHmmss))
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\env.ps1"
if (-not $Python) {
    . "$PSScriptRoot\local-config.ps1"
    $Python = if ($taskDeviceConfig.python) { $taskDeviceConfig.python } else { 'python' }
}
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskBinary = Join-Path $taskRoot "artifacts\$Name.bin"
$taskPackage = Join-Path $taskRoot "artifacts\$Name.mencrypt"
if ((Test-Path -LiteralPath $taskBinary) -or (Test-Path -LiteralPath $taskPackage)) { throw 'Output already exists; choose another Name' }
& $Python "$PSScriptRoot\build-engine.py"
if ($LASTEXITCODE -ne 0) { throw 'C engine build failed' }
Push-Location "$taskRoot\vendor\km003c-firmware\firmware"
try {
    cargo build --release --locked --features doom
    if ($LASTEXITCODE -ne 0) { throw 'Firmware build failed' }
    llvm-objcopy -O binary target\thumbv7em-none-eabihf\release\km003c-firmware $taskBinary
    if ($LASTEXITCODE -ne 0) { throw 'Binary conversion failed' }
} finally { Pop-Location }
& "$taskRoot\vendor\km003c-firmware\target\release\km003c-host-cli.exe" package $taskBinary $taskPackage
if ($LASTEXITCODE -ne 0) { throw 'Package validation failed' }
Write-Output "Created $taskPackage (device unchanged)"
