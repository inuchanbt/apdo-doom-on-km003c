# Building APDO DOOM

The recorded build used Windows, PowerShell, Python 3.12.14, Rust 1.99.1 (Windows GNU), and LLVM-MinGW's 20261020 UCRT x86_64 package with Clang 23.1.4. Other environments have not been verified.

## 1. Reconstruct the source

```powershell
python scripts/prepare-sources.py --sources-only
```

The script fetches the exact revisions in `SOURCE_LOCK.json`, applies `patches/`, copies added Rust files from `overlays/`, and prepares the pinned doomgeneric source used by the converter. It refuses a different HEAD or conflicting edits. It does not reset an existing checkout or access USB.

To verify an existing prepared source tree without changing files:

```powershell
python scripts/prepare-sources.py --check --sources-only
```

Keep local changes in the versioned overlays and patches as well as the working `vendor/` checkout. `vendor/` is a build workspace, not the source of record for this repository.

## 2. Configure the toolchains

Install Git, Python 3.10 or newer, and [Rustup](https://rustup.rs/). Install the Windows GNU Rust toolchain and ARM target:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu --component rustfmt --component clippy
rustup target add --toolchain stable-x86_64-pc-windows-gnu thumbv7em-none-eabihf
```

Extract [LLVM-MinGW](https://github.com/mstorsjo/llvm-mingw/releases) to `tools/llvm-mingw-20261020-ucrt-x86_64/`, or set `APDO_LLVM_BIN` to its `bin` directory. The build needs Clang, `llvm-ar`, `llvm-objcopy`, `llvm-nm`, and `llvm-dlltool`. The converter patch uses Clang's evaluated AST when Windows LLVM-MinGW does not provide `llvm-dwarfdump`.

```powershell
. .\scripts\env.ps1
```

This changes the current shell environment only. If `tools/cargo/` and `tools/rustup/` exist, it uses those local installations; otherwise it uses the configured Rustup/Cargo homes. The GNU host linker is Rust's bundled GCC, and import libraries are handled by LLVM's dlltool.

## 3. Prepare private recovery material

This step assumes a previous, verified acquisition from the target meter. This repository does not automate first-time recovery qualification for an unknown unit.

```powershell
New-Item -ItemType Directory -Force local
Copy-Item local.example.json local/device.json
```

Edit `local/device.json` with the verified device serial, stock and external-backup paths, a generated DOOM package path, and the Python executable if needed. Paths are relative to the repository unless absolute. This file and `local/` are ignored by Git.

The host loader allowlist needs `loader-00000004-00003fff.bin`, its `.sha256`, and its `.verified.json` recording two matching reads, in the configured stock backup directory. Full recovery also needs the stock application/package, full original external NOR backup, and verification evidence described in [RECOVERY.md](RECOVERY.md).

```powershell
python scripts/prepare-sources.py
```

The script verifies and copies the private loader baseline into the ignored board checkout. It never distributes that binary. `env.ps1` passes the configured serial as `APDO_DEVICE_SERIAL`; the stock-to-loader path refuses to proceed if no verified serial was configured at build time. The exact loader-byte allowlist and HW2.3 checks remain in place.

## 4. Generate Freedoom data

From the repository root, load the environment and change to the converter directory:

```powershell
. .\scripts\env.ps1
$env:HOSTCC = 'clang'
Push-Location vendor/dmc100-doom
python -I tools/conv/__main__.py --packs E1M1 all
Pop-Location
```

The converter downloads the official Freedoom 0.13.0 release into its ignored `dl/` cache and verifies its pinned SHA-256. To use a cached release explicitly, add `--zip dl/freedoom-0.13.0.zip`. It generates `gen/` and `build/packs/E1M1/DOOM.DAT`. The tested pack is 2,802,000 bytes; its SHA-256 is recorded in `SOURCE_LOCK.json`.

## 5. Build

```powershell
.\scripts\build-host.ps1
.\scripts\build-firmware.ps1 -Python python -Name km003c-doom
```

The firmware script compiles the C engine, links the `doom` Rust feature for `thumbv7em-none-eabihf`, converts the ELF to a raw application image, and validates/packages it as `artifacts/km003c-doom.mencrypt`. It refuses to overwrite an output with the same name. A default timestamped name can also be used. **Neither build command flashes USB hardware.**

The application begins at `0x4000`. The DOOM feature starts USB and the game task while leaving measurement/PD tasks inactive. The engine reads the first 3 MiB of external NOR; the final 1 MiB stays protected from game-service writes.

## Optional checks

Use the source-preparation check above, then the upstream Rust checks:

```powershell
. .\scripts\env.ps1
Push-Location vendor/km003c-firmware
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
Push-Location firmware
cargo fmt -- --check
cargo clippy --release --locked --features doom -- -D warnings
Pop-Location
Pop-Location
```

To exercise the C engine without USB:

```powershell
python scripts/build-engine.py --host-test
python scripts/test-engine.py
```

The optional `scripts/render-frame.py` converts a test frame to PNG and requires Pillow. The saved `docs/images/doom-host-preview.png` comes from this host harness. The README now shows the Web controller with a live device capture composited into the supplied enclosure image.
