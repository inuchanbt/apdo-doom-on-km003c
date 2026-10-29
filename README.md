# APDO — DOOM on KM003C

[English](README.md) · [日本語](README.ja.md)

> This project complies with EPR (Extended Pixel Range). In demanding combat scenes, AVS (Adjustable Violence System) dynamically optimizes violence and frame rate in 100 mHz increments. The result is delivered to the KM003C display as an APDO (Augmented Power DOOM Object)—

*EPR, AVS, and APDO above are USB-PD wordplay. The actual implementation is a native DOOM port running at roughly 6 fps.*

**Yes, the POWER-Z KM003C USB-C power meter runs DOOM.** More precisely, it runs the low-memory DOOM engine with Freedoom's E1M1 data on its own MCU. The PC builds, flashes, and verifies it; gameplay runs on the meter.

**[▶ Watch the hardware demo — 60 seconds · MP4 · 5.1 MB](docs/videos/apdo-doom-on-km003c.mp4)**

![Web controller with a live capture from the KM003C](docs/images/km003c-web-controller.jpg)

*Web control panel with a live 120×90 capture from the actual meter, composited into the supplied KM003C image. The meter renders the game; the browser controls and previews it.*

## What works

| Item | Verified result |
| --- | --- |
| Hardware | One POWER-Z KM003C HW2.3 unit; original firmware 2.0.7 |
| Execution | Native Cortex-M4 / HC32F460 at 192 MHz |
| Game | Freedoom 0.13.0, E1M1 |
| Display | 240×180, centered on the 240×240 LCD |
| Performance | About 171–173 ms per rendered frame, roughly 6 fps |
| Controls | Movement, turning, shooting, use, and menu inputs |
| PC controls | Browser keyboard/mouse panel over USB; device gameplay confirmed |
| Meter LCD | v7 adds brightness buttons and manual 90° rotation from the PC |
| PC preview | v6 captures the LCD game output at 120×90 RGB565; frame download included |
| Startup | Automatic startup after about 3 seconds; unplug/replug and gameplay confirmed on the device |
| Recovery | Original application and all 4 MiB of external NOR restored and verified; stock firmware boot confirmed |

There is no sound, save support, persistent settings, or supported transition to another map in this build. The DOOM configuration does not start the meter's measurement or PD tasks. Use the side HID port for power and leave the measurement connectors disconnected.

This is an experimental port verified on **one unit**, not a compatibility claim for every HW2.3 meter. Updates require an exact match with a privately saved loader baseline. Stock firmware, bootloader dumps, calibration data, device serials, and compiled firmware are excluded from this repository.

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

## Source and build

The tested environment is Windows with PowerShell, Python 3, the Rust Windows GNU toolchain, and LLVM-MinGW. Upstream repositories are pinned in [SOURCE_LOCK.json](SOURCE_LOCK.json); patches and added source files are versioned here.

```powershell
git clone https://github.com/inuchanbt/apdo-doom-on-km003c.git
cd apdo-doom-on-km003c
python scripts/prepare-sources.py --sources-only
```

This reconstructs the source without accessing USB. For the **first backup**, build the independent read-only tool with `.\scripts\build-backup.ps1`; it needs no prior backup or private loader. Follow [first acquisition and verification](docs/FIRST-BACKUP.md) to obtain your own recovery material before building the hardware-update host. Initial acquisition accepts HW2.3 / stock 2.0.7; the new bootstrap sequence still needs an end-to-end test on untouched stock hardware. See [the build guide](docs/BUILD.md) for toolchains, Freedoom conversion, and the subsequent builds.

```powershell
.\scripts\build-host.ps1
.\scripts\build-firmware.ps1
```

These build commands do not flash a device. The repository does not include a universal installer or a ready-to-flash release.

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
