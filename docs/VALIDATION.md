# Validation record

Recorded on 2026-10-29 (Japan time) for one POWER-Z KM003C HW2.3 unit, originally running firmware 2.0.7. Device identifiers and raw acquisition files remain private.

## Device checks

- Read the original application, loader, calibration, internal tail, and external NOR twice and verified agreement.
- Confirmed physical Exit-at-power-on entry to native DFU and application-to-DFU recovery.
- Verified full application readback and unchanged loader after updates.
- Uploaded the 2,802,000-byte Freedoom E1M1 pack and verified its complete readback.
- Compared the protected final 1 MiB of external NOR before and after game-data installation.
- Observed approximately 171–173 ms per frame, about 6 fps.
- The user confirmed the 3D game display and movement, turning, and firing on the device.
- The user unplugged/replugged the HID cable and confirmed automatic startup and working controls on final firmware v4.
- Restored the original external NOR, verified all 4 MiB against the original backup, restored the stock application, verified the unchanged loader, and confirmed stock 2.0.7 startup.
- Reinstalled and verified the final DOOM application and game data after the recovery test. The working game remains installed.

Local evidence includes `artifacts/final-verification.json`, `flash-final/`, `game-data-final/`, timestamped `restore-nor-*/` and `restore-stock-*/` directories, and `backups/`. These files are excluded from Git because they include device-specific material.

## Build and host checks

- The C host harness exercised 140 game frames with forward movement, turning, and firing, followed by menu input. Captured frames had changing RGB565 output.
- Host and firmware Clippy checks passed with `-D warnings`.
- The Rust workspace's 33 existing tests passed.
- Rebuilding with the saved firmware script produced a byte-identical raw application.

The README image is a host render. Host rendering and compilation checks supplement the device checks; they are not evidence of hardware operation by themselves.

The public repository subsequently parameterizes local recovery paths and the build-time serial and adds an explicit `game-backup --stock-prefix` argument. These host/build changes do not alter the installed v4 game image. Publication checks and the exact pushed revision are available in Git history.
