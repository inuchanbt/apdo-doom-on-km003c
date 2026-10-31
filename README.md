# APDO DOOM — independent firmware

[日本語](README.ja.md)

This project complies with **EPR (Extended Pixel Range)**. Under intense combat,
**AVS (Adjustable Violence System)** dynamically optimizes violence and frame rate
in 100 mHz increments, finally delivering an **APDO (Augmented Power DOOM Object)**
to the KM003C screen. (The USB-PD terminology is a joke.)

This branch, `codex/independent-firmware`, rebuilds the KM003C application as a
DOOM-only program. **Experimental firmware is available as a Pre-release.**
The legacy v8 implementation remains on `main`; this download is the independent build.

## Experimental firmware download — read the risks first

[Download v0.1.0-independent](https://github.com/inuchanbt/apdo-doom-on-km003c/releases/tag/v0.1.0-independent).
The recommended `-firmware.zip` includes the application-only `.mencrypt`,
notices and installation guide. Also obtain the separate Freedoom E1M1 data ZIP,
and the [prebuilt installation guide](docs/INSTALL-PREBUILT.md). Matching complete
source, upstream/dependency notices, a release manifest and SHA-256 checksums
are supplied alongside them. The automatic GitHub source archive alone omits
upstream/dependency sources; use the explicitly attached `-source.tar.gz`.

**Installing this can permanently disable your meter or lose factory data.
Recovery is not guaranteed; repair or replacement may be necessary.**

- Tested on **one HW2.3 meter originally running stock 2.0.7**. Other revisions,
  firmware versions and individual units are not validated. Do not force a mismatch.
- This replaces the tester application with DOOM. **USB measurement, PD negotiation
  and the stock tester UI are unavailable while it is installed.**
- Game installation changes the first 3 MiB of external NOR. Acquire your own
  verified stock/DFU snapshots and restore package **before flashing**, then a full
  verified original 4 MiB NOR backup **before writing any game data**.
- Returning to stock requires the original NOR contents **and** original app in
  the documented order. A stock app file alone is not a complete rollback.
- Interrupted writes, wrong hardware, firmware bugs or mismatched backups may
  prevent recovery. The new full first-install/NOR-write/restore sequence has
  **not been qualified end-to-end on hardware**; this release is for experimenters.
- Connect **only the side HID port** and leave both measurement connectors empty.
  Never write a bootloader/ICG or use another unit's recovery data.

The update file contains no stock app, loader, ICG, calibration or device identifier.
It contains the Freedoom startup logo, but **not the playable game pack**. The
`.mencrypt` has been flashed and read back using this project's qualified host
workflow; loading it through the manufacturer's graphical updater is **untested**.
This is an unofficial, experimental project, supplied without project warranty; manufacturer approval/support is not claimed. See [release details](docs/BINARY-RELEASE.md).

![Web controller](docs/images/km003c-web-controller.jpg)

## What is independent?

The active build does not fetch or link `okhsunrog/km003c-firmware`, its board
crates, or the generated `hc32f460-pac`. Board startup, GPIO, panel/backlight,
shared LCD/NOR transport and the DOOM-only USB interface live in `independent/`.
An explicitly MIT-licensed subset of `hc32f460-embassy-rs` supplies checked clock
transitions, interrupt routing and TimerA timekeeping. It retains its attribution
and tests. This is not a claim of a clean-room implementation: hardware information
from the earlier investigation remains useful.

The GPL DOOM engine, Freedoom data, square projection, short-read cache, Steady
frame pacing and Web controller are retained. The host's native-loader transport
still uses the separately licensed, pinned `km003c-lib`.

## Build and recovery

See [BUILD.md](docs/BUILD.md) and the [implementation/provenance notes](docs/INDEPENDENT.md).
The firmware, host and read-only backup utility can be built without any device
backup. **Flashing still requires your own verified recovery material.** The new
host reads that material at runtime; it embeds neither a loader dump nor a serial.

Start an untouched meter with [FIRST-BACKUP.md](docs/FIRST-BACKUP.md).
It covers independent initial acquisition, game installation and ordered restoration.
The complete install/restore sequence is still being qualified on hardware.
Do not use `docs/legacy/`, legacy overlays or the old board patch as build instructions
for this branch. They describe the previous implementation.

## Hardware risks

Only HW2.3 / stock 2.0.7 recovery has been investigated. Unsupported revisions,
interrupted writes, firmware bugs or a mismatched backup can leave the meter
unusable; recovery is not guaranteed. Keep measurement connectors empty and use
only the side HID connection during installation and play. This firmware does not
provide USB measurements, PD negotiation or the stock tester UI. Restoring those
features requires the original application and external NOR contents, including
calibration/device data. Never flash a bootloader or another unit's recovery dump.

## Status and license

Check [INDEPENDENT.md](docs/INDEPENDENT.md) for the recorded validation status.
The release pairs the tested application with corresponding source and license
notices; it does not make the legacy v8 binary redistributable or guarantee
installation/recovery. See [third-party notices](THIRD_PARTY_NOTICES.md).
