# SPDX-License-Identifier: MIT
$taskProjectRoot = Split-Path -Parent $PSScriptRoot
$taskConfigPath = Join-Path $taskProjectRoot 'local\device.json'
if (-not (Test-Path -LiteralPath $taskConfigPath)) {
    throw 'Create local/device.json from local.example.json and configure your verified recovery material.'
}
$taskDeviceConfig = Get-Content -Raw -LiteralPath $taskConfigPath | ConvertFrom-Json
if (-not $taskDeviceConfig.device_serial -or $taskDeviceConfig.device_serial -eq 'YOUR_DEVICE_SERIAL') {
    throw 'Configure the verified device serial in local/device.json.'
}
