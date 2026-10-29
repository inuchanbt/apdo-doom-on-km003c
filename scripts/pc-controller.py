# SPDX-License-Identifier: MIT
"""Local browser keyboard controller. USB is handled by the Rust host CLI."""
import argparse
from collections import deque
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import secrets
import subprocess
import threading
import time
import webbrowser

ROOT = Path(__file__).resolve().parents[1]


class Bridge:
    def __init__(self, cli):
        self.lock = threading.Lock()
        self.token = secrets.token_urlsafe(32)
        self.buttons = 0
        self.updated = 0.0
        self.reply = {}
        self.received = 0.0
        self.history = deque(maxlen=32)
        self.error = ""
        self.stopped = threading.Event()
        self.process = subprocess.Popen(
            [str(cli), "game-control"], cwd=ROOT, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
            encoding="utf-8", errors="replace", bufsize=1,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        for target in (self.read_replies, self.read_errors, self.heartbeat):
            threading.Thread(target=target, daemon=True).start()

    def write_locked(self, buttons):
        try:
            self.process.stdin.write(f"{buttons}\n")
            self.process.stdin.flush()
            return True
        except (BrokenPipeError, OSError, ValueError):
            self.error = self.error or "USB controller stopped; restart the controller."
            return False

    def update(self, buttons):
        with self.lock:
            self.buttons, self.updated = buttons, time.monotonic()
            # Forward transitions immediately, including taps shorter than a heartbeat.
            return self.write_locked(buttons)

    def heartbeat(self):
        while not self.stopped.wait(0.1):
            with self.lock:
                buttons = self.buttons if time.monotonic() - self.updated < 0.5 else 0
                self.write_locked(buttons)

    def read_replies(self):
        for line in self.process.stdout:
            try:
                reply = json.loads(line)
            except ValueError:
                continue
            with self.lock:
                self.reply, self.received = reply, time.monotonic()
                self.history.append(reply)

    def read_errors(self):
        for line in self.process.stderr:
            with self.lock:
                if line.strip():
                    self.error = line.strip()

    def snapshot(self):
        with self.lock:
            return dict(self.reply, connected=self.process.poll() is None and
                        time.monotonic() - self.received < 2, error=self.error,
                        recent_inputs=list(self.history))

    def close(self):
        self.stopped.set()
        with self.lock:
            self.write_locked(0)
            try:
                self.process.stdin.close()
            except (BrokenPipeError, OSError, ValueError):
                pass
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            self.process.wait(timeout=5)


def handler_for(bridge):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def valid_host(self):
            return self.headers.get("Host") == f"127.0.0.1:{self.server.server_port}"

        def respond(self, status, body, content_type="application/json"):
            if isinstance(body, dict):
                body = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.send_header("Content-Security-Policy", "default-src 'self'; frame-ancestors 'none'; base-uri 'none'")
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if not self.valid_host():
                return self.respond(403, {"error": "Invalid host"})
            if self.path == "/api/session":
                return self.respond(200, {"token": bridge.token})
            if self.path == "/api/status":
                return self.respond(200, bridge.snapshot())
            assets = {"/": ("controller.html", "text/html; charset=utf-8"),
                      "/controller.js": ("controller.js", "text/javascript; charset=utf-8"),
                      "/controller.css": ("controller.css", "text/css; charset=utf-8")}
            if self.path not in assets:
                return self.respond(404, {"error": "Not found"})
            name, content_type = assets[self.path]
            self.respond(200, (ROOT / "web" / name).read_bytes(), content_type)

        def do_POST(self):
            try:
                self.connection.settimeout(2)
                length = int(self.headers.get("Content-Length", "0"))
                # Drain bounded requests before rejecting them. Closing a Windows
                # socket with unread input can reset it before the error is read.
                body = self.rfile.read(min(max(length, 0), 4096))
                origin = f"http://127.0.0.1:{self.server.server_port}"
                if (not self.valid_host() or self.headers.get("Origin") != origin or
                        not secrets.compare_digest(self.headers.get("X-APDO-Token", ""), bridge.token)):
                    return self.respond(403, {"error": "Invalid session"})
                if not 0 < length <= 64:
                    raise ValueError()
                data = json.loads(body)
                if not isinstance(data, dict):
                    raise ValueError()
                if self.path == "/api/keys":
                    buttons = data.get("buttons")
                    if type(buttons) is not int or not 0 <= buttons <= 255:
                        raise ValueError()
                    ok = bridge.update(buttons)
                    return self.respond(200 if ok else 409, {"ok": ok})
                if self.path == "/api/stop":
                    bridge.update(0)
                    self.respond(200, {"ok": True})
                    threading.Thread(target=self.server.shutdown, daemon=True).start()
                    return
                self.respond(404, {"error": "Not found"})
            except (ValueError, TypeError, TimeoutError):
                self.respond(400, {"error": "Expected a key mask from 0 to 255"})
    return Handler


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, default=ROOT / "vendor/km003c-firmware/target/release/km003c-host-cli.exe")
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--no-browser", action="store_true")
    parser.add_argument("--session-file", type=Path)
    args = parser.parse_args()
    bridge = Bridge(args.cli)
    server = None
    try:
        server = ThreadingHTTPServer(("127.0.0.1", args.port), handler_for(bridge))
        url = f"http://127.0.0.1:{server.server_port}/"
        if args.session_file:
            args.session_file.write_text(json.dumps({"url": url, "pid": os.getpid()}) + "\n")
        print(f"KM003C controller: {url}", flush=True)
        print("Keep this process running. Close with Stop controller or Ctrl+C.", flush=True)
        if not args.no_browser:
            webbrowser.open(url)
        server.serve_forever(poll_interval=0.1)
    except KeyboardInterrupt:
        pass
    finally:
        if server:
            server.server_close()
        bridge.close()


if __name__ == "__main__":
    main()
