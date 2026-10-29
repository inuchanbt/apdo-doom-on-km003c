# SPDX-License-Identifier: MIT
"""Exercise local HTTP input boundaries without USB or a real keyboard."""
import http.client
import importlib.util
import json
import queue
import struct
import subprocess
from pathlib import Path
import threading
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("pc_controller", ROOT / "scripts/pc-controller.py")
controller = importlib.util.module_from_spec(spec)
spec.loader.exec_module(controller)


class FakeBridge:
    token = "test-session"

    def __init__(self):
        self.inputs = []
        self.preview = []
        self.display = []

    def update(self, buttons):
        self.inputs.append(buttons)
        return True

    def snapshot(self):
        return {"connected": True}
    def frame(self):
        return None
    def set_preview(self, enabled):
        self.preview.append(enabled)
        return True
    def set_display(self, setting, value):
        self.display.append((setting, value))
        return True


class ControllerHttpTests(unittest.TestCase):
    def setUp(self):
        self.bridge = FakeBridge()
        self.server = controller.ThreadingHTTPServer(("127.0.0.1", 0), controller.handler_for(self.bridge))
        self.worker = threading.Thread(target=self.server.serve_forever, kwargs={"poll_interval": 0.01})
        self.worker.start()
        self.origin = f"http://127.0.0.1:{self.server.server_port}"

    def tearDown(self):
        self.server.shutdown()
        self.server.server_close()
        self.worker.join()

    def request(self, path, payload=None, headers=None):
        connection = http.client.HTTPConnection("127.0.0.1", self.server.server_port, timeout=2)
        try:
            actual_headers = {"Origin": self.origin, "X-APDO-Token": self.bridge.token,
                              "Content-Type": "application/json"}
            actual_headers.update(headers or {})
            body = json.dumps(payload) if payload is not None else None
            connection.request("POST" if payload is not None else "GET", path, body=body, headers=actual_headers)
            response = connection.getresponse()
            return response.status, response.read()
        finally:
            connection.close()

    def test_press_then_release_reaches_bridge_in_order(self):
        self.assertEqual(self.request("/api/keys", {"buttons": 2})[0], 200)
        self.assertEqual(self.request("/api/keys", {"buttons": 0})[0], 200)
        self.assertEqual(self.bridge.inputs, [2, 0])

    def test_other_origins_and_sessions_cannot_send_input(self):
        self.assertEqual(self.request("/api/keys", {"buttons": 1}, {"Origin": "https://example.com"})[0], 403)
        self.assertEqual(self.request("/api/keys", {"buttons": 1}, {"X-APDO-Token": ""})[0], 403)
        self.assertEqual(self.request("/api/keys", {"buttons": 1}, {"Host": "example.com"})[0], 403)
        self.assertEqual(self.bridge.inputs, [])

    def test_invalid_masks_are_not_forwarded(self):
        for payload in ({"buttons": -1}, {"buttons": 256}, {"buttons": True}, {"buttons": "2"}, [], {"buttons": 1, "extra": "x" * 64}):
            with self.subTest(payload=payload):
                self.assertEqual(self.request("/api/keys", payload)[0], 400)
        self.assertEqual(self.bridge.inputs, [])

    def test_private_files_are_not_served(self):
        self.assertEqual(self.request("/../local/device.json")[0], 404)
        self.assertEqual(self.request("/local/device.json")[0], 404)

    def test_missing_frame_is_reported(self):
        self.assertEqual(self.request("/api/frame.bmp?snapshot=1")[0], 404)

    def test_preview_can_pause_without_sending_game_keys(self):
        self.assertEqual(self.request("/api/preview", {"enabled": False})[0], 200)
        self.assertEqual(self.request("/api/preview", {"enabled": True})[0], 200)
        self.assertEqual(self.request("/api/preview", {"enabled": 1})[0], 400)
        self.assertEqual(self.bridge.preview, [False, True])
        self.assertEqual(self.bridge.inputs, [])

    def test_lcd_settings_validate_values_without_sending_game_keys(self):
        for setting, value in [("brightness", 10), ("brightness", 100), ("rotation", 0), ("rotation", 3)]:
            self.assertEqual(self.request("/api/display", {"setting": setting, "value": value})[0], 200)
        self.assertEqual(len(self.bridge.display), 4)
        for setting, value in [("brightness", 0), ("brightness", 101), ("rotation", 4), ("rotation", True), ("loader", 1)]:
            self.assertEqual(self.request("/api/display", {"setting": setting, "value": value})[0], 400)
        self.assertEqual(len(self.bridge.display), 4)
        self.assertEqual(self.bridge.inputs, [])
        self.assertEqual(self.request("/api/display", {"setting": "rotation", "value": 1}, {"Origin": "https://example.com"})[0], 403)


class FrameEncodingTests(unittest.TestCase):
    def test_rgb565_order_orientation_and_padding(self):
        bitmap = controller.frame_bmp({"width": 1, "height": 2, "rgb565": "00f8e007"})
        self.assertEqual(bitmap[:2], b"BM")
        self.assertEqual(struct.unpack_from("<ii", bitmap, 18), (1, -2))
        self.assertEqual(struct.unpack_from("<III", bitmap, 54), (0xf800, 0x07e0, 0x001f))
        self.assertEqual(bitmap[66:], b"\x00\xf8\0\0\xe0\x07\0\0")
    def test_rejects_truncated_and_unbounded_payloads(self):
        for frame in ({"width": 120, "height": 90, "rgb565": "00"},
                      {"width": 65535, "height": 90, "rgb565": ""},
                      {"width": 1, "height": 1, "rgb565": "zzzz"}):
            with self.subTest(frame=frame), self.assertRaises(ValueError):
                controller.frame_bmp(frame)


class FakePipe:
    def __init__(self):
        self.lines = queue.Queue()
    def __iter__(self):
        while True:
            line = self.lines.get()
            if line is None:
                return
            yield line
    def close(self):
        pass


class FakeUsbProcess:
    def __init__(self, alive=True):
        self.stdout, self.stderr = FakePipe(), FakePipe()
        self.commands = []
        self.exited = threading.Event()
        self.returncode = None
        self.stdin = self
        self.frames = 0
        if not alive:
            self.finish()
    def write(self, line):
        if self.poll() is not None:
            raise BrokenPipeError()
        self.commands.append(line.strip())
        if line.strip().isdigit():
            self.frames += 1
            self.emit({"frames": self.frames, "running": True, "applied_buttons": int(line)})
    def flush(self):
        pass
    def close(self):
        self.finish()
    def emit(self, reply):
        self.stdout.lines.put(json.dumps(reply) + "\n")
    def finish(self):
        if not self.exited.is_set():
            self.returncode = 1
            self.exited.set()
            self.stdout.lines.put(None)
            self.stderr.lines.put(None)
    def poll(self):
        return self.returncode
    def wait(self, timeout):
        if not self.exited.wait(timeout):
            raise subprocess.TimeoutExpired("fake USB", timeout)
        return self.returncode
    terminate = finish
    kill = finish


class UsbReconnectTests(unittest.TestCase):
    def wait_for(self, predicate):
        deadline = time.monotonic() + 3
        while time.monotonic() < deadline:
            if predicate():
                return
            time.sleep(0.005)
        self.fail("USB bridge state did not converge")

    def test_late_connect_replug_drops_keys_and_accepts_restarted_frame_counter(self):
        missing, first, second = FakeUsbProcess(False), FakeUsbProcess(), FakeUsbProcess()
        processes = iter([missing, first, second])
        bridge = controller.Bridge("fake", process_factory=lambda *a, **k: next(processes), retry_seconds=0.01)
        try:
            self.assertFalse(bridge.update(2))
            self.wait_for(lambda: bridge.snapshot()["connected"])
            self.assertEqual(bridge.snapshot()["connection_id"], 2)
            self.assertEqual(first.commands[:2], ["0", "preview 1"])
            self.assertTrue(bridge.update(2))
            first.emit({"screen": {"snapshot": 1, "width": 1, "height": 1, "rgb565": "00f8"}})
            self.wait_for(lambda: bridge.snapshot()["screen"] is not None)
            frame_id = bridge.snapshot()["screen"]["frame_id"]
            bridge.set_preview(False)
            first.finish()
            self.wait_for(lambda: bridge.snapshot()["connection_id"] == 3 and bridge.snapshot()["connected"])
            self.assertEqual(second.commands[:2], ["0", "preview 0"])
            self.assertTrue(all(command == "0" for command in second.commands if command.isdigit()))
            self.assertIsNone(bridge.snapshot()["screen"])
            self.assertIsNone(bridge.snapshot()["display"])
            bridge.accept_reply(first, {"frames": 99999, "running": False})
            self.assertTrue(bridge.snapshot()["running"])
            second.emit({"screen": {"snapshot": 1, "width": 1, "height": 1, "rgb565": "e007"}})
            self.wait_for(lambda: bridge.snapshot()["screen"] is not None)
            self.assertGreater(bridge.snapshot()["screen"]["frame_id"], frame_id)
        finally:
            bridge.close()
        self.assertFalse(bridge.worker.is_alive())
        self.assertIsNotNone(second.poll())

    def test_helper_launch_error_keeps_server_usable_and_stop_cancels_retry(self):
        failed = threading.Event()
        def unavailable(*args, **kwargs):
            failed.set()
            raise FileNotFoundError("test helper missing")
        bridge = controller.Bridge("missing", process_factory=unavailable, retry_seconds=1)
        try:
            self.assertTrue(failed.wait(1))
            self.assertFalse(bridge.snapshot()["connected"])
            self.assertTrue(bridge.update(0))
            self.assertFalse(bridge.update(1))
            self.assertTrue(bridge.set_preview(False))
        finally:
            bridge.close()
        self.assertFalse(bridge.worker.is_alive())


if __name__ == "__main__":
    unittest.main()
