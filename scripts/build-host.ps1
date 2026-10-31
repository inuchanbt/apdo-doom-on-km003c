# SPDX-License-Identifier: MIT
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\env.ps1"
cargo build --manifest-path "$PSScriptRoot\..\independent\Cargo.toml" --release --locked -p apdo-host
if ($LASTEXITCODE -ne 0) { throw 'Independent host build failed' }
