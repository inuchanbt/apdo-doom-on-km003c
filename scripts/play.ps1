# SPDX-License-Identifier: MIT
param([switch]$NoBrowser)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\env.ps1"
. "$PSScriptRoot\local-config.ps1"
$taskPython = if ($taskDeviceConfig.python) { $taskDeviceConfig.python } else { 'python' }
$taskArgs = @("$PSScriptRoot\pc-controller.py")
if ($NoBrowser) { $taskArgs += '--no-browser' }
& $taskPython @taskArgs
if ($LASTEXITCODE -ne 0) { throw 'PC controller exited with an error' }
