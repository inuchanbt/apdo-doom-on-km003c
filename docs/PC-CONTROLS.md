# PC controls — independent firmware

The game runs on the KM003C. The local browser supplies input and displays a
120×120 preview from the independent firmware's LCD output path.

## Start and reconnect

After installation, connect only side HID, leaving both measurement connectors
empty. Build the independent host with `scripts/build-host.ps1`, then run:

```powershell
.\scripts\play.ps1
```

Open the printed HTTP URL in a normal browser and click **Enable keyboard**.
Keep the server running; opening `web/controller.html` directly does not start
USB communication. `play.ps1 -NoBrowser -Port 50594` prints a fixed local URL
without opening a tab. Only Python's standard library and the built host are used.
The server listens on `127.0.0.1`.

| Action | Key |
| --- | --- |
| Forward / backward | W / S or ↑ / ↓ |
| Turn left / right | A / D or ← / → |
| Fire | Space / Ctrl |
| Door / switch | E |
| Menu | Esc |
| Menu up / down / confirm | ↑ / ↓ / Enter |

Buttons also support mouse/touch holds. **Release all** clears inputs. Leaving
the tab releases keys and disables keyboard capture; enable it again on return.
Only mapped keys are captured while enabled, with no global keyboard hook.

The server can start before connection and retries after disconnects or stalled
helpers. Its URL stays available; allow time for USB enumeration and automatic
DOOM startup. Re-enable keyboard after reconnection. A stock or native-DFU device
does not provide the independent game service.

**Stop controller** stops the server and retry loop, releasing PC inputs while
DOOM can continue locally. Run `play.ps1` again to resume and use its printed URL.
If a cached tab says **Failed to fetch**, check the current server URL/port and
reload after restarting the server. Use one server and active control tab; stop
it before running a different USB CLI operation or updating firmware.

## Display and pacing

**Meter LCD** changes actual backlight brightness (10–100%) and LCD orientation
in 90° steps. The panel shows acknowledged values. Settings reset on power-up.
Changing them releases keys and disarms keyboard capture.

**Device only** leaves the game upright at its fixed calibrated position while
the enclosure illustration rotates around it. **Device + game** also rotates
the picture as a joke mode. This selector changes only the PC view; saved BMPs
stay upright. Type-C is left at LCD 0°, bottom at 90°, right at 180°, top at 270°.

**Steady** targets four 35 Hz game tics per frame (8.75 fps); heavier scenes can
exceed that budget. **Uncapped** uses the fastest rendering cadence. Turn off
**Live preview** when comparing smoothness on the meter. See [performance](PERFORMANCE.md).

The preview frame is square, sized to its panel width and capped by available
viewport height. Desktop panels appear side by side from 900 CSS pixels; narrower
windows stack them with ordinary vertical scrolling.

## Input and protocol

PC keys combine with physical buttons. Physical Exit held for two seconds
requests native DFU; PC fire cannot trigger that gesture. The host stops renewing
nonzero input after 500 ms of silent stdin. The device's 750 ms input lease also
releases keys if the host disappears. Checks happen between rendered frames.

The independent binary-framed protocol replaces the legacy RPC endpoints.
`independent/target/release/apdo-host.exe game-control --preview` accepts decimal
button masks on stdin and emits JSON status/capture records. Masks are fire 1,
forward 2, left 4, right 8, use 16, menu 32, confirm 64, backward 128. EOF releases
input. `preview 0/1`, `brightness 10..100`, `rotation 0..3` and `pacing 0/1` change
settings on the same connection. Do not use legacy `game-input`/`game-profile`
commands with this host.

The HTTP helper validates loopback Host, same-origin POSTs, session tokens and
bounded inputs. Only allowlisted Web assets/API routes are served; backups are
not exposed. Run `python -m unittest discover -s tests -v` for the HTTP/reconnect
checks. See [validation](VALIDATION.md) for actual hardware scope.
