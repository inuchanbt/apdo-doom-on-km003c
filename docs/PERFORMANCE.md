# Performance — independent firmware

The independent version retains square projection, the short-read cache, SRAMH
engine arena and Steady pacing from the earlier work. It uses a 240×240 square
layout with a 208-row view and a 32-row status panel. The renderer still computes
200 rows, which are scaled to fill the view. Messages overlay the bottom eight
rows of the view, and the horizontal field of view is preserved.
Steady targets four 35 Hz tics (114.286 ms, 8.75 fps) between frames. This is a
target, not a minimum guaranteed rate: scene cost and USB preview can exceed it.

LCD columns and external NOR reads share one QSPI controller under the game task.
The application uses synchronous transfers, not a full RGB565 double buffer or
an asynchronous DMA pipeline. Indexed strips/caches reduce RAM use. Game assets
are not LZ4 compressed. Faster NOR reads are enabled only after a full comparison
with the original read speed at startup; `-SafeNor` keeps the original speed.

The Web **Frame pacing** selector chooses Steady or Uncapped. Disable **Live
preview** when evaluating motion on the physical screen. The preview's update
rate is distinct from the game render rate. The independent host does not
implement the old v8 `game-profile` CLI.

The detailed v8 DWT samples, medians, build comparisons and frame-interval study
remain on [legacy](https://github.com/inuchanbt/apdo-doom-on-km003c/blob/legacy/docs/PERFORMANCE.md),
alongside its profiling tools and `performance-v8.json`. They explain the earlier
optimization decisions but are **not measurements of the independent board
implementation**. No new independent-firmware throughput benchmark is claimed.

## HW2.3 internal flash optimization (2.0.7-apdo5)

The follow-up `2.0.7-apdo5` retains these optimizations and changes
the in-game loader gesture to holding all four physical buttons for two seconds.
Exit remains available for sustained fire. The manual recovery gesture remains
holding Exit while connecting the HID cable. The owner confirmed sustained
fire and the four-button loader gesture on one HW2.3 unit.

The game reads 32 frequently used 4 KiB E1M1 pages from MCU application flash.
The build extracts them from the SHA-256-pinned pack using the page list in
`SOURCE_LOCK.json`. At game initialization the engine compares every valid byte
of these pages with the installed NOR. Any mismatch disables the whole local
copy. Cross-page reads and reads of other pages use the existing NOR path.
The extra 128 KiB occupies application flash. The 240×240 display, RGB565
output, HUD, 35 Hz simulation and pacing options use the existing code.

In a host replay of 140 frames (stationary, forward, turn, fire, stationary),
the observer counted reads beneath the engine caches after initialization:

| External NOR work | Previous path | With pinned pages |
| --- | ---: | ---: |
| Read transactions | 146,128 | 30,525 |
| Payload bytes | 6,287,977 | 1,327,668 |

All 140 RGB565 frame hashes matched. Page selection used this same replay,
ranking pages by requested bytes plus four command/address bytes per request
fragment. Results in other areas can differ. MCU frame time and on-device
gameplay still need a physical comparison. For that comparison, use the same
scene with **Uncapped** pacing and **Live preview** off on both builds.

```powershell
python scripts/build-engine.py --host-test --square --no-pinned-pages
python scripts/measure-engine-reads.py artifacts/before.json
python scripts/build-engine.py --host-test --square
python scripts/measure-engine-reads.py artifacts/after.json --reference artifacts/before.json
python scripts/measure-engine-reads.py --check-fallback
```

Run [the engine and input tests](BUILD.md#checks) for projection, caching, HUD
and command-cadence regression checks. See [VALIDATION.md](VALIDATION.md) for the
actual hardware scope and [PC-CONTROLS.md](PC-CONTROLS.md) for the controls.
