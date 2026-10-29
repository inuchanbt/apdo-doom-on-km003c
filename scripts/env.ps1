# SPDX-License-Identifier: MIT
$taskRoot = Split-Path -Parent $PSScriptRoot
if (Test-Path -LiteralPath "$taskRoot\tools\cargo") { $env:CARGO_HOME = "$taskRoot\tools\cargo" }
if (Test-Path -LiteralPath "$taskRoot\tools\rustup") { $env:RUSTUP_HOME = "$taskRoot\tools\rustup" }
$env:CARGO_NET_GIT_FETCH_WITH_CLI = 'true'
$taskLlvm = if ($env:APDO_LLVM_BIN) { $env:APDO_LLVM_BIN } else { Join-Path $taskRoot 'tools\llvm-mingw-20261020-ucrt-x86_64\bin' }
$env:APDO_LLVM_BIN = $taskLlvm
$taskRustupRoot = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE '.rustup' }
$taskCargoRoot = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$taskGcc = Join-Path $taskRustupRoot 'toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $taskGcc 'x86_64-w64-mingw32-gcc.exe'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-C dlltool=$taskLlvm\llvm-dlltool.exe"
$taskRustBin = Join-Path $taskRustupRoot 'toolchains\stable-x86_64-pc-windows-gnu\bin'
$env:PATH = "$taskRustBin;$taskCargoRoot\bin;$taskGcc;$taskLlvm;$env:PATH"
if (Test-Path -LiteralPath "$taskRoot\local\device.json") {
    $taskEnvConfig = Get-Content -Raw -LiteralPath "$taskRoot\local\device.json" | ConvertFrom-Json
    $env:APDO_DEVICE_SERIAL = [string]$taskEnvConfig.device_serial
} else {
    Remove-Item Env:APDO_DEVICE_SERIAL -ErrorAction SilentlyContinue
}
