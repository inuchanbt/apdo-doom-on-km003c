# SPDX-License-Identifier: MIT
"""Exercise local HTTP input boundaries without USB or a real keyboard."""
import http.client
import importlib.util
import json
import struct
from pathlib import Path
import threading
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


if __name__ == "__main__":
    unittest.main()
