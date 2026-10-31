# First backup and independent installation (development workflow)

**Hardware flashing can render the meter unusable. Recovery is not guaranteed.**
This procedure targets HW2.3 / stock 2.0.7 only. Keep measurement connectors empty;
use only the side HID port. Use one meter and the same PC USB port throughout.
Backups contain private device/calibration data: keep them off GitHub.

The independent build requires no backup. Write operations require your own
qualified backup at runtime. On 2026-10-31 the owner confirmed this independent
workflow from initial installation through complete stock restoration on the
same HW2.3 unit. Other units remain untested; see [VALIDATION.md](VALIDATION.md).

## 1. Build the read-only acquisition tool

```powershell
.\scripts\build-backup.ps1
bootstrap/target/release/km003c-backup.exe inspect
```

This executable contains no erase/program/update/reboot/settings/PD command and
no embedded private baseline. Confirm the model, hardware, firmware and serial.
Stop if this is not an untouched stock HW2.3 / 2.0.7 application.

## 2. Acquire the original regions twice

```powershell
bootstrap/target/release/km003c-backup.exe stock --serial YOUR_SERIAL --output backups/stock-original
```

Use a new output directory. The tool acquires the installed app, loader bytes
`4..0x3fff`, internal tail, calibration region and the first 3 MiB of external NOR.
Each region is read twice; CRC/SHA checks and the identity are recorded. The
original app's committed size/CRC must validate. This is **not yet a complete
4 MiB NOR backup**; do not write game data at this stage.

## 3. Qualify manual DFU recovery

Unplug HID. Hold Exit while inserting the same HID cable into the same PC USB
port, then release Exit after the update/DFU screen appears.

```powershell
bootstrap/target/release/km003c-backup.exe dfu-check --stock backups/stock-original
bootstrap/target/release/km003c-backup.exe verify --stock backups/stock-original --require-dfu
```

This step reads the loader/app twice in native DFU, compares them with the stock
acquisition and records qualification. It does not write or start firmware.
Missing evidence, a different port or any mismatch must be investigated.

## 4. Build the independent app and original restore container

Follow [BUILD.md](BUILD.md) to prepare the public engine/data and toolchains, then:

```powershell
.\scripts\build-host.ps1
independent/target/release/apdo-host.exe package-installed backups/stock-original/stock-app.bin backups/stock-original/restore.mencrypt
.\scripts\build-firmware.ps1 -Python python -Name apdo-independent
```

`package-installed` verifies the original committed CRC and ensures that the
restore package reproduces the installed app byte-for-byte. Keep an offline copy
of the entire stock backup and restore package before the next step.

## 5. Install the app, then acquire the complete NOR

While still in the qualified native DFU mode:

```powershell
independent/target/release/apdo-host.exe flash backups/stock-original artifacts/apdo-independent.mencrypt artifacts/first-app-flash
independent/target/release/apdo-host.exe start backups/stock-original artifacts/apdo-independent.mencrypt
independent/target/release/apdo-host.exe nor-backup backups/stock-original backups/nor-original
```

Flash compares the loader before programming, verifies every app byte afterwards
and confirms unchanged loader bytes. Failure stays in DFU without retrying writes.
The new app starts a DOOM-only service. With stock NOR contents it does not start
the engine and keeps the Freedoom banner visible; use the PC helper to confirm service readiness. The backup command pauses automatic boot, reads all 4 MiB twice and
requires its first 3 MiB to match the qualified stock acquisition. The full image,
SHA-256 and qualification are saved. If game data was already installed, this
command refuses to describe that prefix as original stock data.

## 6. Install the exact Freedoom pack

```powershell
independent/target/release/apdo-host.exe install-game backups/stock-original backups/nor-original vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT artifacts/game-preflight --dry-run
independent/target/release/apdo-host.exe install-game backups/stock-original backups/nor-original vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT artifacts/game-install
independent/target/release/apdo-host.exe play
python scripts/pc-controller.py --port 50594
```

Use a new evidence directory for each command. The installer accepts only the
pinned pack hash. Before its first erase it verifies the full original backup,
physical USB port, protected 1 MiB tail and the entire current prefix. That prefix
must be either the qualified original or the exact planned game image. Unknown
changes are refused. Only changed sectors below 3 MiB are programmed. Bytes beyond
the pack retain their original values, including the end of the final sector.
The entire 4 MiB is read back and compared. Programming errors are not retried.

DOOM auto-starts on later power cycles. Hold OK during startup to cancel automatic
boot and retain the storage service. Holding Exit for two seconds requests native
DFU. Normal play and screenshots never issue NOR program/erase commands.

## 7. Restore original tester functionality

Stop the Web controller and enter native DFU with `apdo-host loader`. Hold the
physical OK button while starting the independent app again:

```powershell
independent/target/release/apdo-host.exe start backups/stock-original artifacts/apdo-independent.mencrypt
```

Release OK after startup. `info` must show no rendered frames; simply pausing an
already initialized engine does not enable NOR writes. Then restore NOR first:

```powershell
independent/target/release/apdo-host.exe restore-nor backups/stock-original backups/nor-original vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT artifacts/nor-restore
independent/target/release/apdo-host.exe loader
independent/target/release/apdo-host.exe flash backups/stock-original backups/stock-original/restore.mencrypt artifacts/stock-app-restore
independent/target/release/apdo-host.exe start backups/stock-original backups/stock-original/restore.mencrypt
```

Do not continue to the stock app if full NOR restoration/readback failed. The
protected tail must match and is never rewritten by the service. Stock app flash
also requires exact app readback and unchanged loader. Stock start checks model,
hardware and firmware identity after re-enumeration. Confirm the stock UI and
functions physically. Never write a loader, ICG or another unit's backup.

For a failed startup, `apdo-host diagnostics STOCK_BACKUP NEW_OUTPUT_DIR` performs
read-only retained-RAM acquisition while in native DFU. Preserve that evidence
before deciding how to recover; do not repeatedly retry programming after an
ambiguous transfer failure.
