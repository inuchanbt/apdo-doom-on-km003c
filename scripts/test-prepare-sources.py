# SPDX-License-Identifier: MIT
"""Offline tests for bootstrap verification before importing a private baseline."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("prepare_sources", Path(__file__).with_name("prepare-sources.py"))
prepare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prepare)


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.patch = patch.object(prepare, "ROOT", self.root)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        (self.root / "local").mkdir()
        (self.root / "local/device.json").write_text(json.dumps({"device_serial": "123456", "stock_backup": "stock"}))
        self.stock = self.root / "stock"
        self.stock.mkdir()
        self.source = self.stock / "loader-00000004-00003fff.bin"
        self.data = b"synthetic" * (0x3ffc // 9)
        self.data = self.data.ljust(0x3ffc, b"\x00")
        self.source.write_bytes(self.data)
        Path(str(self.source) + ".verified.json").write_text('{"matching_reads": 2}')
        Path(str(self.source) + ".sha256").write_text(hashlib.sha256(self.data).hexdigest())
        self.target = self.root / "vendor/km003c-firmware/fw/bootloader_local_hw2.3.bin"
        self.target.parent.mkdir(parents=True)
        self.verifier = self.root / "bootstrap/target/release/km003c-backup.exe"
        self.verifier.parent.mkdir(parents=True)
        self.verifier.touch()

    def manifest(self, serial="123456"):
        (self.stock / "manifest.json").write_text(json.dumps({"identity": {"serial": serial}}))

    def test_new_baseline_requires_manifest(self):
        with self.assertRaisesRegex(RuntimeError, "FIRST-BACKUP"):
            prepare.prepare_baseline()
        self.assertFalse(self.target.exists())

    def test_serial_mismatch_never_runs_verifier(self):
        self.manifest("654321")
        with patch.object(prepare.subprocess, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "serial differs"):
                prepare.prepare_baseline()
            run.assert_not_called()

    def test_missing_verifier_has_build_instruction(self):
        self.manifest()
        self.verifier.unlink()
        with self.assertRaisesRegex(RuntimeError, "build-backup"):
            prepare.prepare_baseline()

    def test_failed_qualification_never_imports_loader(self):
        self.manifest()
        with patch.object(prepare.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "", "DFU mismatch")):
            with self.assertRaisesRegex(RuntimeError, "DFU mismatch"):
                prepare.prepare_baseline()
        self.assertFalse(self.target.exists())

    def test_success_invokes_offline_dfu_verifier_before_import(self):
        self.manifest()
        with patch.object(prepare.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "", "")) as run:
            prepare.prepare_baseline()
            self.assertEqual(run.call_args.args[0], [str(self.verifier), "verify", "--stock", str(self.stock), "--require-dfu"])
        self.assertEqual(self.target.read_bytes(), self.data)

    def test_existing_experiment_baseline_is_preserved(self):
        self.target.write_bytes(self.data)
        prepare.prepare_baseline(check=True)
        self.target.write_bytes(b"different")
        with self.assertRaisesRegex(RuntimeError, "differs"):
            prepare.prepare_baseline()

    def test_check_does_not_create_baseline(self):
        self.manifest()
        with patch.object(prepare.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "", "")):
            with self.assertRaisesRegex(RuntimeError, "Missing private"):
                prepare.prepare_baseline(check=True)
        self.assertFalse(self.target.exists())


if __name__ == "__main__":
    unittest.main()
