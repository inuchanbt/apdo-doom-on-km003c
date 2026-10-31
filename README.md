# APDO DOOM — independent firmware

[日本語](README.ja.md)

This project complies with **EPR (Extended Pixel Range)**. Under intense combat,
**AVS (Adjustable Violence System)** dynamically optimizes violence and frame rate
in 100 mHz increments, finally delivering an **APDO (Augmented Power DOOM Object)**
to the KM003C screen. (The USB-PD terminology is a joke.)

`main` contains the independent DOOM-only application for the KM003C.
**Experimental firmware is available as a Pre-release.** The previous v8
implementation and its tools are preserved on the [legacy branch](https://github.com/inuchanbt/apdo-doom-on-km003c/tree/legacy).
See [branch and tool migration](docs/BRANCHES.md) when updating an existing workspace.

## Experimental firmware download — read the risks first

**HW2.1 incompatibility report:** A [community tester](https://x.com/tomoca2/status/2116438041766375717)
reports that a **September 2023 HW2.1 unit freezes at the title screen**. Attempts
to restore the original firmware also errored; the tester reports successful
recovery using forced DFU mode. The exact firmware build and cause have not been
confirmed, and this report has not been independently reproduced. **HW2.1 is not
supported; do not bypass the HW2.3 checks.** Recovery on that unit does not
guarantee recovery on another. See [compatibility reports](docs/VALIDATION.md#external-hw21-failure-report).

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
  prevent recovery. On **2026-10-31**, the owner confirmed the independent
  first-install/game-data-write/complete-stock-restore workflow on the same
  HW2.3 unit. This does not guarantee recovery on another unit or after a failure.
- Connect **only the side HID port** and leave both measurement connectors empty.
  Never write a bootloader/ICG or use another unit's recovery data.

The update file contains no stock app, loader, ICG, calibration or device identifier.
It contains the Freedoom startup logo, but **not the playable game pack**. The
`.mencrypt` has been flashed and read back using this project's qualified host
workflow; loading it through the manufacturer's graphical updater is **untested**.
This is an unofficial, experimental project, supplied without project warranty; manufacturer approval/support is not claimed. See [release details](docs/BINARY-RELEASE.md).

![Web controller](docs/images/km003c-web-controller.jpg)

[Watch the hardware demo — 60 seconds](docs/videos/apdo-doom-on-km003c.mp4).
The recording shows the earlier implementation; the game still runs on the meter.

## Play from the PC

After completing the [installation guide](docs/INSTALL-PREBUILT.md), run
`scripts/play.ps1` and open its printed HTTP URL in a normal browser. Keep the
server running and click **Enable keyboard**. Use W/S or ↑/↓ to move, A/D or ←/→
to turn, Space/Ctrl to fire, E to use, Esc for the menu and Enter to confirm.
Turn off **Live preview** when prioritizing smooth motion on the meter.
See [PC controls](docs/PC-CONTROLS.md) and [screen capture](docs/SCREEN-CAPTURE.md).

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
The owner confirmed the complete independent install/restore sequence on
2026-10-31; [validation details](docs/VALIDATION.md) distinguish that report from
automated checks. Use the `legacy` branch only for the previous implementation;
its host tools and recovery commands are not interchangeable with this version.

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
