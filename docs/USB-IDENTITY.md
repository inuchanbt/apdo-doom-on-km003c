# USB identity and migration

Current source builds (`2.0.7-apdo3`, dated 2026-11-11) use a generic HID
interface for the independent DOOM application:

| Descriptor | Exact value |
| --- | --- |
| VID:PID | `16c0:05df` |
| Product | `KM003C DOOM` |
| HID usage page / usage | `ffad` / `0001` |
| Reports | Unnumbered, 64-byte Input and Output; 1 ms interrupt endpoints |

This is Objective Development's **shared generic HID ID**, not an ID assigned
exclusively to APDO. Its [published conditions](https://github.com/obdev/v-usb/blob/master/usbdrv/USB-IDs-for-free.txt)
require a controlled email/domain in the manufacturer string, retention of that
address, complete manufacturer/product string comparisons in the host, and the
OS's standard HID driver. The firmware and host share a maintainer-controlled
manufacturer string, provided in English/US (`0x0409`). It must remain under the maintainer's control while these builds are
in use. [Their FAQ](https://github.com/obdev/v-usb/blob/master/usbdrv/USB-ID-FAQ.txt)
also permits other USB implementations/hardware. This makes no USB-IF
certification or manufacturer-endorsement claim.

The same sharing policy is used by the T3P DOOM project; its product name is
different. The host checks VID/PID **and both complete strings**, then the HID
usage. Missing strings, prefixes and other products sharing the ID are rejected.
Descriptor matching selects a protocol; it is not proof of a unit's authenticity.

## Update both sides

Build `scripts/build-host.ps1` before `scripts/build-firmware.ps1`. Old PC helpers
cannot communicate with the new HID firmware. The Web server/JSON API stays the
same, so the existing browser controls need no changes. Stop the Web controller
before replacing its helper executable; restart it afterwards.

The new helper also recognizes the earlier `5fc9:0063`, `APDO project`,
`APDO DOOM` WinUSB application. That is a **host-only compatibility path** for
existing installations, including the native-loader request. New MCU builds do
not advertise that identity, a WinUSB compatible ID or its interface GUID.

The stock application and native DFU retain their existing identities and
tools. The backup manifest/qualification schema, loader comparisons, per-unit
calibration checks and NOR write boundaries are unchanged. Keep using the
qualified backup and its recorded physical USB port for installation/restoration.
The host requires exactly one matching APDO KM003C USB device and HID collection;
it refuses ambiguous multi-device selection rather than guessing which HID path
belongs to a backup's port. The loader request now retains the port selected
when the client opened, rather than selecting it again before the request.

## Framing and verification status

The APDO request and response header retain their version/opcode/sequence and
1024-byte data limit. A HID request carries the existing 64 bytes. A response's
first report contains its 16-byte header plus up to 48 data bytes; following
reports contain up to 64 data bytes each. All reports are padded to 64 bytes;
padding is discarded by the host. HIDAPI's leading zero report-ID byte is used
only in host writes, not in the device's unnumbered report descriptor. There are
no bulk zero-length termination packets on this transport.

Offline tests cover exact identity matching, every payload size from 0 through
1024, report boundaries, stale sequences, wrong opcodes and interrupted replies.
On 2026-11-11 the owner reported flashing the new firmware to the KM003C and
confirmed that it operated without problems. The v0.1.2 experimental release
contains that exact firmware and its matching Windows host. This is an
owner-reported flash/operation check, not a newly inspected full first-install,
NOR-programming and stock-restoration round trip. Preview performance has not
been newly benchmarked. Existing v0.1.0/v0.1.1 assets remain historical WinUSB
builds and are not replaced.

The separate HW2.1 trial host still permits only its pinned trial/stock images;
the new HID firmware is not added to that allowlist by this change.

Local checks on 2026-11-11 passed 51 tests (20 hardware, 6 host/storage,
4 protocol, 9 backup/recovery and 12 controller HTTP tests), host/protocol/ARM
Clippy, formatting, ARM release linking, a Windows x64 release host build and
the HW2.1 host compile check. The offline packager validated a 225,288-byte
application and produced a 225,512-byte update container. The isolated local
files are under `artifacts/usb-hid-validation/`. Their original validation record
describes the pre-hardware-check state; the v0.1.2 release record separately
records the owner's later flash/operation confirmation.
