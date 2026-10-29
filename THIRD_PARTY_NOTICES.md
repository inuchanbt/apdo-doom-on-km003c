# Third-party notices and license scope

The repository's existing LICENSE is MIT, copyright 2026 @USB_PD_EPR_240W. It applies to original project documentation, orchestration scripts, and original Rust additions where no other license is stated. It does not relicense upstream material or GPL-covered engine integration.

The low-memory DOOM engine and converter are based on Artur Bieniek's [DMC100-Doom](https://github.com/ArturBieniek4/DMC100-Doom), incorporating the id Software DOOM engine and doomgeneric. Preserve their source notices. The files in native/ and the DMC100-Doom converter patch are GPL-2.0-or-later; see COPYING and the upstream notices. Revisions and local changes are recorded in SOURCE_LOCK.json, patches/, overlays/, and native/.

The board firmware and host transport are based on [okhsunrog/km003c-firmware](https://github.com/okhsunrog/km003c-firmware) and its pinned km003c-lib dependency. The pinned board repository has no top-level license file. Patches do not grant an additional license for that upstream source. Consult its individual file notices and dependency licenses before redistributing bundled sources or firmware binaries. The repository publishes source additions and patches, not a copy of the upstream firmware or proprietary recovery images.

Game assets are [Freedoom 0.13.0](https://freedoom.github.io/), distributed under its modified BSD license. The README's Web-controller screenshot, the host-rendered preview, and the device-capture image in docs use those assets. The release's COPYING and credits are retained in notices/freedoom-0.13.0/. Keep those notices with redistributed game assets. No commercial DOOM IWAD is included.

Hardware backups, device identifiers, calibration data, compiled firmware, and converted game packs are excluded from version control. Source reconstruction fetches the pinned upstream repositories; their notices remain in those checkouts.

The KM003C enclosure artwork at web/assets/km003c-shell.webp was supplied by the project owner for the controller presentation. The original image is preserved; rotation and live-image compositing are performed in CSS. Original image rights remain with their respective owner; the project's MIT license does not grant additional rights to this supplied artwork.
