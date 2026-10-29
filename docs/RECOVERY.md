# Recovery and per-device prerequisites

The original HW2.3 meter's full stock restoration has been performed successfully. This proves the recorded procedure for that unit; it does not guarantee recovery for every device or failure mode.

For another untouched HW2.3 / stock 2.0.7 unit, start with [FIRST-BACKUP.md](FIRST-BACKUP.md). The independent tool builds without existing backups and verifies stock/DFU reads before importing that unit's loader. Do not use the original unit's dumps. The new bootstrap sequence is covered by offline tests and still needs an end-to-end stock-device test. A successful read check alone is not proof of a successful flash/restore.

## Keep the private workspace

The GitHub repository contains source and documentation. It excludes the original firmware, bootloader, calibration/settings data, serial, generated packages, and USB logs. A fresh clone is not a recovery backup.

Keep the original `backups/`, `artifacts/`, `tools/`, `vendor/`, and ignored `local/device.json`, plus another copy of the recovery material. The configured stock directory includes:

- `stock-app.bin` and `stock-restore.mencrypt`;
- `loader-00000004-00003fff.bin`, SHA-256, and evidence of two matching reads;
- calibration and internal-tail snapshots;
- `external-first-3MiB.bin`, acquired through stock firmware.

The configured external backup directory includes `external-4MiB.bin` and `verified.json`, recording two matching reads, its SHA-256, and agreement with the stock 3 MiB prefix.

## Normal complete restoration

Connect only the side HID port to the PC, with the measurement ports disconnected. Use a prepared workspace with that meter's verified stock restore package, full original NOR backup, loader baseline and configured paths:

```powershell
.\scripts\restore-stock.ps1
```

The script uses `local/device.json` and the built host CLI. It enters native DFU, restarts the game maintenance service, pauses gameplay, validates the original full-NOR backup and protected tail, restores the original first 3 MiB, and reads back all 4 MiB. It then restores the original application, verifies the whole application and unchanged loader, and checks that stock HW2.3 firmware 2.0.7 starts.

The final 1 MiB is never erased or programmed by the game service. A protected-tail mismatch stops restoration. Each run creates new timestamped evidence directories under ignored `artifacts/`.

## If the game does not respond

Disconnect the HID cable. Hold Exit while reconnecting it, then release Exit once the native update/DFU screen appears. This entry path is independent of application USB. Run the normal restoration script from that state.

If the custom application itself cannot start, first reinstall the locally saved, verified DOOM package to regain the external-NOR restoration service:

```powershell
. .\scripts\env.ps1
. .\scripts\local-config.ps1
$cli = '.\vendor\km003c-firmware\target\release\km003c-host-cli.exe'
& $cli flash $taskDeviceConfig.doom_image --evidence ('artifacts/rescue-' + (Get-Date -Format yyyyMMdd-HHmmss))
if ($LASTEXITCODE -ne 0) { throw 'Custom application recovery failed' }
.\scripts\restore-stock.ps1
```

The flash command verifies the application's full readback and the byte-exact private loader baseline. It does not replace the bootloader or ICG. The stock application alone is not the complete recovery sequence after game resources have replaced the original NOR prefix.

## Offline evidence check

For new bootstrap acquisitions, verify the stock and native-DFU evidence without USB:

```powershell
.\bootstrap\target\release\km003c-backup.exe verify --stock backups/my-device-stock --require-dfu
```

This does not check a later full-NOR backup or prove a completed restoration. `game-backup` and the restoration commands validate their own full-NOR/readback evidence. For the original experiment's historical evidence layout only:

```powershell
python scripts/check-recovery.py
```

This checks the saved backup hashes, matching-read records, full-NOR prefix agreement, loader snapshots, and recorded complete restoration. It expects the original experiment's local evidence layout and does not contact USB.
