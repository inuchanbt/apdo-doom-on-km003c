# SPDX-License-Identifier: MIT
[CmdletBinding()]
param(
    [ValidateSet('Menu', 'Install', 'Backup', 'Restore', 'RecoverApp', 'RemoveDoomData', 'CheckPackage')]
    [string] $Action = 'Menu',
    [string] $BackupPath,
    [ValidateSet('2.3', '2.1')]
    [string] $TargetHardware = '2.3'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($TargetHardware -ne '2.3') { throw 'This HID release targets HW 2.3. Use the historical v0.1.1 package for the HW 2.1 trial.' }

$hostExe = Join-Path $PSScriptRoot 'apdo-host.exe'
$backupExe = Join-Path $PSScriptRoot 'km003c-backup.exe'
$firmware = Join-Path $PSScriptRoot 'apdo-doom-km003c-v0.1.2-independent.mencrypt'
$game = Join-Path $PSScriptRoot 'DOOM.DAT'
$trial = $TargetHardware -eq '2.1'
$trialSteps = [Collections.Generic.List[object]]::new()
$trialResult = 'stopped'
$physicalCheck = 'unrecorded'

function Assert-Digest($Path, $Expected) {
    if ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ne $Expected) {
        throw "SHA-256 mismatch for $Path"
    }
}

function Invoke-Tool($Exe, [string[]] $Arguments) {
    $step = [ordered]@{ command = $Arguments[0]; result = 'started' }
    if ($trial) {
        if ($Arguments[0] -in 'flash', 'start', 'check-app') {
            $step['image'] = if ($Arguments[2] -eq $restoreImage) { 'stock' } else { 'doom' }
        }
        $trialSteps.Add($step)
    }
    Write-Host "`n> $(Split-Path -Leaf $Exe) $($Arguments[0])"
    & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) {
        $step['result'] = 'failed'
        $step['exit_code'] = $LASTEXITCODE
        throw "$($Arguments[0]) stopped with exit code $LASTEXITCODE. See the error above."
    }
    $step['result'] = 'passed'
}

function Confirm-Operation($Word, $Prompt) {
    if ((Read-Host "$Prompt Type $Word to continue") -cne $Word) {
        throw 'Operation cancelled.'
    }
}

function Enter-Dfu {
    Write-Host 'Unplug side HID. Hold Exit while reconnecting to the same PC USB port.'
    Read-Host 'Release Exit at the native update screen and press Enter' | Out-Null
}

function Write-Checkpoint($Path) {
    $file = [IO.File]::Open($Path, 'CreateNew', 'Write', 'None')
    try {
        $file.WriteByte(1)
        $file.Flush($true)
    } finally {
        $file.Dispose()
    }
}

foreach ($exe in @($hostExe, $backupExe)) {
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Missing $exe" }
    $builtFor = & $exe target-hardware
    if ($LASTEXITCODE -ne 0 -or $builtFor -ne $TargetHardware) {
        throw "Installer tool $(Split-Path -Leaf $exe) does not match the HW $TargetHardware package. Use the included launcher."
    }
}
Assert-Digest $firmware '666129410162fff3b1ef69888393e17edcd82ed6376b59b32b4d2d9b13f346a9'
Assert-Digest $game '6b25a4f1adb6f1f1f5f42226c272b93ccff4f356ac8d7034cf83712b2da70c08'
if ($Action -eq 'CheckPackage') {
    Write-Host 'Installer payloads passed SHA-256 verification.'
    return
}

Write-Host "APDO DOOM for KM003C HW $TargetHardware / stock FW 2.0.7"
if ($trial) { Write-Host 'HW 2.1 experimental hardware trial. Preserve the backup and check the screen at each prompt.' }
if ($Action -eq 'Menu') {
    Write-Host '1. Back up and install DOOM'
    Write-Host '2. Back up stock firmware only (read-only)'
    Write-Host '3. Restore original NOR and stock firmware'
    Write-Host '4. Recover stock app after an app-only install failure'
    Write-Host '5. Remove leftover DOOM data after stock firmware was already restored'
    $Action = switch (Read-Host 'Select an operation') {
        '1' { 'Install' }
        '2' { 'Backup' }
        '3' { 'Restore' }
        '4' { 'RecoverApp' }
        '5' { 'RemoveDoomData' }
        default { throw 'Choose 1, 2, 3, 4 or 5.' }
    }
    $BackupPath = Read-Host 'Backup folder path (blank creates a new folder for 1 or 2)'
}

if ($Action -eq 'RemoveDoomData' -and [string]::IsNullOrWhiteSpace($BackupPath)) {
    $BackupPath = Read-Host 'Original installation backup folder path (contains stock and nor)'
}
if ([string]::IsNullOrWhiteSpace($BackupPath)) {
    if ($Action -in 'Restore', 'RecoverApp', 'RemoveDoomData') { throw 'Select the original installation backup folder.' }
    $folder = '{0}-{1}' -f (Get-Date -Format 'yyyyMMdd-HHmmss'), ([guid]::NewGuid().ToString('N').Substring(0, 8))
    $BackupPath = Join-Path (Join-Path $env:LOCALAPPDATA 'APDO DOOM\Backups') $folder
}
$session = [IO.Path]::GetFullPath($BackupPath.Trim().Trim('"'))
$stock = Join-Path $session 'stock'
$nor = Join-Path $session 'nor'
$original = Join-Path $stock 'stock-app.bin'
$restoreImage = Join-Path $stock 'restore.mencrypt'
$appAttempted = Join-Path $session 'app-write-attempted'
$gameAttempted = Join-Path $session 'game-write-attempted'

Write-Host "Backup and verification records: $session"
Write-Host 'Close MTOOLS and other KM003C software. Connect one meter via side HID only.'
Write-Host 'Leave both measurement ports empty and keep the same meter and PC USB port.'

$mutex = [Threading.Mutex]::new($false, 'Local\APDO-Doom-KM003C-Installer')
$locked = $false
try {
    try { $locked = $mutex.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $locked = $true }
    if (-not $locked) { throw 'Another APDO installer is running.' }

    if ($Action -in 'Install', 'Backup') {
        if ((Test-Path -LiteralPath $appAttempted) -or (Test-Path -LiteralPath $gameAttempted) -or (Test-Path -LiteralPath $nor)) {
            throw 'This folder already has an installation attempt. Preserve it for restoration.'
        }
        if (Test-Path -LiteralPath $stock) {
            if ($Action -eq 'Backup') { throw 'Choose a new backup folder.' }
            Enter-Dfu
        } else {
            [IO.Directory]::CreateDirectory($session) | Out-Null
            Read-Host 'Connect the meter running stock FW and press Enter' | Out-Null
            Invoke-Tool $backupExe @('stock', '--output', $stock)
            Enter-Dfu
            Invoke-Tool $backupExe @('dfu-check', '--stock', $stock)
            Invoke-Tool $hostExe @('package-installed', $original, $restoreImage)
        }
    }
    Invoke-Tool $backupExe @('verify', '--stock', $stock, '--require-dfu')
    Invoke-Tool $hostExe @('audit-image', $restoreImage, $original)

    if ($Action -eq 'Backup') {
        Write-Host "Stock firmware and the first 3 MiB of NOR were saved to $session."
        Write-Host 'Reconnect side HID normally to return to stock. Full 4 MiB NOR backup runs during installation.'
        $trialResult = 'passed'
        return
    }

    $run = Join-Path $session ('run-' + [guid]::NewGuid().ToString('N'))
    [IO.Directory]::CreateDirectory($run) | Out-Null
    if ($Action -eq 'Install') {
        Invoke-Tool $hostExe @('check-app', $stock, $restoreImage)
        Write-Host 'Firmware installation can brick the meter. Preserve this backup folder.'
        Confirm-Operation 'INSTALL' 'Install APDO DOOM and its game data.'
        Write-Checkpoint $appAttempted
        Invoke-Tool $hostExe @('flash', $stock, $firmware, (Join-Path $run 'app-flash'))
        Invoke-Tool $hostExe @('start', $stock, $firmware)
        if ($trial) {
            Read-Host 'With the Freedoom logo visible, press Enter to begin the Exit test' | Out-Null
            Invoke-Tool $hostExe @('exit-test', $stock, $firmware)
            Invoke-Tool $hostExe @('start', $stock, $firmware)
        }
        Invoke-Tool $hostExe @('nor-backup', $stock, $nor)
        if ($trial) { Confirm-Operation 'GAME' 'Full original NOR backup verified. Write Freedoom E1M1.' }
        Write-Checkpoint $gameAttempted
        Invoke-Tool $hostExe @('install-game', $stock, $nor, $game, (Join-Path $run 'game-install'))
        Invoke-Tool $hostExe @('play', $stock)
        if ($trial) {
            Invoke-Tool $hostExe @('check-game', $stock)
            $physicalCheck = if ((Read-Host 'Game visible and buttons working? Type YES if verified') -ceq 'YES') { 'confirmed' } else { 'unconfirmed' }
        }
        $trialResult = 'passed'
        Write-Host "DOOM started. Keep this folder for restoration: $session"
        return
    }

    if ($Action -eq 'RecoverApp') {
        if (-not (Test-Path -LiteralPath $appAttempted -PathType Leaf) -or (Test-Path -LiteralPath $gameAttempted)) {
            throw 'App-only recovery requires the original installer folder from an attempt that stopped before game writing.'
        }
        Confirm-Operation 'APP-ONLY' 'Use the original session folder and confirm that game data was never written or attempted.'
        Enter-Dfu
    } else {
        if (-not (Test-Path -LiteralPath (Join-Path $nor 'verified.json') -PathType Leaf)) {
            throw 'Full restore requires the verified 4 MiB NOR backup from this installation.'
        }
        Invoke-Tool $hostExe @('verify-nor', $stock, $nor, $game)
        if ($Action -eq 'RemoveDoomData') {
            Confirm-Operation 'REMOVE-DATA' 'Temporarily write the DOOM service, restore the original NOR, then restore stock firmware. This rewrites firmware and can brick the meter.'
        } else {
            Confirm-Operation 'RESTORE' 'Restore this meter from the selected backup.'
        }
        Enter-Dfu
        if ($Action -eq 'RemoveDoomData') {
            Invoke-Tool $hostExe @('check-app', $stock, $restoreImage)
            Invoke-Tool $hostExe @('flash', $stock, $firmware, (Join-Path $run 'cleanup-service-flash'))
        }
        Read-Host 'Hold OK on the meter, press Enter, and keep holding OK until the Freedoom logo appears' | Out-Null
        Invoke-Tool $hostExe @('start', $stock, $firmware)
        Invoke-Tool $hostExe @('restore-nor', $stock, $nor, $game, (Join-Path $run 'nor-restore'))
        Invoke-Tool $hostExe @('loader', $stock)
    }

    Invoke-Tool $hostExe @('flash', $stock, $restoreImage, (Join-Path $run 'stock-flash'))
    Invoke-Tool $hostExe @('start', $stock, $restoreImage)
    if ($Action -in 'Restore', 'RemoveDoomData') {
        Write-Host 'Original NOR restored and all 4 MiB verified. DOOM game data was replaced with the original bytes.'
    }
    Write-Host 'Stock firmware readback and boot identity verified. Check the original screen and measurement functions.'
    if ($trial) {
        $physicalCheck = if ((Read-Host 'Original screen and tester functions working? Type YES if verified') -ceq 'YES') { 'confirmed' } else { 'unconfirmed' }
    }
    $trialResult = 'passed'
} catch {
    Write-Warning "Stopped at the first failure. Keep the backup and records in $session."
    throw
} finally {
    try {
        if ($locked -and $trial -and (Test-Path -LiteralPath $session -PathType Container)) {
            $reportName = 'hw21-doom-{0}-{1}.json' -f $Action.ToLowerInvariant(), [guid]::NewGuid().ToString('N').Substring(0, 8)
            $reportPath = Join-Path $session $reportName
            [ordered]@{
                schema = 'apdo-doom-hw21-trial/1'
                operation = $Action
                result = $trialResult
                physical_check = $physicalCheck
                steps = $trialSteps.ToArray()
            } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $reportPath -Encoding UTF8
            Write-Host "Shareable trial report: $reportPath"
        }
    } finally {
        if ($locked) { $mutex.ReleaseMutex() }
        $mutex.Dispose()
    }
}
