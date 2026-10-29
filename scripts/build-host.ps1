$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\env.ps1"
if (-not (Test-Path -LiteralPath "$PSScriptRoot\..\vendor\km003c-firmware\fw\bootloader_local_hw2.3.bin")) {
    throw 'Missing private loader baseline. Start with scripts/build-backup.ps1 and docs/FIRST-BACKUP.md, then run scripts/prepare-sources.py.'
}
Push-Location "$PSScriptRoot\..\vendor\km003c-firmware"
try {
    cargo build --release --locked -p km003c-host-cli
    if ($LASTEXITCODE -ne 0) { throw 'Host build failed' }
} finally { Pop-Location }
