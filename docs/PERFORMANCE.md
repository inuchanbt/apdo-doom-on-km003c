# Rendering performance and square output (v8)

The initial limit was external NOR reading, rather than LCD transfer bandwidth.
The game renders strips and synchronously sends individual RGB565 columns.
There is no full native framebuffer. LCD and NOR share one QSPI controller,
with the game task as its exclusive owner.

## Measurement

Stop the browser controller so it releases USB, then use the updated host and v8:

```powershell
. .\scripts\env.ps1
vendor/km003c-firmware/target/release/km003c-host-cli.exe game-profile --samples 40 |
    Set-Content -Encoding utf8 artifacts/profile.json
python scripts/summarize-profile.py artifacts/profile.json
```

The read-only endpoint snapshots DWT cycle counts at the end of each rendered
frame: total engine step, physical NOR reads, synchronous LCD blits, capture
shadow updates, physical NOR bytes/calls and panel pixels. It also reports the
arena address, world viewport width and qualified NOR speed. The counters do not
include the cooperative delay between frames. LCD time includes the driver,
address commands and pin muxing, rather than measuring wire time alone. Other
time includes engine computation, conversion, cache lookup and instrumentation;
interrupts can contribute to any timed region.

Recorded on 2026-10-30 on the original HW2.3 meter at 192 MHz, stationary at the
initial E1M1 position, with PC preview off. Each run collects 40 distinct completed
frames and discards the first five samples. The figures are medians, not combat
benchmarks. `render_fps` is the reciprocal of median engine-step time, rather
than the USB preview rate or a guarantee of sustained gameplay cadence.

| Configuration | Engine step | NOR | LCD | Render fps |
| --- | ---: | ---: | ---: | ---: |
| Original `-Oz`, ordinary SRAM, no additional cache | 176.232 ms | 124.412 ms | 11.293 ms | 5.674 |
| `-O2`, SRAMH arena, no additional cache | 175.589 ms | 124.556 ms | 11.288 ms | 5.695 |
| Above, plus 8 KiB short-record cache | 168.119 ms | 113.799 ms | 11.268 ms | 5.948 |
| Square, original NOR speed, no added cache | 154.499 ms | 106.867 ms | 14.250 ms | 6.473 |
| Final letterbox, short-record cache + qualified 16 MHz | 117.993 ms | 63.982 ms | 11.276 ms | 8.475 |
| Final square, short-record cache + qualified 16 MHz | 104.228 ms | 53.425 ms | 14.254 ms | 9.594 |

The final square run reduced engine time by about 41%, or 1.69 times the baseline
render throughput. Both final variants reported all 2,802,000 game bytes verified
at 16 MHz. [Machine-readable medians](performance-v8.json) also retain the
rejected cache experiment (`cache`).

The baseline transfers about 89,935 NOR bytes in 2,188 reads per frame. Its
unchanged HUD does not need a complete redraw: 36,000 panel pixels are sent for
the 3D region. Multiplying a whole-frame byte count by fps therefore does not
describe all work performed by this renderer.

## Changes

- The contiguous 32 KiB engine arena moves to SRAMH at `0x1fff8000`, including
  its strip buffers and engine caches. The linker prevents simultaneous use of
  the PD fast-code section. This NOLOAD arena is explicitly cleared at startup.
  The manufacturer specifies [32 KiB of single-cycle RAM at 200 MHz](https://www.xhsc.com.cn/product/1246.html).
- The C build defaults to `-O2`. `-Oz` remains available for comparison. These
  changes together had a small effect in this NOR-bound scene; the measurement
  does not isolate their individual contributions.
- An 8 KiB, four-way data cache (plus about 2 KiB of tags) uses 16-byte lines
  to retain short reads across frames.
  Larger column/flat reads bypass it. Only immutable game reads use this cache;
  the USB storage service reads the actual NOR directly.
- Standard `03` game reads can use HCLK/12 (16 MHz), while other flash commands
  and LCD transfers retain their settings. On **every game startup**, each byte
  of the validated game image is compared at the original 8 MHz and proposed
  16 MHz. A mismatch keeps the original speed. No write-enable/program/erase is
  sent by qualification. This adds several seconds to cold startup and qualifies
  the current boot conditions, not every voltage/temperature or board revision.

A rejected 8 KiB cache with 64-byte lines cached every read: it increased NOR
traffic to 151,040 bytes and engine time to 251.053 ms. Prefetching extra data was
counterproductive. The retained short-record policy was measured separately.

## Square mode

```powershell
.\scripts\build-firmware.ps1 -Square -Name km003c-doom-square
```

Square mode uses a **240×200 world viewport and a complete 240×40 HUD/message
band**. It recomputes the vanilla 90-degree horizontal projection, plane slopes,
weapon scale and wall light scaling for the new width. It retains the horizontal
field of view and exposes more vertical scenery. It does not crop the old
picture or stretch its 3D geometry. Game simulation and map data are unchanged.

The engine still draws doubled columns: 120 logical world columns instead of
160. UI screens retain their original 320-column coordinate system and are
resampled separately, so HUD fields and menu options are retained. The device
and browser preview include the whole square output; preview resolution is
120×120. The default build retains the existing 240×180 letterboxed layout and
120×90 preview. Rebuild the host for square captures.

Build comparisons can select `-EngineOpt Oz`, `-EngineCacheKiB 0`, or `-SafeNor`.
An ordinary-SRAM comparison additionally requires `-EngineArena sram -EngineCacheKiB 0`
without `-Square`, to preserve stack space. Square mode requires the SRAMH arena to leave
space for projection tables and capture buffers. `-SafeNor` skips the speed
qualification and keeps original-speed reads. These switches affect compilation;
building does not flash a device. The Freedoom pack format is unchanged.

## Consistent motion and local input

The default **Steady** mode targets one frame every four game tics
(about 114.286 ms, 8.75 fps). It leaves short frames idle until the next slot;
frames whose rendering exceeds the budget take longer. This smooths cadence
rather than increasing the fastest scene's peak rate. **Uncapped** renders again
as soon as the normal USB scheduling gap ends. The Web panel's **Frame pacing**
selector changes the meter's scheduler, independently of LCD rotation and PC
preview. It resets to Steady on power-up.

A subsequent on-device comparison on 2026-10-31 collected 160 samples per mode
on the same boot, holding right turn with preview off and discarding five samples:

| Mode | Minimum frame interval | Median | 90th percentile | Maximum |
| --- | ---: | ---: | ---: | ---: |
| Steady | 114.288 ms | 114.288 ms | 114.289 ms | 117.981 ms |
| Uncapped | 44.434 ms | 55.267 ms | 103.637 ms | 113.189 ms |

Every one of 144 observed consecutive Steady frame pairs advanced exactly four
game tics. Frame-interval standard deviation was 0.333 ms, versus 19.555 ms in
the uncapped sample. The owner confirmed more even motion after this correction.
These are the tested starting-position turn results; heavier scenes and preview
traffic can lengthen the interval.

The original fixed-turn diagnostic collected 160 samples with turn held. The
sampled player angle and game clock kept advancing, while engine cost varied
from 43.19 to 110.57 ms as the view rotated (median 53.20 ms). No preview was
transferred. This explains why throughput alone did not describe perceived
motion consistency. The diagnostic reports both DWT work cycles and wall-clock
frame intervals; use `interval_us` for paced frames, since a cycle counter need
not count clock-gated sleep.

Local input commands are now built immediately before simulation. The KM003C
build skips the upstream command-building calls after rendering and after each
simulated tic. This avoids queuing old key states behind a slow frame, while
retaining the original 35 Hz simulation and five-command limit. The host
regression first reproduces delayed turn press/release with a simulated 100 ms
draw, then verifies that both apply in the first frame with the correction.

With the Web server stopped:

```powershell
vendor/km003c-firmware/target/release/km003c-host-cli.exe game-pacing steady
vendor/km003c-firmware/target/release/km003c-host-cli.exe game-profile --samples 160 --turn-right
vendor/km003c-firmware/target/release/km003c-host-cli.exe game-pacing uncapped
```

`game-profile` is read-only by default. **`--turn-right` actively applies a leased
right-turn input** and renews it while sampling, then explicitly releases it.
Cold startup qualification completes before the input test begins. Keep physical
buttons released during this diagnostic. A disconnected input lease expires.
To check the host reproduction, build the host engine with
`--legacy-input-queue`, run `scripts/test-input-cadence.py --expect-legacy-lag`,
then rebuild without that switch and run `scripts/test-input-cadence.py`.

## LZ4 and DMA

Assets currently use palette indices, mipmaps, encoded sprite posts and bounded
record/column/flat caches, with synchronous NOR reads. They are **not LZ4
compressed**. A compressed format would need independently indexed blocks and a
bounded decompression cache; a FIFO ring alone can repeatedly evict randomly
accessed columns. Compression remains an experiment, not an implemented speedup.

The engine already has two indexed strip buffers, but this board's pump is
synchronous. Full RGB565 double buffering at 240×180 would use 172,800 bytes
before engine and capture state. Partial ping-pong DMA is more plausible. Since
LCD and NOR share the controller, a panel DMA transfer must finish before NOR
reads can change the bus owner. CPU work using RAM can overlap; simultaneous NOR
reads cannot. DMA also preserves the physical wire time. The measured 11 ms LCD
region cannot account for the original 176 ms frame by itself.

## Validation

The host harness compares the new projection generator against all existing
320/160-column vanilla angle tables before using the 240/120-column projection.
It checks reads across cache lines and the image end, changing gameplay frames,
movement/turn/fire/menu input, and complete HUD redraw after an LCD clear.
Cached and uncached runs produced identical captured-frame hashes. Firmware
updates use complete application readback and unchanged-loader checks. Device
logs, raw packages and recovery material remain in ignored local artifacts.
