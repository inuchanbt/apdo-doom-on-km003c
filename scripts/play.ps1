# SPDX-License-Identifier: MIT
param([switch]$NoBrowser,[string]$Python='python',[int]$Port=0)
$ErrorActionPreference = 'Stop'
$taskArgs = @("$PSScriptRoot\pc-controller.py",'--port',"$Port")
if ($NoBrowser) { $taskArgs += '--no-browser' }
& $Python @taskArgs
if ($LASTEXITCODE -ne 0) { throw 'PC controller exited with an error' }
