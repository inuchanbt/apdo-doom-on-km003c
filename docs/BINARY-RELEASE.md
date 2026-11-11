# Independent experimental binary releases

[v0.1.2-independent](https://github.com/inuchanbt/apdo-doom-on-km003c/releases/tag/v0.1.2-independent)
adds generic HID communication, the matching Windows host and an HW2.3 installer
for stock FW 2.0.7. Firmware metadata is `2.0.7-apdo3`. The exact previously built
firmware was flashed and reported working by the owner on 2026-11-11. The new
transport has not had a newly verified full initial-install/stock-restore round
trip or performance benchmark. See [USB identity and migration](USB-IDENTITY.md).
The package includes firmware, game data, notices, complete corresponding source
and checksums. The risks and backup requirements below continue to apply.

The v0.1.2 release is HW2.3 only. Do not use its installer for HW2.1.
The Windows kit includes `Remove-Leftover-DOOM-Data.cmd` / menu item 5 for
restoring original NOR after the stock app was already restored. It requires
the original qualified backups and temporarily rewrites firmware; this added
workflow has software checks but no new hardware round-trip confirmation.
See [the cleanup guide](REMOVE-DOOM-DATA.md).

The [v0.1.1 Windows installers](https://github.com/inuchanbt/apdo-doom-on-km003c/releases/tag/v0.1.1-independent)
include the corrected display, the application and the Freedoom E1M1 game data.
Choose HW 2.3 or the experimental HW 2.1 package for stock FW 2.0.7. Start with
`START-HERE.md` in the ZIP. See [hardware validation](VALIDATION.md#hw21-gameplay-report)
for the scope of the earlier gameplay report.

## Historical application-only release

Release: [v0.1.0-independent](https://github.com/inuchanbt/apdo-doom-on-km003c/releases/tag/v0.1.0-independent).
Firmware metadata: `KM003C / 2.0.7-apdo2`. This is a **Pre-release**, not a stable,
manufacturer-approved or universally compatible update.

**HW2.1 failure report:** A [tester with a September 2023 HW2.1 unit](https://x.com/tomoca2/status/2116438041766375717)
reported an unresponsive Freedoom startup splash after installing the release's
HW2.3 `.mencrypt` with Mtools 2.5.2. A follow-up supplied on 2026-11-07 shows an
APDO DOOM entry and `LIBUSB_ERROR_NO_DEVICE` at 0% in the stock updater, followed
by reported error-free recovery through forced DFU. The reporter installed the
application alone, so the splash matches the expected wait for game data.
The v0.1.0 application-only release targeted HW2.3. See
[the compatibility record](VALIDATION.md#external-hw21-failure-report).

## Assets and provenance

- An application-only `.mencrypt`, linked at `0x4000`, with the Freedoom startup banner.
- The exact Freedoom 0.13.0 E1M1 pack in a separate ZIP (2,802,000 bytes).
- An explicit corresponding-source archive: active project source, patched pinned
  DMC100-Doom, the pinned doomgeneric converter inputs, generated C, the original
  verified Freedoom archive, Cargo dependencies by origin, and matching Rust library
  sources/notices. General build tools are installed separately.
- Upstream/dependency notices, `INSTALL-PREBUILT.md`, `RELEASE-MANIFEST.json` and
  `SHA256SUMS.txt` accompany the downloads. Use the attached source archive, not
  GitHub's automatic repository-only source ZIP.

The application bytes were rebuilt and compared with the existing device-verified
build. The pinned library validates the update container and reproduces every byte
of the installed readback. The native loader remained unchanged. No stock app,
loader/ICG dump, settings, calibration, private backup or device identifier is
included in the public assets. No host executable is included in this release.

## Tested scope and unverified paths

One HW2.3 unit originally running stock 2.0.7 was tested for flashing/full app
readback, unchanged loader, native DFU return/restart, rendering, PC input and
preview. The startup logo preview was compared pixel-for-pixel with its source.
Earlier stock restore/PD checks used the legacy workflow. On **2026-10-31**, after
the release was published, the owner additionally confirmed the independent
workflow from initial installation through complete stock restoration on the
same HW2.3 unit. This is an owner-reported hardware round trip; new private
acquisition/readback logs were not independently inspected for this update.
Additional units/revisions and exhaustive protocol conformance are not qualified.
See [VALIDATION.md](VALIDATION.md).

The release tag and attached files remain unchanged. Their qualification flags
describe the evidence available when the assets were created. This later result
is documented on `main` and in the updated release notes.

The manufacturer-updater installation attempt is documented in the external
HW2.1 report above. The project-tested installation path uses the qualified
host workflow. Full manufacturer-updater and HW2.1 compatibility require
further verification.

## Risks and required preparation

This can permanently disable the meter or destroy factory data; recovery is not
guaranteed and repair/replacement may be necessary. DOOM replaces tester functions:
there is no USB measurement, PD negotiation or stock tester UI while installed.
Game data rewrites the original first 3 MiB of NOR. Your own qualified stock/DFU
backup and stock restore package are required before app flashing; your own full
original 4 MiB NOR backup is required before game writes. Stock app alone is not a
complete rollback. Restore NOR first, then the original app, as documented.

Connect only side HID, leaving both measurement connectors empty. Do not write a
loader/ICG, substitute another unit's backup, bypass a hardware/port/hash check,
or retry an ambiguous program failure. See [INSTALL-PREBUILT.md](INSTALL-PREBUILT.md)
and [FIRST-BACKUP.md](FIRST-BACKUP.md). This project is unofficial and provided
without project warranty; manufacturer approval/support is not claimed.

## License scope

The distributed combined firmware is GPL-2.0-or-later; its upstream components
retain their individual notices. For dual MIT/Apache dependencies this distribution
uses the MIT option; Unicode and 0BSD notices are retained. Freedoom art is BSD;
the converted pack also contains GPL engine strings/tables. Full license texts,
credits and corresponding source are provided next to the binary downloads.
See [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

The independent binary does not contain `okhsunrog/km003c-firmware` or its generated
PAC. Only explicitly licensed HAL parts are retained. Redistribution permission
for the **legacy v8 board-derived binary** is still absent; this release does not
redistribute it or relicense the historical files.
