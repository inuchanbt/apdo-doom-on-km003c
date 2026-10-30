# PC keyboard controls

Firmware **2.0.7-doom5 (v5)** adds a separate USB game-input endpoint. The user confirmed movement, turning, and firing from the PC on the actual meter. **v6** also adds a [120×90 RGB565 screen preview and image download](SCREEN-CAPTURE.md). Rendering and game simulation still run on the KM003C.

## Start

Connect only the side HID port to the PC. From the prepared workspace, after building the updated host and installing the PC-control firmware:

```powershell
.\scripts\play.ps1
```

The helper opens a local browser page. Click **Enable keyboard**, then play while watching the meter or the v6 preview. The page also has holdable mouse/touch buttons. Keep the helper process running. Use **Stop controller** or Ctrl+C to exit; this releases PC inputs without stopping the game.

`play.ps1 -NoBrowser` prints the URL without opening a tab. The controller uses Python's standard library and the built Rust host CLI; no extra Python packages or web downloads are required. It listens only on `127.0.0.1` on a dynamically allocated port.

| Action | PC key |
| --- | --- |
| Forward / backward | W / S, or ↑ / ↓ |
| Turn left / right | A / D, or ← / → |
| Fire | Space or Ctrl |
| Door / switch | E |
| Menu | Esc |
| Menu up / down / confirm | ↑ / ↓ / Enter |

Use **Release all** to clear inputs. Switching tabs, minimizing, or clicking outside the page releases inputs and disables keyboard capture. Click **Enable keyboard** again when returning. Only mapped game keys are captured while enabled; there is no global keyboard hook.

The server can start before the meter is connected. It automatically restarts the USB helper after connection failures, unplug/replug, or a stalled helper. The HTTP server and URL stay available throughout. Allow several seconds for USB enumeration and game auto-start; no page reload is needed. A disconnect clears keys, old display settings, and captured frames. Click **Enable keyboard** again after reconnection. The Live preview preference is retained.

**Stop controller** ends the server and its retry loop. To resume after stopping it, run `play.ps1` again and open the newly printed URL. Use **Release all** for a temporary input pause. Open the HTTP URL in a browser; opening the HTML file directly does not start the USB bridge.

If an old tab says **Failed to fetch**, compare its port with the current printed URL. A stopped server's cached page can still show an old frame. After restarting a server on the same port, reload the page to obtain its new session.

Run one controller process and one active control tab at a time. Stop it before running another USB CLI command or updating firmware.

## Responsive panel

At browser widths of 900 CSS pixels and above, the live preview and controls appear side by side. Preview size follows the available viewport height, and shorter desktop windows use compact spacing. At narrower widths, the panels stack vertically with normal page scrolling. No controls are hidden to make the desktop layout fit. The enclosure still rotates around the fixed video center.

## Physical LCD settings (v7)

**Meter LCD** provides **Brightness − / +** buttons and **Rotate left / Rotate right** in 90° steps. Brightness is adjustable in 10% steps from 10% to 100%. Here 100% is the original verified LED drive level 70, not the expander's maximum output. The firmware rejects values outside those bounds.

The buttons release and disarm keyboard input before sending a setting. Displayed values follow the device acknowledgement. Rotation clears the LCD and forces a full status-bar redraw; gameplay continues. The preview follows the acknowledged orientation in two modes: **Device only** leaves the live game fixed in position and upright while the enclosure rotates around its calibrated screen center (default); **Device + game** also rotates the video around that fixed center as a joke mode. The enclosure turns opposite to the physical LCD rotation, so 0°, 90°, 180°, and 270° place the Type-C connector left, bottom, right, and top respectively. Switching preview mode changes only the PC view, releases keys, and disarms keyboard input. Raw capture and saved BMP images remain upright because sampling happens before LCD rotation/backlight; the PC picture does not darken. Settings live in RAM only: power-up restores 100% brightness and 0° rotation. Enable keyboard again after adjusting them.

The separate `km003c/doom/display` endpoint accepts status, brightness, or absolute quarter-turn rotation and returns the accepted state. The existing storage/recovery and input schemas remain unchanged. On `game-control` stdin, `brightness 50` and `rotation 3` set 50% and 270° clockwise (90° left); acknowledgements use separate `display` JSON records.

## Input behavior

- PC keys are combined with the physical game buttons. The long-Exit-to-loader gesture uses only the physical Exit button; holding PC fire does not trigger it.
- Short key transitions are queued between rendered frames, including press/release pairs shorter than one frame.
- The enabled control tab sends heartbeats. Idle tabs do not repeatedly send zero input, and focus-loss/key-up handlers release only input owned by that page. After 500 ms without browser input, the helper sends a release. The Rust bridge independently expires silent stdin after 500 ms.
- Every nonzero USB input has a renewable 750 ms lease. The firmware releases it on expiry, even if the PC process crashes or USB disconnects. Lease validation caps it at 1,000 ms. Firmware checks occur between rendered frames.
- The game-input endpoint is separate from the existing storage/recovery commands. Starting the controller does not write flash or enter the loader.

Input response follows the meter's rendering cadence. This does not make the engine render at PC monitor speeds.

## Scripted input and status

With the browser controller stopped:

```powershell
. .\scripts\env.ps1
$cli = '.\vendor\km003c-firmware\target\release\km003c-host-cli.exe'
& $cli game-input 2 --lease-ms 300
& $cli game-input-status
& $cli game-input 0
```

The status request does not renew or change held keys. Masks are: fire `1`, forward `2`, turn left `4`, turn right `8`, use `16`, menu `32`, confirm `64`, backward `128`. Combine them with bitwise OR. The `game-control` command maintains one USB connection, reads decimal masks line by line from stdin, and emits one JSON acknowledgement per exchange. EOF sends release; stale queued transitions are dropped. With `--preview`, it also emits completed `screen` records; stdin commands `preview 0` and `preview 1` pause/resume image capture without changing game keys.

The HTTP helper validates the loopback Host, same-origin POSTs, a per-process session token, and byte-sized masks. It serves only the allowlisted controller assets and its explicit API routes; it does not expose the workspace or backups.

## Validation

Rust tests cover expiry, explicit release, invalid leases, short taps, and stdin EOF/silence. HTTP tests cover ordered input, rejected origins/sessions, invalid masks, and private-file access. On the device, a 300 ms forward input was accepted and then expired to zero; the user confirmed browser keyboard movement, turning, and firing. The application update was fully read back and the native loader remained unchanged.

```powershell
python -m unittest discover -s tests -v
```
