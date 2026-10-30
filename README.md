# APDO DOOM — independent firmware

[日本語](README.ja.md)

This project complies with **EPR (Extended Pixel Range)**. Under intense combat,
**AVS (Adjustable Violence System)** dynamically optimizes violence and frame rate
in 100 mHz increments, finally delivering an **APDO (Augmented Power DOOM Object)**
to the KM003C screen. (The USB-PD terminology is a joke.)

This branch, `codex/independent-firmware`, rebuilds the KM003C application as a
DOOM-only program. **It is under development. No public prebuilt release is offered.**
The working v8 implementation remains on `main`.

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
No prebuilt package will be published as part of this branch work. Source and
binary release completeness, dependency notices and recovery instructions must
be checked before a later release. See [third-party notices](THIRD_PARTY_NOTICES.md).
