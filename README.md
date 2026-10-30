# APDO — DOOM on KM003C

[English](README.md) · [日本語](README.ja.md)

> This project complies with EPR (Extended Pixel Range). In demanding combat scenes, AVS (Adjustable Violence System) dynamically optimizes violence and frame rate in 100 mHz increments. The result is delivered to the KM003C display as an APDO (Augmented Power DOOM Object)—

*EPR, AVS, and APDO above are USB-PD wordplay. The implementation is a native DOOM port; measured performance is listed below.*

**Yes, the POWER-Z KM003C USB-C power meter runs DOOM.** More precisely, it runs the low-memory DOOM engine with Freedoom's E1M1 data on its own MCU. The PC builds, flashes, and verifies it; gameplay runs on the meter.

## Flashing risks — read first

**This experimental firmware can leave your meter unusable and can cause loss of
factory data. Repair or replacement may be necessary. Recovery is not guaranteed,
even with backups.** A wrong image, an interrupted update, hardware differences
or software faults can prevent normal boot or USB access.

- Only **one KM003C HW2.3 unit, originally on stock 2.0.7**, has been tested.
  This does not establish compatibility with every HW2.3 unit or other revisions.
- Before the first custom-application flash, verify **your own meter's** stock
  and DFU backups, make its stock restore package and keep another copy. Before
  installing game data, also complete its verified **4 MiB external-NOR backup**.
  Follow [the first-backup sequence](docs/FIRST-BACKUP.md); never substitute
  another unit's loader, calibration or recovery data.
- Game data overwrites the original first **3 MiB of external NOR**. Restoring
  only the stock application does not undo that write. Full rollback requires
  the original NOR data and stock application in the [documented order](docs/RECOVERY.md).
- Exit-held power-on DFU is a recovery route while the native loader and USB
  path remain functional. It cannot guarantee recovery from a damaged loader,
  inaccessible USB, lost factory data or hardware damage.
- During flashing and gameplay, connect **only the side HID port** and leave
  both measurement connectors empty. The DOOM application replaces the normal
  measurement/PD application. This is not an official manufacturer update;
  warranty/support treatment has not been established.

**Prebuilt firmware is not currently published:** the upstream board firmware's
redistribution license is unresolved. A risk disclaimer does not replace that
permission. See [binary-release status and requirements](docs/BINARY-RELEASE.md).

**[▶ Watch the hardware demo — 60 seconds · MP4 · 5.1 MB](docs/videos/apdo-doom-on-km003c.mp4)**

![Web controller with a live capture from the KM003C](docs/images/km003c-web-controller.jpg)

*v8 controller with a 120×120 device capture inside the supplied KM003C image and the new frame-pacing selector. Image transfers are paused in this screenshot while testing native motion; the meter renders the game.*

## What works

| Item | Verified result |
| --- | --- |
| Hardware | One POWER-Z KM003C HW2.3 unit; original firmware 2.0.7 |
| Execution | Native Cortex-M4 / HC32F460 at 192 MHz |
| Game | Freedoom 0.13.0, E1M1 |
| Display | 240×180 letterbox, or v8 square mode: 240×200 world + complete 40-row HUD |
| Performance | Steady mode targets 8.75 fps; uncapped starting scene ~8.5 fps letterbox / ~9.6 fps square, preview off |
| Controls | Movement, turning, shooting, use, and menu inputs |
| PC controls | Browser keyboard/mouse panel over USB; device gameplay confirmed |
| Meter LCD | v7 adds brightness buttons and manual 90° rotation from the PC |
| PC preview | 120×90 letterbox / 120×120 square RGB565; frame download included |
| Startup | Starts after a 3-second hold-button window plus game-read qualification; device auto-start verified |
| Recovery | Original application and all 4 MiB of external NOR restored and verified; stock firmware boot confirmed |

There is no sound, save support, persistent settings, or supported transition to another map in this build. The DOOM configuration does not start the meter's measurement or PD tasks. Use the side HID port for power and leave the measurement connectors disconnected.

This is an experimental port verified on **one unit**, not a compatibility claim for every HW2.3 meter. Updates require an exact match with a privately saved loader baseline. Stock firmware, bootloader dumps, calibration data and device serials are excluded. Generated firmware is excluded from the source tree; any future eligible binaries would be separate GitHub Release assets, subject to the [release requirements](docs/BINARY-RELEASE.md).

## Controls

| Action | Buttons |
| --- | --- |
| Move forward | OK |
| Turn | Left / Right |
| Fire | Tap Exit |
| Open a door / use a switch | Left + Right |
| Open the menu | Exit + OK |
| Menu up / down | Left / Right |
| Menu confirm / back | OK / Exit |
| Stay in the USB maintenance service | Hold OK during startup |
| Return to the native update loader | Hold Exit for about 2 seconds |

Holding Exit while connecting the HID cable also enters the native update screen, independently of the application.

### Play from the PC

With the PC-control firmware (v5) and updated host installed:

```powershell
.\scripts\play.ps1
```

You can start the server before connecting the meter: it retries USB automatically, including after unplug/replug. Reconnection clears held keys; click **Enable keyboard** again to resume. Open the printed HTTP URL in any normal browser, rather than opening `web/controller.html` directly.

Click **Enable keyboard** in the local panel. Use **WASD / arrows**, **Space / Ctrl** to fire, **E** to use, **Esc** for the menu, and **Enter** to confirm. Switching away releases keys; enable them again when returning. The meter still renders the game. See [PC controls](docs/PC-CONTROLS.md) for setup, input expiry, and scripted control.

With v7, **Meter LCD** buttons adjust the physical backlight and rotate the screen left/right in 90° steps. The preview offers **Device only** (game stays upright at a fixed position, default) and **Device + game** (joke mode). Settings reset on power-up.

With v6, the same panel displays a live **120×90 RGB565 screen capture**. **Save frame** downloads a BMP. See [screen capture](docs/SCREEN-CAPTURE.md) for the resolution, memory use, and preview-rate limitations.

The **Frame pacing** selector offers Steady (8.75 fps target, default) and Uncapped. Steady evens out the varying cost of different views; local inputs are sampled before simulation to avoid an extra frame of queued old input.

v8 adds a square display build that retains the horizontal field of view and all HUD fields. The engine arena uses fast SRAMH; qualified 16 MHz NOR reads and a short-record cache improve the starting-scene render rate. Startup compares all game bytes at the original and faster read speeds before enabling faster reads. See [measurements, square mode, and the LZ4/DMA assessment](docs/PERFORMANCE.md).

## Source and build

The tested environment is Windows with PowerShell, Python 3, the Rust Windows GNU toolchain, and LLVM-MinGW. Upstream repositories are pinned in [SOURCE_LOCK.json](SOURCE_LOCK.json); patches and added source files are versioned here.

```powershell
git clone https://github.com/inuchanbt/apdo-doom-on-km003c.git
cd apdo-doom-on-km003c
python scripts/prepare-sources.py --sources-only
```

This reconstructs the source without accessing USB. For the **first backup**, build the independent read-only tool with `.\scripts\build-backup.ps1`; it needs no prior backup or private loader. Follow [first acquisition and verification](docs/FIRST-BACKUP.md) to obtain your own recovery material before building the hardware-update host. Initial acquisition accepts HW2.3 / stock 2.0.7. Stock/DFU acquisition and baseline import have been verified on the original unit after restoring stock; other units have not been tested. See [the build guide](docs/BUILD.md) for toolchains, Freedoom conversion, and the subsequent builds.

```powershell
.\scripts\build-host.ps1
.\scripts\build-firmware.ps1
```

These build commands do not flash a device. There is no universal installer or published prebuilt firmware release; [licensing and release requirements](docs/BINARY-RELEASE.md) still need to be resolved.

## Recovery and verification

The tested recovery sequence restores the original external NOR contents, reads back all 4 MiB, restores the original application, verifies that the loader is unchanged, and checks the stock firmware identity. The final 1 MiB of NOR is never writable through the game service.

For the original working directory with its private backups configured:

```powershell
.\scripts\restore-stock.ps1
```

Read [the recovery guide](docs/RECOVERY.md) before using it. A fresh clone does not contain recovery backups. Keep the original workspace and an additional copy of its backups.

[Validation notes](docs/VALIDATION.md) distinguish device checks from the host renderer and build tests. Hardware success includes actual gameplay and a complete stock restoration, not just a successful compile.

## Repository layout

| Path | Contents |
| --- | --- |
| `bootstrap/` | Independent read-only first-backup tool and offline verifier |
| `native/` | C engine bridge and host test harness |
| `overlays/` | Added Rust files for the board firmware and host tools |
| `patches/` | Changes to the pinned upstream repositories |
| `scripts/` | Source reconstruction, builds, tests, and recovery helpers |
| `web/` | Local keyboard/mouse control panel |
| `docs/` | Build, recovery, and validation documentation |
| `notices/` | Freedoom license and credits |
| `vendor/`, `tools/` | Local source checkouts and toolchains; ignored |
| `local/`, `backups/`, `artifacts/` | Private configuration, recovery data, and generated evidence; ignored |

## Credits and licenses

- [okhsunrog/km003c-firmware](https://github.com/okhsunrog/km003c-firmware): board support, Rust firmware, host tools, and native-loader workflow.
- [ArturBieniek4/DMC100-Doom](https://github.com/ArturBieniek4/DMC100-Doom): low-memory DOOM engine and game-data converter.
- [doomgeneric](https://github.com/ozkl/doomgeneric) and id Software: the engine's upstream lineage.
- [Freedoom](https://freedoom.github.io/): freely redistributable game assets. No commercial DOOM IWAD is included.
- [The custom-firmware post on r/UsbCHardware](https://www.reddit.com/r/UsbCHardware/comments/1xhtny5/running_custom_firmware_on_my_powerz_km003c/): the starting point for this experiment.

The existing [MIT license](LICENSE) covers original project material where no other license is stated. The C engine integration and DMC100-Doom changes are GPL-2.0-or-later; [COPYING](COPYING) supplies the GPL text. Upstream terms remain applicable. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for scope and asset notices.
