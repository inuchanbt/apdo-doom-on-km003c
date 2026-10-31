# Rebuilding the release source archive

Use the explicit `apdo-doom-v0.1.0-independent-source.tar.gz`, not GitHub's automatic
repository-only ZIP. It contains upstream sources and dependency notices, generated
engine C, the verified Freedoom ZIP/game pack and Rust library sources.
`SOURCE-MANIFEST.json` binds its files to the tagged project commit.

General build tools are not bundled: Git, Python 3.10+, Rust 1.99.1 Windows GNU with
`thumbv7em-none-eabihf`, and LLVM-MinGW (tested 20261020 UCRT / Clang 23.1.4).
Install the tools first; the actual Cargo/C builds can then run without registry/git
access. `.cargo/config.toml` uses the bundled dependencies and sets Cargo offline.

```powershell
rustup toolchain install 1.99.1-x86_64-pc-windows-gnu --profile minimal
rustup target add --toolchain 1.99.1-x86_64-pc-windows-gnu thumbv7em-none-eabihf
$env:APDO_RUST_TOOLCHAIN = '1.99.1-x86_64-pc-windows-gnu'
$env:APDO_LLVM_BIN = 'C:\path\to\llvm-mingw\bin'
. .\scripts\env.ps1
.\scripts\build-host.ps1
.\scripts\build-backup.ps1
.\scripts\build-firmware.ps1 -Python python -Name rebuilt-independent
```

Do not run `prepare-sources.py` in this unpacked archive: its upstream sources
are already present, with the local DMC patch applied; no `.git` directories are
needed for the build. The generated C files are under `vendor/dmc100-doom/gen`.
The converter and its exact input WAD/doomgeneric source are also included for
inspection or regeneration. General compiler/toolchain sources are referenced by
the recorded tool versions; the matching Rust runtime library sources and notices
are additionally supplied under `toolchain/`.

No backup/device connection is needed to build. Do not put private recovery data
into the source archive. Compilation path names can change embedded diagnostic
strings, so a rebuild in another directory is not promised to have an identical
hash. Release checksums identify the actual tested download, not every rebuild.

`vendor-release.py` isolates crates.io/git packages in separate directories to
retain their original identities. Git build manifests expand workspace fields and
pin intra-repository dependencies to the recorded revision; original manifests
are preserved alongside them. Upstream source code is unchanged. The selected
MIT alternative and all upstream notices are retained. Build/packaging scripts
are included under `scripts/`.

Maintainer-only release bundling scripts use Python 3.11+ (`tomllib`); the recorded
release used Python 3.12.14. Ordinary firmware/host build scripts do not need it.
