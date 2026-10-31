# Independent branch license scope

The active build on `codex/independent-firmware` is described in
[INDEPENDENT.md](docs/INDEPENDENT.md). It does not include the unlicensed board
firmware or generated PAC. The MIT clock/interrupt/timer subset retains its
[upstream license](independent/hardware/LICENSE-MIT). New board/wire/host code is
MIT unless marked otherwise. Linking the GPL engine creates a combined firmware
subject to the applicable GPL terms; this is not an all-MIT firmware.

The following notices describe both retained engine/media assets and historical
v8 inputs. References to the old board host's embedded recovery data apply to
the legacy build, not `independent/host`, which loads qualified data at runtime.
No release completeness claim is made before the independent release audit.

# Third-party notices and license scope

The repository's existing LICENSE is MIT, copyright 2026 @USB_PD_EPR_240W. It applies to original project documentation, orchestration scripts, and original Rust additions where no other license is stated. It does not relicense upstream material or GPL-covered engine integration.

The independent `bootstrap/` backup tool is an original MIT-licensed Rust addition. Its pinned `km003c-lib` transport dependency and other Cargo dependencies retain their own licenses. Building it does not include or redistribute a device-private loader baseline.

The low-memory DOOM engine and converter are based on Artur Bieniek's [DMC100-Doom](https://github.com/ArturBieniek4/DMC100-Doom), incorporating the id Software DOOM engine and doomgeneric. Preserve their source notices. The files in native/ and the DMC100-Doom converter patch are GPL-2.0-or-later; see COPYING and the upstream notices. Revisions and local changes are recorded in SOURCE_LOCK.json, patches/, overlays/, and native/.

The board firmware and host transport are based on [okhsunrog/km003c-firmware](https://github.com/okhsunrog/km003c-firmware) and its pinned km003c-lib dependency. The pinned board repository has no top-level license file. Patches do not grant an additional license for that upstream source. Consult its individual file notices and dependency licenses before redistributing bundled sources or firmware binaries. The repository publishes source additions and patches, not a copy of the upstream firmware or proprietary recovery images.

Game assets are [Freedoom 0.13.0](https://freedoom.github.io/), distributed under its modified BSD license. The README's Web-controller screenshot, the host-rendered preview, and the device-capture image, and the hardware demonstration video in docs use those assets. The release's COPYING and credits are retained in notices/freedoom-0.13.0/. Keep those notices with redistributed game assets. No commercial DOOM IWAD is included.

Hardware backups, device identifiers, calibration data, compiled firmware, and converted game packs are excluded from version control. Source reconstruction fetches the pinned upstream repositories; their notices remain in those checkouts.

The KM003C enclosure artwork at web/assets/km003c-shell.webp was supplied by the project owner for the controller presentation. The original image is preserved; rotation and live-image compositing are performed in CSS. Original image rights remain with their respective owner; the project's MIT license does not grant additional rights to this supplied artwork.

The hardware demonstration video in `docs/videos/` was supplied by the project owner. Rights in the original recording are retained; the code license does not grant an additional license to the recording.


## Prebuilt firmware release review (2026-10-31)

No prebuilt application or local board-host executable is currently offered.
Redistribution permission for the pinned board firmware has not been obtained.
Its source and binary terms must be clarified in a way compatible with the
linked GPL-2.0-or-later engine before publishing a combined firmware package and
its corresponding source. The local board-host executable also embeds the private
loader baseline and can contain the configured serial; it is not a public
release asset. See [binary-release requirements](docs/BINARY-RELEASE.md).

The repository's own MIT license and the README's hardware-risk notice do not
grant additional rights in upstream code. A future release must retain required
license notices and supply the corresponding source/build material; the automatic
GitHub source archive does not include ignored upstream checkouts.

The startup banner in `independent/firmware/assets/freedoom-banner.rgb565` is the
original `M_DOOM` menu logo extracted from Freedoom 0.13.0 Phase 1, flattened onto
black and converted to RGB565. It retains Freedoom's modified BSD license, not
the surrounding MIT code license. Required COPYING/CREDITS are retained under
`notices/freedoom-0.13.0/`. The adjacent JSON records exact archive/lump/asset hashes;
`scripts/extract-splash.py` reproduces the pixels from the verified official ZIP.
