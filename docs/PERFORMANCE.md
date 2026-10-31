# Performance — independent firmware

The independent version retains square projection, the short-read cache, SRAMH
engine arena and Steady pacing from the earlier work. It uses a 240×240 square
layout with a complete 40-row HUD and preserves the horizontal field of view.
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

Run [the engine and input tests](BUILD.md#checks) for projection, caching, HUD
and command-cadence regression checks. See [VALIDATION.md](VALIDATION.md) for the
actual hardware scope and [PC-CONTROLS.md](PC-CONTROLS.md) for the controls.
