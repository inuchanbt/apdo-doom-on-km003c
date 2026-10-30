# Independent branch release status

On `codex/independent-firmware`, the active application no longer includes the
unlicensed board firmware or generated PAC. See [INDEPENDENT.md](INDEPENDENT.md)
for the explicit MIT HAL subset, new board code and recorded validation.
This removes those dependencies from the new build; it does not relicense the
legacy files or retroactively clear redistribution of the v8 binary.

No prebuilt firmware is published by this branch work. Before a later public
release, qualify the new NOR installation/restore path on hardware, review the
complete pinned dependency notices, assemble corresponding engine/firmware/build
source, and verify that no private recovery/device data enters any release asset.
The new host reads private recovery data at runtime instead of embedding it.

The following v8 review is retained as historical context.

# Prebuilt firmware: status and requirements

**Status on 2026-10-31: no firmware binary is published.** Preparing a valid
update container and documenting device risks do not establish redistribution
rights for the code it contains.

## Current publication blocker

The board application is derived from
[okhsunrog/km003c-firmware](https://github.com/okhsunrog/km003c-firmware), pinned
to `87b3f787d1c1e2ea6213461e1e1cdfce29fb57e3`. That checkout has no license file
or redistribution grant in its README/Cargo manifests. GitHub currently reports
no detected repository license either. Separate redistribution permission has
not been obtained. The project's MIT license does not license this upstream
code; a hardware-risk disclaimer does not supply that permission.

[GitHub's licensing guidance](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository)
distinguishes the ability to view/fork a public repository from a license to
redistribute its contents. The upstream author needs to clarify the applicable
terms before a binary containing that code is offered here.

The permission must cover **modified source and binary redistribution under
terms compatible with the linked GPL-2.0-or-later engine**. Permission to post a
binary alone would not resolve the corresponding-source requirements. Preserve
all applicable notices and determine the license of the combined distribution;
do not label the complete firmware MIT merely because original project scripts
and additions use MIT. See [third-party notices](../THIRD_PARTY_NOTICES.md).

## What a future firmware download would contain

The proposed initial variant is the tested v8 square application: a 240×200
world plus the complete 40-row HUD, with Steady frame pacing by default.

- A custom application-only `.mencrypt` update container, targeting `0x4000`.
- No stock application, native loader/ICG image, factory calibration, settings,
  device identifier or original external-NOR backup.
- No built-in game pack. The separate, verified Freedoom E1M1 data is still
  needed in external NOR before gameplay can start.

The `.mencrypt` format is validated and has been flashed/read back with this
project's host workflow on the original HW2.3 unit. **Loading it through an
official graphical updater has not been tested.** It must not be advertised as
a verified official-updater or one-click installation path.

The model/version fields in a container are not proof of hardware compatibility.
Only one HW2.3 meter, originally running stock 2.0.7, has been tested. Other
revisions and previously modified units are outside the validated workflow.

## Work needed before publication

1. Record upstream redistribution terms and check them against the complete
   linked application's licensing requirements.
2. Supply complete corresponding source and build material for the distributed
   object code, together with required license/credit notices. GitHub's automatic
   source ZIP does **not** contain the ignored `vendor/` checkouts. A source
   archive must be assembled deliberately; external source links and local
   patches alone should not be presented as that complete archive.
3. Rebuild the chosen variant from the recorded source revision. Validate its
   container and application bounds; bind its SHA-256 to the exact tested
   package and verified application readback. Check for private data and keep
   hardware evidence/device identifiers out of public assets.
4. Include installation/recovery instructions that retain the existing
   per-device checks. The current host embeds a private loader allowlist and
   can include a configured serial: **do not upload that local host executable**.
   A generic prebuilt installer would require a separate implementation and
   verification; it is not supplied by publishing an application file.
5. Publish the eligible assets, source and notices together as an explicitly
   experimental GitHub Release, with the README risks repeated in its notes.
   Keep stock/recovery images private. The source tree continues to exclude
   generated `.mencrypt` files.

## Device preparation still applies to prebuilt images

Start with [FIRST-BACKUP.md](FIRST-BACKUP.md). Before the first custom-application
write, obtain this meter's verified stock/DFU snapshots and stock restore package,
and keep an additional copy. Before writing game data, acquire and verify its
complete original 4 MiB external-NOR backup through the maintenance service.

The game-data installation overwrites the original first 3 MiB. Returning to
stock afterwards requires restoring the original NOR contents **and** stock
application in the documented order. A stock application file alone is not a
complete rollback. Native DFU entry helps only while the loader and USB path
remain usable; neither a backup nor a successful test on one unit guarantees
recovery. See [RECOVERY.md](RECOVERY.md).

Connect only the side HID port during these operations and leave both measurement
connectors empty. The DOOM application does not provide the original measurement
and PD functions. This is an independent experiment, not an official manufacturer
update; warranty/support treatment has not been established.
