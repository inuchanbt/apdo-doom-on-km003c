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

## PC controls (v5)

The subsequent v5 update adds a separate leased-input endpoint and browser controller. Its whole application was read back and the loader was unchanged. A 300 ms forward command was accepted by the device and a later read-only input status showed all remote/applied keys released. The user confirmed movement, turning, and firing from the browser keyboard. The game engine and NOR game resources are unchanged. See [PC-CONTROLS.md](PC-CONTROLS.md) for input handling and focused test coverage.

## Screen preview (v6)

The v6 application was read back completely and its loader was unchanged. An actual 120×90 RGB565 snapshot from the device was retrieved, decoded, and displayed in the browser. The user confirmed live PC display and keyboard actions reflected in that image. Pausing image transfers kept gameplay and input active. The Rust suite passed 40 tests and the Python suite passed eight tests, including snapshot consistency, clipping, image bounds, byte order, orientation, padding, and HTTP handling. Raw BMP captures and transport timings are kept locally in `artifacts/`. See [SCREEN-CAPTURE.md](SCREEN-CAPTURE.md) for the capture path and performance limitations.

## Physical LCD controls (v7)

The v7 application was fully read back and the native loader was unchanged. The user confirmed physical brightness changes, left/right rotation, and correct display including the HUD. The Rust suite passed 41 tests and the HTTP/BMP suite passed nine, including display value bounds and same-origin enforcement. Rotation uses the board driver's streamed quarter-turn transform; it clears the physical LCD and invalidates the engine's status-bar redraw state. A host engine check clears its simulated LCD and verifies that a redraw restores all status-bar pixels without advancing game time.

## USB late connection and reconnection

The server was started with the HID cable unplugged and continued retrying beyond five failed helper sessions. After the cable was connected, live screen capture and keyboard control resumed; the user confirmed both on the same server. No firmware update was needed. Eleven Python tests pass, including helper failure/restart, clearing held keys across connections, rejecting stale helper output, preserving preview preference, accepting restarted device snapshot counters, and cancelling retries on Stop.

## Responsive Web controller

The preview and controls use two columns from 900 CSS pixels wide; narrower windows stack them. The layout was inspected at 1280×600, 1280×720, 1366×668, 1920×1080, 3840×2160, 900×600, and 390×844. Desktop layouts fit within the viewport with the footer visible; phone width uses vertical scrolling without horizontal overflow. The short-window check also verified that button text fits. Live capture and keyboard enable/release were checked, and the README screenshot was refreshed at 1280×720.

## Independent first-backup workflow

The new `bootstrap/` tool builds separately from the board workspace and uses no private firmware include. A cold release build also passed from a separate copy containing only the bootstrap crate and its environment/build scripts, with no `local/`, `backups/`, or `vendor/`, no configured serial, and a separate target directory (cached public Cargo dependencies, offline). Bootstrap acquisition/verification tests (9) and Python baseline-import tests (7) passed using synthetic files and no USB. The board host workspace tests passed (42, including the new installed-backup packaging test), and both host workspaces passed Clippy with warnings denied. The saved original stock application was packaged offline with exact installed-byte roundtrip validation; reusing the output path was rejected.

The initial software-only checks on 2026-10-29 did not constitute an end-to-end stock-device acquisition. No firmware was flashed during those initial checks. The following section records the subsequent hardware verification. See [FIRST-BACKUP.md](FIRST-BACKUP.md) for the staged stock/DFU/full-NOR procedure.

## Bootstrap hardware check (2026-10-30)

The independent tool was subsequently tested on the original HW2.3 unit after restoring its stock 2.0.7 application and all 4 MiB of original external NOR. Full application/NOR readback and unchanged loader were verified. The owner reported normal operation of the PD functions they checked after restoration; other charging protocols and exhaustive PD conformance were not tested.

- Stock acquisition read all five regions twice, checked the committed application CRC, identities, lengths and SHA-256 records, and passed offline verification. Every region also matched the earlier original backup byte for byte.
- A restore package made from the new application backup was byte-identical to the previously verified stock restore package.
- The tool rejected the custom RPC application as a stock source, rejected DFU qualification while still in the stock application, and refused a fresh baseline import without DFU evidence.
- The owner manually entered native DFU with Exit held on the same USB port. The tool read the loader and complete stock application twice; both matched the stock acquisition. Offline verification with `--require-dfu` passed.
- The real source-preparation importer accepted the qualified acquisition in a separate private workspace with no existing loader baseline, copied the exact loader, and passed its subsequent check.
- The exact previously installed DOOM v7 image was reinstalled, with full application readback and unchanged loader verified.

After the owner's stock PD checks, the NOR prefix differed from the earlier acquisition by 114 bytes in two sectors, and its protected tail had also changed. The full-NOR acquisition correctly refused the stale stock-prefix reference before any game-resource write. A fresh stock acquisition independently confirmed the changed prefix. A new stock/DFU set and matching two-read full-NOR backup were then acquired, copied into private `backups/`, and selected together in the local configuration. The original backups and previous configuration were retained. No verification flags were overridden.

The same Freedoom E1M1 pack was subsequently restored and read back, with the current protected tail unchanged. The updated host completed automatic stock/RPC-to-DFU transitions and a final live application/loader verification. DOOM auto-started after the final restart, and the controller again captured a live 120×90 game frame at its existing local URL.

During preparation, USB handoff briefly preceded interface availability and stopped before programming. The host now waits for read-only loader identity probes to succeed on the selected physical port, with an eight-second deadline. It never retries a flash data transfer. Three focused readiness tests passed (10 host-library tests total), as did workspace Clippy with warnings denied.

The offline recovery checker now supports complete bootstrap snapshot sets and an explicit configuration path for historical evidence. Seven synthetic tests cover prefix drift, corruption/truncation, protected-tail hashes, read evidence and incomplete restoration pairs. Both the new snapshot set and a complete historical restoration record passed checks. Snapshot consistency is kept distinct from proof of a completed hardware restoration.

The owner confirmed physical game controls after restoring DOOM. Their normal-browser tab still pointed to an old, stopped server port and reported `Failed to fetch`; navigating it to the active server restored the connection. Idle-tab zero-input heartbeats were also removed so an inactive page cannot repeatedly clear another page's controls. With the in-app page and Opera both open, Opera keyboard input produced accepted and applied forward, right-turn and fire masks on the meter, followed by zero remote/applied keys. A screenshot and input observations are kept with the private run evidence.

Raw acquisitions, hashes, restore packages, logs and comparison records remain in ignored `artifacts/bootstrap-device-check-20261030-*/`. This validates one restored unit, not a second untouched meter or every firmware/hardware revision.
