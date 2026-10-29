$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\env.ps1"
Push-Location "$PSScriptRoot\..\vendor\km003c-firmware"
try {
    cargo build --release --locked -p km003c-host-cli
    if ($LASTEXITCODE -ne 0) { throw 'Host build failed' }
} finally { Pop-Location }
