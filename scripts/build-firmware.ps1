# SPDX-License-Identifier: MIT
param(
    [string]$Python = 'python',
    [switch]$SafeNor,
    [ValidatePattern('^[A-Za-z0-9_-]+$')][string]$Name = ('apdo-independent-' + (Get-Date -Format yyyyMMdd-HHmmss))
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\env.ps1"
$taskRoot = Split-Path -Parent $PSScriptRoot
$taskBinary = Join-Path $taskRoot "artifacts\$Name.bin"
$taskPackage = Join-Path $taskRoot "artifacts\$Name.mencrypt"
$taskHost = Join-Path $taskRoot 'independent\target\release\apdo-host.exe'
if ((Test-Path -LiteralPath $taskBinary) -or (Test-Path -LiteralPath $taskPackage)) { throw 'Output exists; choose another Name' }
if (-not (Test-Path -LiteralPath $taskHost)) { throw 'Build the independent host first: scripts/build-host.ps1' }
& $Python "$PSScriptRoot\build-engine.py" --square
if ($LASTEXITCODE -ne 0) { throw 'Engine build failed' }
$taskOldSafeNor = $env:APDO_SAFE_NOR
$env:APDO_SAFE_NOR = if ($SafeNor) { '1' } else { '0' }
Push-Location "$taskRoot\independent\firmware"
try {
    cargo build --release --locked -p apdo-firmware
    if ($LASTEXITCODE -ne 0) { throw 'Independent firmware build failed' }
    llvm-objcopy -O binary ../target/thumbv7em-none-eabihf/release/apdo-firmware $taskBinary
    if ($LASTEXITCODE -ne 0) { throw 'Binary conversion failed' }
} finally { $env:APDO_SAFE_NOR = $taskOldSafeNor; Pop-Location }
& $taskHost package $taskBinary $taskPackage
if ($LASTEXITCODE -ne 0) { throw 'Package validation failed' }
Write-Output "Created $taskPackage (device unchanged)"
