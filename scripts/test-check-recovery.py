# SPDX-License-Identifier: MIT
"""Synthetic snapshot checks; no firmware dump or USB required."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('check_recovery', Path(__file__).with_name('check-recovery.py'))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.stock = self.root / 'stock'
        self.external = self.root / 'external'
        self.stock.mkdir()
        self.external.mkdir()
        self.data = bytes([1]) * 0x300000 + bytes([2]) * 0x100000
        self.meta = {'sha256': check.digest(self.data), 'matching_reads': 2,
                     'stock_prefix_matches': True, 'protected_tail_sha256': check.digest(self.data[0x300000:])}
        (self.stock / 'external-first-3MiB.bin').write_bytes(self.data[:0x300000])
        (self.external / 'external-4MiB.bin').write_bytes(self.data)
        self.write_meta()

    def write_meta(self):
        (self.external / 'verified.json').write_text(json.dumps(self.meta))

    def test_valid_snapshot(self):
        self.assertEqual(check.verify_external(self.stock, self.external), self.meta)

    def test_changed_prefix_is_rejected(self):
        (self.stock / 'external-first-3MiB.bin').write_bytes(bytes([3]) * 0x300000)
        with self.assertRaisesRegex(ValueError, 'differs from stock prefix'):
            check.verify_external(self.stock, self.external)

    def test_truncated_or_corrupt_capture_is_rejected(self):
        for data in [self.data[:-1], bytes([3]) + self.data[1:]]:
            (self.external / 'external-4MiB.bin').write_bytes(data)
            with self.assertRaisesRegex(ValueError, 'length/hash'):
                check.verify_external(self.stock, self.external)

    def test_updated_hash_does_not_hide_changed_tail(self):
        data = self.data[:-1] + bytes([4])
        (self.external / 'external-4MiB.bin').write_bytes(data)
        self.meta['sha256'] = check.digest(data)
        self.write_meta()
        with self.assertRaisesRegex(ValueError, 'tail hash'):
            check.verify_external(self.stock, self.external)

    def test_required_read_evidence(self):
        for field, invalid in [('matching_reads', 1), ('stock_prefix_matches', False)]:
            old = self.meta[field]
            self.meta[field] = invalid
            self.write_meta()
            with self.assertRaisesRegex(ValueError, 'Two matching'):
                check.verify_external(self.stock, self.external)
            self.meta[field] = old

    def app_record(self, directory, complete=True):
        directory.mkdir(parents=True)
        if complete:
            (directory / 'application-readback.bin').write_bytes(b'synthetic application')
            (directory / 'loader-before.bin').write_bytes(b'synthetic loader')
            (directory / 'loader-after.bin').write_bytes(b'synthetic loader')
            (directory / 'verified.json').write_text('{"loader_unchanged": true}')

    def legacy(self):
        (self.stock / 'stock-app.bin').write_bytes(b'synthetic application')
        (self.stock / 'loader-00000004-00003fff.bin').write_bytes(b'synthetic loader')
        (self.stock / 'dfu-recovery-check').mkdir()
        (self.stock / 'dfu-recovery-check/loader.bin').write_bytes(b'synthetic loader')
        self.app_record(self.root / 'artifacts/stock-restore-check')
        for stamp, complete in [('001', True), ('002', False)]:
            nor = self.root / ('artifacts/restore-nor-' + stamp)
            nor.mkdir()
            (nor / 'verified.json').write_text(json.dumps({'full_external_restore_verified': True,
                'protected_tail_unchanged': True, 'sha256': self.meta['sha256']}))
            self.app_record(self.root / ('artifacts/restore-stock-' + stamp), complete)

    def test_incomplete_run_does_not_hide_complete_historical_pair(self):
        self.legacy()
        self.assertEqual(check.verify_legacy(self.stock, self.external, self.root), 'restore-nor-001')

    def test_nor_only_record_never_proves_complete_restoration(self):
        self.legacy()
        (self.root / 'artifacts/restore-stock-001/loader-after.bin').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'Complete matching'):
            check.verify_legacy(self.stock, self.external, self.root)


if __name__ == '__main__':
    unittest.main()
