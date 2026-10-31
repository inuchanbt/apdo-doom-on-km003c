# Screen capture — independent firmware

The independent firmware records the pixels sent to the LCD into a **120×120
RGB565** software shadow of the square display. Rendering and game simulation
remain on the KM003C. This captures the application's LCD output path, including
the HUD and startup banner; it does not read arbitrary memory, the ST7789 GRAM
or the stock tester UI.

## Use

Run `scripts/play.ps1` and open its printed HTTP URL. **Live preview** transfers
snapshots into the enclosure illustration; **Save frame** downloads the latest
completed image as a lossless top-down RGB565 BMP. The browser enlarges it with
nearest-neighbor pixels. Uncheck Live preview to keep input active while avoiding
capture overhead; the last frame stays visible.

The preview frame is square at desktop and mobile widths. Its desktop size is
limited by both the available column width and viewport height, avoiding tall
empty space in narrow side-by-side layouts. The inset remains calibrated 20 px
narrower and 4 px left of its original placement.

**Device only** rotates the enclosure around the fixed upright video center.
**Device + game** rotates the video too. This affects the PC view only: saved
BMPs stay upright, and backlight changes do not darken the captured pixel values.
See [PC controls](PC-CONTROLS.md).

## Consistency and transport

The live shadow and immutable snapshot each occupy 28,800 bytes. A snapshot is
frozen between render calls. Every bounded USB chunk must identify the same
snapshot generation; a stale generation or invalid range is rejected. The wire
format limits reply payloads to 1,024 bytes; the host also checks the complete
geometry and data length before presenting a frame.

The game uses indexed strip buffers and RGB565 column output rather than a full
240×240 RGB565 framebuffer. Capture retains HUD updates even when a game frame
does not repaint the entire status bar. No flash/NOR writes are issued by normal
preview capture.

The panel reports game render rate and preview update rate separately. Preview
traffic and scene complexity can reduce game cadence; it is not a PC-rate video
stream. Historical 120×90 captures, pipeline details and measurements belong to
[legacy v8 and earlier](https://github.com/inuchanbt/apdo-doom-on-km003c/blob/legacy/docs/SCREEN-CAPTURE.md)
and must not be treated as independent-firmware benchmarks.

The loopback HTTP API exposes only the completed BMP at `/api/frame.bmp`, with
session/origin checks on control requests. No image library is required to play
or save frames. See [validation](VALIDATION.md) for the device checks.
