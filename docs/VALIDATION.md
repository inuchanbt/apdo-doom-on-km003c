# Validation — independent firmware

## Shared HID firmware reported working on 2026-11-11

The owner reported flashing `2.0.7-apdo3` to the KM003C and confirmed operation
without problems. v0.1.2 publishes the exact 225,512-byte update container from
that build (SHA-256 `666129410162fff3b1ef69888393e17edcd82ed6376b59b32b4d2d9b13f346a9`)
with its matching host. This confirms owner-reported flashing/operation; it does
not assert separately observed Web-preview performance, all HID commands or a
new full stock-install/NOR/restore round trip. The earlier round-trip evidence
below remains specific to its earlier build. The new release targets HW2.3;
the historical HW2.1 trial is not updated.

The shared-ID change passed 51 offline/model/HTTP tests, host/protocol/ARM Clippy,
ARM release linking, Windows host release building and HW2.1 host compilation.
See [USB identity and migration](USB-IDENTITY.md) for transport details.

The release also adds removal of leftover game data after the stock application
was already restored. Four USB-free Windows tests exercise the actual installer
with stub executables: complete operation order, invalid NOR backup, wrong
installed app, and NOR-restore failure. They confirm that a failed check prevents
firmware writes, and a failed NOR restoration prevents stock flashing. This
additional workflow is not a hardware test. The total is 55 tests across the HID
and cleanup changes; see [the cleanup guide](REMOVE-DOOM-DATA.md).

## Hardware round trip reported on 2026-10-31

The meter owner confirmed: the **new independent procedure from initial
installation through complete restoration** worked on the actual meter.
This refers to the same **KM003C HW2.3 originally running stock 2.0.7** used
throughout development, not a second previously untouched device.

The documented workflow covers original stock/DFU acquisition and qualification,
application installation, complete original NOR acquisition, game-data
installation and complete restoration in NOR-then-stock-app order. The tools
require byte/hash checks and protected-tail/loader checks along that path. The
latest end-to-end result is **owner-reported**; no new private raw acquisitions,
readback logs or evidence manifests were independently inspected for this
repository update. It must not be presented as an additional automated audit.

This confirms a working round trip on one unit. It does not establish recovery
from interrupted programming, compatibility with other units/revisions, the
manufacturer's GUI updater, or exhaustive charging-protocol/PD conformance.
The project remains experimental and backup requirements remain in force.

## External HW2.1 failure report

A [report by @tomoca2 on X](https://x.com/tomoca2/status/2116438041766375717),
received on 2026-10-31, describes one **HW2.1 meter manufactured in September 2023**:

- The reporter described an unresponsive title screen and a failed attempt to
  restore the original firmware from the PC application.
- The reporter recovered the unit using **forced DFU mode**.

### Follow-up supplied on 2026-11-07

The project owner supplied the reporter's additional description, screenshots,
and a read-only DFU survey after recovery.

- The reported input file was
  `apdo-doom-km003c-hw2.3-v0.1.0-independent.mencrypt`, installed with the stock
  Mtools 2.5.2 application. The PC screenshot labels the application
  **POWER-Z Lab 2.5.2** and shows `2.0.7-apdo2` in the firmware pane.
- The meter photo shows the built-in **Freedoom startup splash logo**. The
  reporter described the meter as unresponsive at that screen.
- The PC screenshot lists **DOOM / APDO DOOM**, with an older KM003C entry marked
  Offline. The update dialog shows **0%** and `Failed LIBUSB_ERROR_NO_DEVICE`.
  The firmware pane also displays HW 2.1 / FW 2.0.7. These are UI observations
  rather than a fresh stock-identity readback or a successful APDO exchange.
- The reporter confirmed that Mtools installed only the `.mencrypt`. The
  separate Freedoom E1M1 game data was never written. The reporter also observed
  no visible response to button operations. The length of the Exit press was
  unspecified.
- With a normal connection, POWER-Z Lab showed the DOOM entry as active. The
  reporter entered native DFU by holding Back/Exit during power-on, then
  selected the 2.0.7 chip icon to restore the original firmware. The reporter
  confirmed normal USB tester operation after recovery. The post-recovery
  screenshots show **KM003C APP connected**, HW 2.1, FW 2.0.7, and ROM `4MB`.
- The read-only DFU survey matched the published HW2.1 loader SHA-256
  `17d51eeff9fb6abe1a7024a108006248e0aaebdaae52de69a3e26c000565d4bf` and the
  stock 2.0.7 application SHA-256
  `c63057eaf08b8625e7ac2fc7270c06d75515e78fdbdd46080bdf4b696984ec67`.
  The original application was 456,896 bytes and its CRC passed.

### Interpretation from the tagged source

The [`v0.1.0-independent` game task](https://github.com/inuchanbt/apdo-doom-on-km003c/blob/v0.1.0-independent/independent/firmware/src/game.rs)
draws the startup splash before entering its service loop. Automatic engine
startup requires `DMCDOOM\0` at NOR offset zero and the three-second boot delay.
With an absent signature, the service keeps the splash visible. The confirmed
app-only installation explains the screen. The same task checks for a
two-second Exit hold while waiting. The [verified HW2.1 button mapping](https://github.com/okhsunrog/km003c-firmware/blob/87b3f787d1c1e2ea6213461e1e1cdfce29fb57e3/docs/reverse-engineering/board_bringup.md)
matches the tagged GPIO assignments for all four buttons.

The [tagged USB service](https://github.com/inuchanbt/apdo-doom-on-km003c/blob/v0.1.0-independent/independent/firmware/src/usb.rs)
advertises `APDO DOOM` and accepts the APDO request format. The stock updater
uses its own application protocol. Its device list records recognition of the
DOOM descriptors, while the reported update error leaves APDO service
responsiveness unresolved.

The official 225,256-byte DOOM application and the loader matching the survey
passed an offline execution of the loader's normal application-jump path. The
test ended at the reset vector. Live APDO replies, the sustained Exit action,
gameplay, and a full NOR installation and restore on HW2.1 remain unverified.

These observations describe the app-only v0.1.0 installation through Mtools. The
subsequent HW 2.1 Windows installer uses its own hardware checks and game-data
transfer.

### Private HW2.1 trial

The [Windows trial](INSTALL-WINDOWS-HW21-TRIAL.ja.md) builds its own HW2.1 tools
and pins the surveyed loader, stock application, and public DOOM image. It
checks the live splash reply and waits for the meter's physical Exit transition
to native DFU before game-data writes. It then checks the complete NOR backup,
game readback and frame progress. Restoration verifies NOR before starting the
original application and checking stock calibration.

An offline run of the matching native loader's erase, write, CRC commit and
startup path used the public DOOM and exact stock application bytes. DOOM erased
28 modeled sectors and stock erased 56. The modeled internal flash matched the
original bytes after restoration. This run models the flash and CRC peripherals.
### HW2.1 gameplay report

On 2026-11-07 the collaborator reported gameplay with the earlier HW 2.1
Windows trial, and a supplied video shows the game rendering and responding to
buttons. This establishes gameplay on the surveyed HW 2.1 unit with the earlier
build. The video does not establish a full stock restore or test the updated
v0.1.1 binary. The HW 2.1 release remains experimental.

## v0.1.1 display correction on HW2.3

On 2026-11-10 the project owner reported flashing the corrected firmware on
their HW2.3 KM003C and confirmed that the gap above the status panel was gone
and the game ran correctly. The tested `.mencrypt` has SHA-256
`249ca43a5ec5f5f1bde0634368b87f4c7de77c4935adfa1d9f64e5503255e3dd`, matching the
firmware bundled in both Windows ZIPs for [v0.1.1-independent](https://github.com/inuchanbt/apdo-doom-on-km003c/releases/tag/v0.1.1-independent).

This is an owner-reported application and gameplay check on one HW2.3 unit.
The full v0.1.1 Windows installer and stock restoration workflows still await
end-to-end hardware verification.

## Previously recorded engineering checks

- Independent ARM firmware and Windows host compile; Clippy checks passed.
- Offline/model/HTTP tests cover the licensed clock/interrupt/timer subset,
  binary wire bounds, stock/DFU backup validation, image packaging, storage
  planning/round-trip restoration, source preparation and controller reconnects.
- Real Freedoom engine checks cover square projection parity, cached reads,
  movement/turn/fire/menu, full HUD redraw and input cadence after slow draws.
- The startup banner's archive/asset hashes and captured device pixels matched
  the expected original Freedoom render; the banner build auto-started DOOM.
- On the same HW2.3 device, independent application flashing and complete
  readback, unchanged loader, native DFU return and verified restart were checked.
  PC input changed captured views and subsequently released; preview is 120×120.
- The published `.mencrypt` was rebuilt byte-for-byte against that device-tested
  package and audited against all 225,256 installed application bytes.
- The explicit release source archive built the Windows host, ARM application
  and standalone backup utility offline from bundled sources with a fresh Cargo
  home and no device backup. The eight uploaded assets were downloaded and
  checked against the verified local files. See the release manifest/checksums.

For implementation details and original test scope see [INDEPENDENT.md](INDEPENDENT.md).
For repeatable software checks see [BUILD.md](BUILD.md).

## Main integration checks on 2026-10-31

The merged application, native engine, protocol, backup utility and host sources
match the published independent branch; this integration changes documentation,
removes legacy-only files and fixes the browser preview layout. The 48 software
tests passed (20 hardware models, 3 packaging/storage, 2 wire bounds, 9 backup,
12 controller/HTTP and 2 source-preparation tests), along with Rust formatting
and the pinned engine-source check. No device was flashed for this integration.

Browser inspection confirmed a square preview at narrow desktop, HD, mobile
and 4K widths, with no horizontal overflow. On the reported 909×1184 viewport,
the frame changed from 413×760 to 413×413. The meter was not connected to the
independent service during this layout check; it verifies the browser layout,
not another hardware gameplay test.

## Branch and release history

`main` now contains the independent implementation. Historical v5–v8 measurements,
checks and the earlier stock/PD restoration record are retained with the code on
[legacy](https://github.com/inuchanbt/apdo-doom-on-km003c/blob/legacy/docs/VALIDATION.md).
Those historical results are not new independent-firmware measurements.

The `v0.1.0-independent` tag and its binary/source/checksum assets are unchanged.
They record the evidence available when published, before the owner's latest
round-trip confirmation. Current documentation and release notes record the
subsequent confirmation without rewriting the original asset evidence.
