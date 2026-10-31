# Validation — independent firmware

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

- The application froze at the title screen.
- Attempts to restore the original firmware also produced an error.
- The reporter successfully recovered the unit using **forced DFU mode**.

This entry records the reporter's account supplied by the project owner. It has
not been independently reproduced on project hardware. The exact firmware build,
stock firmware version, title-screen stage, error output and root cause were
not supplied in the quoted report. No specific DFU entry sequence is established
by that account, so it is not a new recovery procedure or a universal workaround.

**HW2.1 remains unsupported.** Do not bypass hardware validation to try it.
Project qualification remains limited to the same HW2.3 / stock 2.0.7 unit;
recovery reported on this HW2.1 unit does not establish recoverability of other
units or failure modes. No HW2.1 fix or additional compatibility claim is made.

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
