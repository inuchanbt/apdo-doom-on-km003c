# PC keyboard controls

Firmware **2.0.7-doom5 (v5)** adds a separate USB game-input endpoint. The user confirmed movement, turning, and firing from the PC on the actual meter. Rendering and game simulation still run on the KM003C; the browser is a controller, not a video stream.

## Start

Connect only the side HID port to the PC. From the prepared workspace, after building the updated host and installing the PC-control firmware:

```powershell
.\scripts\play.ps1
```

The helper opens a local browser page. Click **Enable keyboard**, then play while watching the meter. The page also has holdable mouse/touch buttons. Keep the helper process running. Use **Stop controller** or Ctrl+C to exit; this releases PC inputs without stopping the game.

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

Run one controller process and one active control tab at a time. Stop it before running another USB CLI command or updating firmware.

## Input behavior

- PC keys are combined with the physical game buttons. The long-Exit-to-loader gesture uses only the physical Exit button; holding PC fire does not trigger it.
- Short key transitions are queued between rendered frames, including press/release pairs shorter than one frame.
- The browser sends heartbeats. After 500 ms without browser input, the helper sends a release. The Rust bridge independently expires silent stdin after 500 ms.
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

The status request does not renew or change held keys. Masks are: fire `1`, forward `2`, turn left `4`, turn right `8`, use `16`, menu `32`, confirm `64`, backward `128`. Combine them with bitwise OR. The `game-control` command maintains one USB connection, reads decimal masks line by line from stdin, and emits one JSON acknowledgement per exchange. EOF sends release; stale queued transitions are dropped.

The HTTP helper validates the loopback Host, same-origin POSTs, a per-process session token, and byte-sized masks. It serves only the three controller assets and its explicit API routes; it does not expose the workspace or backups.

## Validation

Rust tests cover expiry, explicit release, invalid leases, short taps, and stdin EOF/silence. HTTP tests cover ordered input, rejected origins/sessions, invalid masks, and private-file access. On the device, a 300 ms forward input was accepted and then expired to zero; the user confirmed browser keyboard movement, turning, and firing. The application update was fully read back and the native loader remained unchanged.

```powershell
python -m unittest discover -s tests -v
```
