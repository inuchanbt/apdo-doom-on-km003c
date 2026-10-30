# First backup on your own KM003C

The independent `bootstrap/` crate builds **without a previous backup, a private loader binary, `local/device.json`, or the board firmware checkout**. Its commands are `inspect`, `stock`, `dfu-check`, and offline `verify`. It uses the exact pinned `km003c-lib` revision in `Cargo.toml` and `Cargo.lock` for authenticated reads. It exposes no flash, erase, programming, reboot, application-start, LCD-settings, or PD-control command. In the stock application it opens a normal vendor session with Connect/authentication and memory-read requests, with USB reset disabled; "read-only" refers to device storage and settings, not the direction of USB packets.

The initial acquisition currently accepts **KM003C HW2.3 with stock firmware 2.0.7**. Other revisions stop before acquisition. On 2026-10-30, stock acquisition, manual DFU double reads, offline verification, restore-package generation and fresh baseline import were verified on the original meter after restoring its stock application and all external NOR. The owner also reported normal operation of the PD functions they checked after restoration. This remains an experimental workflow verified on **one restored unit**, not a second, previously untouched meter. See [the validation record](VALIDATION.md#bootstrap-hardware-check-2026-10-30); no compatibility or recovery guarantee is implied for other meters.

## 1. Build before connecting the meter

Clone this repository and install the **host** toolchain described in [BUILD.md, section 2](BUILD.md#2-configure-the-toolchains): Git, Rust Windows GNU, and LLVM-MinGW (`llvm-dlltool`). The ARM target, Python, Freedoom, and board sources are not needed for the bootstrap build.

From the repository root:

```powershell
.\scripts\build-backup.ps1
$backup = '.\bootstrap\target\release\km003c-backup.exe'
& $backup --help
```

Keep `bootstrap/Cargo.lock`; `build-backup.ps1` uses `--locked`. It builds software only and does not open USB. You do not need a dummy loader file or somebody else's firmware dump.

## 2. Acquire from the untouched stock application

Close the Web-controller server and official meter software, and let their USB helper processes finish, so they cannot compete for USB. Attach **one meter**, using only its side HID connector, with both measurement connectors empty. Start normally into the original measurement application. Keep the same PC USB port and cable through the DFU check.

```powershell
& $backup inspect
```

Confirm the model, hardware, firmware and serial printed for your meter. Replace `YOUR_DEVICE_SERIAL` below with that serial. Create only the parent `backups` directory; the output directory itself must not exist.

```powershell
New-Item -ItemType Directory -Force backups | Out-Null
& $backup stock --serial YOUR_DEVICE_SERIAL --output backups/my-device-stock
if ($LASTEXITCODE -ne 0) { throw 'Acquisition incomplete: stop here' }
& $backup verify --stock backups/my-device-stock
if ($LASTEXITCODE -ne 0) { throw 'Offline verification failed: stop here' }
```

Each range is read twice, compared byte for byte, then saved with SHA-256 and a matching-read record:

| File | Device address | Length |
| --- | --- | --- |
| `stock-app.bin` | `0x00004000` | Committed application length, additionally checked against its original CRC, metadata and vectors |
| `loader-00000004-00003fff.bin` | `0x00000004` | 16,380 bytes |
| `internal-tail-00070000.bin` | `0x00070000` | 65,536 bytes |
| `calibration-03000c00.bin` | `0x03000c00` | 1,024 bytes |
| `external-first-3MiB.bin` | `0x98000000` | 3,145,728 bytes |

`manifest.json` binds these files to the stock identity and physical USB port. Offline verification also checks the model/hardware embedded in the loader and the serial embedded in calibration. An incomplete directory or mismatching second read is a failure, even if some `.bin` files were saved. Preserve it for diagnosis; retry into a **new** directory. Do not hand-edit verification records to make them pass.

This captures the loader starting at address 4, as used by the existing update allowlist. It is **not a complete MCU/SWD image** or a replacement bootloader for recovering a damaged loader/ICG.

## 3. Confirm the independent native DFU read path

Unplug HID. Hold **Exit** while plugging it back into the **same PC USB port**, then release Exit after the native update/DFU screen appears. This operation is manual: the bootstrap tool never sends a reboot or mode-switch command.

```powershell
& $backup dfu-check --stock backups/my-device-stock
if ($LASTEXITCODE -ne 0) { throw 'DFU qualification failed: stop here' }
& $backup verify --stock backups/my-device-stock --require-dfu
if ($LASTEXITCODE -ne 0) { throw 'Recovery evidence failed: stop here' }
```

DFU does not expose the stock application serial. The tool selects the recorded physical port, checks model/hardware, reads the loader **and full installed application twice**, and requires exact agreement with the stock acquisition. Keep other meters disconnected. Success writes `dfu-recovery-check/verified.json`, bound to the exact stock manifest and both readbacks. The device remains in DFU. Unplug/replug normally and confirm the stock application still starts.

If entry, any read, identity check, or comparison fails, keep the stock firmware and investigate. A passing read check proves this read path works; it cannot guarantee a future flash write or recovery from hardware/loader damage.

## 4. Import this unit's baseline and make its restore package

Keep another copy of the entire stock directory on separate storage. The files contain private firmware, calibration and identifiers: do not attach them to public issues or commit them.

```powershell
New-Item -ItemType Directory -Force local | Out-Null
Copy-Item local.example.json local/device.json
```

Edit `local/device.json`: set your serial, `stock_backup` to `backups/my-device-stock`, and `external_backup` to `backups/my-device-external`. Keep the generated package path appropriate for [BUILD.md](BUILD.md). Python is required for the source reconstruction below.

```powershell
python scripts/prepare-sources.py
if ($LASTEXITCODE -ne 0) { throw 'Source/baseline preparation failed' }
.\scripts\build-host.ps1
$cli = '.\vendor\km003c-firmware\target\release\km003c-host-cli.exe'
& $cli package-installed backups/my-device-stock/stock-app.bin backups/my-device-stock/stock-restore.mencrypt
if ($LASTEXITCODE -ne 0) { throw 'Restore package creation failed' }
```

For a new baseline, source preparation first runs the independent **offline** verifier with `--require-dfu`, checks that the configured serial agrees, then imports the loader into the ignored board checkout. Existing experiment workspaces with their already imported baseline retain the legacy verification path. The normal host's byte-exact loader allowlist, hardware checks and build-time serial policy remain active.

`package-installed` does not access USB. It verifies the backup's original committed CRC before packaging, then requires that parsing the `.mencrypt` reproduces **every original installed byte**. The output must be new. Copy this restore package to the second backup too. The upstream host's older `stock-backup` command is not the first-acquisition entry point.

## 5. Complete the original NOR backup before installing game data

**The first 3 MiB is not a full external-NOR backup.** On the original meter, stock reads supplied the prefix; a native DFU storage-read attempt was rejected. The complete 4 MiB was read through the custom application's maintenance service, before any game-resource write. The bootstrap tool deliberately makes no claim to acquire the last 1 MiB from stock.

Finish stock-function tests and settings changes **before** acquiring the recovery snapshots. Keep the meter idle from stock acquisition through the full-NOR backup. During the reference test, running stock PD checks between acquisitions changed some NOR contents; the prefix comparison correctly refused the stale snapshot. If this happens, preserve the failed capture, acquire a fresh stock/DFU set and matching full-NOR backup, and keep the earlier snapshots separately. Do not edit verification flags to bypass a mismatch. The active stock and external-backup paths must refer to the same consistent snapshot set.

Continue [BUILD.md](BUILD.md) to generate data and build the custom application. The following is the later **application-write** stage, not part of the read-only bootstrap. Perform it only after steps 2–4 pass and the stock restore package and additional copy exist. Hold Exit while reconnecting HID to enter native DFU:

```powershell
& $cli flash artifacts/km003c-doom.mencrypt --evidence artifacts/first-custom-flash
if ($LASTEXITCODE -ne 0) { throw 'Application flash/readback failed: stop here' }
& $cli start
if ($LASTEXITCODE -ne 0) { throw 'Custom service failed: stop here' }
& $cli game-backup backups/my-device-external --stock-prefix backups/my-device-stock/external-first-3MiB.bin
if ($LASTEXITCODE -ne 0) { throw 'Full NOR backup failed: do not install game data' }
```

The first flash verifies the application readback and unchanged loader; it does not write external NOR. With the original prefix still present, the custom application stays in maintenance rather than starting DOOM. `game-backup` reads all 4 MiB twice, requires the prefix to match the stock acquisition, and saves `external-4MiB.bin` plus `verified.json`, including the protected-tail hash. The output directory must be new. Keep a second copy of this directory **before** proceeding:

```powershell
& $cli game-install vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT backups/my-device-external artifacts/first-game-install
if ($LASTEXITCODE -ne 0) { throw 'Game data verification failed' }
& $cli game-start
```

Game installation writes only the first 3 MiB, requires the verified full backup, checks game readback, and requires the final 1 MiB to remain unchanged. If obtaining the full backup fails, do not issue `game-install`: the stock application can still be restored from native DFU using `flash .../stock-restore.mencrypt --evidence <new-directory>` and `stock-start`, since external NOR has not been modified. After game installation, complete restoration must restore NOR **before** the stock application; see [RECOVERY.md](RECOVERY.md).

## Offline development checks

```powershell
. .\scripts\env.ps1
cargo test --manifest-path bootstrap/Cargo.toml --locked
cargo clippy --manifest-path bootstrap/Cargo.toml --all-targets --locked -- -D warnings
python scripts/test-prepare-sources.py
python scripts/test-check-recovery.py
```

These tests use synthetic data and no USB. They cover mismatching/short reads, overwrite refusal, original application CRC failure despite updated hashes, fixed-range/path checks, calibration identity, missing DFU evidence, and disagreement with either DFU readback. Verification records establish local consistency, not a cryptographic signature or a guarantee that a future device write will succeed.
