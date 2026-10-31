# Building the independent application

This branch targets Windows/PowerShell, Rust 1.99.1 Windows GNU, ARM target
`thumbv7em-none-eabihf`, Python 3.10+, Git and LLVM-MinGW (recorded Clang 23.1.4).
Install those tools as described in the archived [toolchain setup](legacy/BUILD.md#2-configure-the-toolchains).
Set `APDO_LLVM_BIN` to the LLVM `bin` directory if it is outside `tools/`.

## Prepare public sources and game data

```powershell
python scripts/prepare-sources.py
. .\scripts\env.ps1
$env:HOSTCC = 'clang'
Push-Location vendor/dmc100-doom
python -I tools/conv/__main__.py --packs E1M1 all
Pop-Location
```

Preparation fetches only pinned DMC100-Doom/doomgeneric sources, applies the DMC
patch and never reads private recovery data. The converter verifies its official
Freedoom 0.13.0 download. The expected 2,802,000-byte E1M1 pack SHA-256 is in
`SOURCE_LOCK.json`. `python scripts/prepare-sources.py --check` checks existing sources.

## Build without a backup or a connected meter

```powershell
.\scripts\build-host.ps1
.\scripts\build-firmware.ps1 -Python python -Name apdo-independent
.\scripts\build-backup.ps1
```

The firmware script builds the square C engine and `independent/firmware`, then
uses the independent host's offline packager. Outputs are under ignored
`artifacts/`. No command above opens USB. No stock application, loader, serial,
calibration data, board-firmware checkout or generated PAC is linked.

The application is confined to `0x4000 .. 0x73ffe`; the 64-byte loader metadata is
at `0x4420`. SRAMH contains the 32 KiB engine arena. The native loader and ICG are
not part of the image. Programming is a separate, qualified operation.

## Checks

```powershell
. .\scripts\env.ps1
cargo test --manifest-path independent/Cargo.toml -p apdo-protocol -p apdo-host
cargo test --manifest-path independent/Cargo.toml -p apdo-hardware --features time
cargo fmt --manifest-path independent/Cargo.toml --all -- --check
python scripts/test-prepare-sources.py
python scripts/build-engine.py --host-test --square
python scripts/test-engine.py
python scripts/test-input-cadence.py
```

The hardware subset's register-model tests check clock transitions, timeout
handling, inherited routes and timer wrap/deadline races. Wire tests cover framing
bounds and the protected NOR tail. Packaging tests validate vectors and exact
restoration of committed application bytes. Engine tests run the real Freedoom
pack, including input cadence and HUD redraw.

## Local controller

After the independent application has been qualified on the meter:

```powershell
python scripts/pc-controller.py --port 50594
```

Open the printed `http://127.0.0.1:50594/` URL in a browser. Keep the server running;
do not open the HTML with `file://`. The helper is `independent/target/release/apdo-host.exe`.
Its JSON interface keeps the existing Web keyboard/display/preview controls.

## Qualified native-loader operations

Keep the meter on the physical USB port recorded by the first-backup tool.
The following operations read `STOCK_BACKUP` at runtime and require successful
`verify_qualified` evidence from the independent backup utility. They do not embed
that evidence in the executable.

```powershell
# Manually enter native DFU (Exit held during HID insertion).
independent/target/release/apdo-host.exe flash STOCK_BACKUP IMAGE.mencrypt NEW_EVIDENCE_DIR
independent/target/release/apdo-host.exe start STOCK_BACKUP IMAGE.mencrypt
```

Flash compares the loader with the qualified baseline before programming, uses
the pinned km003c-lib updater, then checks every application byte and unchanged
loader bytes. It never retries a failed program operation or starts an unverified
application. Use a new evidence directory each time. `package` / `package-installed`
are offline and require input/output paths. The latter reproduces an original
installed application's size/CRC fields exactly.

The complete development sequence, including double acquisition of the original
NOR, protected-tail checks, game installation and ordered stock restoration, is
in [FIRST-BACKUP.md](FIRST-BACKUP.md). Storage programming has model tests; the
independent full install/restore sequence still needs native hardware qualification.

## Startup banner asset

The original Freedoom 0.13.0 Phase 1 `M_DOOM` logo is checked in as a small RGB565
asset, with its BSD notices and provenance; ordinary builds need no additional
image tools. To reproduce or check it using the verified official archive:

```powershell
python scripts/extract-splash.py --zip vendor/dmc100-doom/dl/freedoom-0.13.0.zip --check
```

Omit `--check` to regenerate the asset and metadata. The script uses only Python's
standard library and refuses a different archive hash. No game-pack/NOR rewrite
is needed for this banner; it is part of the application image.
