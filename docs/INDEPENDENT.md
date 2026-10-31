# Independent firmware implementation

Work happens on `codex/independent-firmware`; `main` retains the tested v8.
The active firmware is `independent/firmware`, the USB wire format is
`independent/protocol`, and the PC helper is `independent/host`.

## Implementation and provenance

New board code implements application vectors/metadata, the recovery request,
GPIO, synchronous open-drain AW9523 I2C, ST7789 commands, shared QSPI ownership,
NOR transactions, physical controls and the DOOM-only WinUSB interface.
Hardware register facts and wiring established during the earlier investigation
are reused. We reviewed existing implementations; this is **not** a strict
clean-room implementation or a claim that no upstream work was consulted.

`independent/hardware` is an explicitly MIT-licensed subset of
[hc32f460-embassy-rs](https://github.com/okhsunrog/hc32f460-embassy-rs/tree/23e09a053ddf5284d28c06407b9172adf9c48a34):

- `clock_decode.rs`, `clocks.rs`, `clocks/startup.rs` and their tests;
- `intc.rs` and its tests;
- `time_driver_tmra.rs` and its tests.

Its [MIT notice](../independent/hardware/LICENSE-MIT) is retained. Changes remove
the generated PAC's TimerA ownership token; the board owns that timer directly.
The timer tick feature is fixed to the same 32,768 Hz used by the application.
Formatting and module paths are adapted. No PAC, SVD file or other HAL modules
are compiled. Register facts can also be checked against the vendor's
BSD-3-Clause CMSIS header (notice under `notices/hc32/`).

The C engine and square projection remain GPL-2.0-or-later and retain their
original source/patches and upstream notices. Freedoom assets retain their BSD
license. The USB stack and Cortex-M runtime retain their package licenses.
`km003c-lib` is used only by the host/backup utilities, under its MIT/Apache terms;
it is not linked into the MCU application.

## Design

There are no measurement, USB-PD, accelerometer or INA229 tasks or RPC endpoints.
LCD/NOR transfers belong to the game task; USB never accesses QSPI concurrently.
The initial panel setup uses generic ST7789 sleep-out, pixel format, orientation,
inversion and display-on commands rather than the old board initialization table.

The original Freedoom Phase 1 menu logo appears centered at 216x50 during the
existing three-second boot window and stays visible while game data is checked.
It is also captured by the PC preview before the game runs. Holding OK still
cancels automatic startup; the banner then remains on the service screen.
The ROM asset adds 11,770 bytes and uses a single 480-byte column for drawing.

The square renderer keeps the 90-degree horizontal field of view and complete
HUD. Steady mode retains a four-tic (114.286 ms) frame target. The short-read cache
and 32 KiB SRAMH arena are unchanged. Faster 16 MHz NOR reads require a full
comparison with 8 MHz on each startup; `-SafeNor` keeps 8 MHz.

The preview has a live 120x120 RGB565 shadow and an immutable snapshot. Requests
carry a version, opcode and sequence; replies are bounded to 1024 data bytes.
Full-size USB packets receive a terminating zero-length packet. A screenshot's
snapshot ID must match for every chunk. Input expires after 750 ms at the meter;
the host stops refreshing nonzero input after 500 ms without a fresh command.
Physical Exit held for two seconds requests the native loader.

The storage service permits game-page writes only before engine initialization,
within the first 3 MiB, without crossing a 256-byte page. Sector erase requires
alignment and confirmation. The final 1 MiB is read-only to this service.

The host packager and backup utility build with no device files. Storage backup
reads the original 4 MiB twice; install/restore checks both complete current regions
before erase, preserves bytes outside the pack and verifies all 4 MiB afterwards.
The storage algorithm has model tests for unknown changes, page boundaries and
complete round-trip restoration.

The host packager and backup utility build with no device files. Flashing reads a
qualified backup at runtime, selects its recorded physical port, verifies the
loader, uses the pinned library's authenticated updater, then reads back the
entire application and unchanged loader. Programming is not retried, and starting
an application is a separate command with another image comparison.

## Validation status

- Independent ARM firmware and Windows host compile; Clippy checks pass.
  The final local application is 225,256 bytes (`2.0.7-apdo2`).
- 48 offline/model/HTTP tests pass across the hardware subset, wire format,
  packager/storage planner, backup utility, source preparation and controller.
- Startup banner: original archive/asset hashes verified; device preview pixels
  exactly match the expected logo render. The banner build also auto-started DOOM.
- Firmware dependency inspection excludes the old board crates, generated PAC
  and PD stack.
- Clock/routing/timer model tests and wire boundary tests pass.
- Real-pack engine tests pass: projection parity, 1007 cached reads, 140 frames,
  movement/turn/fire/menu, full HUD redraw and input cadence after a slow draw.
- Native HW2.3: application flash/full readback and unchanged loader verified;
  the independently initialized LCD/NOR/AW9523 and USB support continuous DOOM
  rendering. Web input was acknowledged, changed the captured view, and released
  to zero afterwards. Preview capture was checked at 120x120. Native DFU entry
  and verified app restart were also exercised using the new host; reset-related
  USB errors are resolved by checking the same physical port for actual DFU.
- The initial I2C SDA/SCL assignment error returned to native DFU; retained panic
  text located it, and the corrected PC13 SDA / PH2 SCL build booted successfully.
- Physical screen/buttons confirmation is pending. New independent NOR write /
  complete stock restore commands have model tests, but have not been exercised
  on the device; do not call the full new install/restore workflow qualified yet.

The legacy overlays, board patch and archived documentation are historical inputs
to v8 and are **not** used by the independent build. They retain their prior
provenance and are not relicensed by this work. The explicit experimental release source bundle
includes independent source, pinned engine, Cargo/runtime source and dependency
notices, and excludes private backups/build caches. The main branch retains
legacy firmware code, with README links to the independent Pre-release.
