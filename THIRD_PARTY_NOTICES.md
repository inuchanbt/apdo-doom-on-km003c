# Third-party notices and license scope — independent release

The combined independent MCU application is distributed under
**GPL-2.0-or-later**; [COPYING](COPYING) supplies that text. This is not an all-MIT
firmware. Original project scripts, documentation, new Rust board/wire/host code
and the independent backup utility are MIT where no other notice is stated;
[LICENSE](LICENSE) retains copyright 2026 @USB_PD_EPR_240W. Each upstream component
retains its copyright/license notices.

## DOOM engine and generated game data

Artur Bieniek's [DMC100-Doom](https://github.com/ArturBieniek4/DMC100-Doom) is pinned
in `SOURCE_LOCK.json`, incorporating id Software DOOM, Chocolate Doom, Raven and
PrBoom-plus contributions and doomgeneric. Engine/converter/native integration
and local DMC changes are GPL-2.0-or-later. Preserve upstream `COPYING`, `LICENSES/`
and `CREDITS.md`, including their detailed file/copyright list. The complete source
bundle includes the patched DMC source, generated C and the pinned unmodified
doomgeneric files used by its converter. Upstream MIT/public-domain material
retains the notices recorded in DMC's credits; unused upstream board drivers are
not linked into the independent KM003C application.

[Freedoom 0.13.0](https://freedoom.github.io/) art/game assets are modified BSD
(BSD-3-Clause), Copyright (c) 2001–2024 Contributors to the Freedoom project.
`notices/freedoom-0.13.0/COPYING.txt` and `CREDITS.txt` accompany the firmware/game
data. **The converted DOOM.DAT also includes GPL engine strings/tables**; it is
not described as entirely BSD. The release supplies corresponding conversion
source and exact inputs alongside it. No commercial DOOM IWAD is included.

The startup `assets/freedoom-banner.rgb565` is the original Phase 1 `M_DOOM` menu
logo, flattened onto black and converted to RGB565. It retains Freedoom's BSD
license, not the surrounding script's MIT license. Its adjacent JSON records
archive/lump/asset hashes; `scripts/extract-splash.py` reproduces it from the
verified official release. The name/contributors must not be used to imply
endorsement of this project.

## HC32 support

`independent/hardware` contains an explicitly MIT-licensed clock/interrupt/timer
subset of `okhsunrog/hc32f460-embassy-rs`; its `LICENSE-MIT`, pinned origin/file
hashes and modifications are retained. Register facts also refer to the vendor's
BSD-3-Clause CMSIS header; that notice is under `notices/hc32/`. No SVD file,
generated `hc32f460-pac`, or unlicensed `okhsunrog/km003c-firmware` board crate is
included in the independent application/source bundle. This is not a clean-room
claim; earlier hardware investigation informed the new board code.

## Rust dependencies and runtime

Cargo locks pin the exact crates. The release's notices/source archives include
upstream license/copyright notices, dependency inventory and source code. The
firmware dependency graph contains GPL project code, MIT/dual MIT–Apache crates,
0BSD and Unicode-3.0 material. Where MIT is an available alternative, this
combined distribution selects MIT; upstream dual-license notices remain intact.
The matching Rust library source and `COPYRIGHT-library.html`/license notices are
also provided. General Rust/LLVM compiler executables are installed separately.

The host/backup utilities use separately licensed, pinned `km003c-lib` under its
MIT/Apache terms. It is not linked into the MCU application. The new host reads
qualified private recovery files at runtime; no loader dump/serial is embedded.
No prebuilt host executable is part of this release.

## Supplied media and historical code

The enclosure image `web/assets/km003c-shell.webp`, README screenshots and hardware
video were supplied/created for this project. Original recording/image rights
remain with their owners; the source-code MIT license grants no additional rights
in supplied artwork. Screenshots/video also show Freedoom assets with their own
notices. Hardware backups, calibration/device data and proprietary stock/loader
images are never release assets.

Legacy v8 patches/overlays, tools and documentation on the
[legacy branch](https://github.com/inuchanbt/apdo-doom-on-km003c/tree/legacy) retain their prior
provenance. That board repository lacks redistribution permission; the
independent release does not redistribute its binary or relicense those files.
They are excluded from the explicit independent source bundle. See
[release scope](docs/BINARY-RELEASE.md) and [independent provenance](docs/INDEPENDENT.md).
