# SPDX-License-Identifier: MIT
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\env.ps1"
# Independent crate: no vendor checkout, local/device.json or loader binary needed.
cargo build --manifest-path "$PSScriptRoot\..\bootstrap\Cargo.toml" --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Read-only backup tool build failed' }
