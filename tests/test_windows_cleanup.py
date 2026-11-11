# SPDX-License-Identifier: MIT
"""Run the actual Windows cleanup orchestration with a USB-free stub CLI."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


@unittest.skipUnless(os.name == 'nt', 'Windows installer')
class Cleanup(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix='apdo-cleanup-test-')
        cls.base = Path(cls.temp.name)
        llvm = Path(os.environ.get('APDO_LLVM_BIN', ROOT / 'tools/llvm-mingw-20261020-ucrt-x86_64/bin'))
        compiler = llvm / 'clang.exe'
        if not compiler.exists():
            cls.temp.cleanup()
            raise unittest.SkipTest('LLVM-MinGW is needed for the USB-free CLI stub')
        stub = cls.base / 'stub.c'
        stub.write_text(r'''
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(int argc, char **argv) {
    if (argc < 2) return 1;
    if (!strcmp(argv[1], "target-hardware")) { puts("2.3"); return 0; }
    FILE *log = fopen(getenv("APDO_CLEANUP_TEST_LOG"), "a");
    if (!log) return 2;
    for (int i=1; i<argc; i++) fprintf(log, "%s%s", i==1 ? "" : "|", argv[i]);
    fputs("\n", log); fclose(log);
    const char *fail = getenv("APDO_CLEANUP_TEST_FAIL");
    return fail && !strcmp(fail, argv[1]) ? 1 : 0;
}
''', encoding='utf-8')
        cls.cli = cls.base / 'stub.exe'
        subprocess.run([str(compiler), '-O2', str(stub), '-o', str(cls.cli)], check=True, capture_output=True)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def run_cleanup(self, fail=''):
        folder = self.base / self._testMethodName
        folder.mkdir()
        script = (ROOT / 'scripts/install-windows.ps1').read_text(encoding='utf-8')
        # Only the fixture payload hashes change. No production code is mocked;
        # the CLI executables are stubs and cannot open USB.
        for name, expected in [
            ('apdo-doom-km003c-v0.1.2-independent.mencrypt', '666129410162fff3b1ef69888393e17edcd82ed6376b59b32b4d2d9b13f346a9'),
            ('DOOM.DAT', '6b25a4f1adb6f1f1f5f42226c272b93ccff4f356ac8d7034cf83712b2da70c08'),
        ]:
            data = ('test fixture ' + name).encode()
            (folder / name).write_bytes(data)
            assert script.count(expected) == 1
            script = script.replace(expected, hashlib.sha256(data).hexdigest())
        (folder / 'install-windows.ps1').write_text(script, encoding='utf-8')
        for name in ('apdo-host.exe', 'km003c-backup.exe'):
            shutil.copyfile(self.cli, folder / name)
        session = folder / 'session'
        (session / 'stock').mkdir(parents=True)
        (session / 'nor').mkdir()
        (session / 'nor/verified.json').write_text('{}')
        log = folder / 'commands.txt'
        env = dict(os.environ, APDO_CLEANUP_TEST_LOG=str(log), APDO_CLEANUP_TEST_FAIL=fail)
        # A PowerShell 7 parent can pass its incompatible module search path to
        # Windows PowerShell 5.1; let the actual launcher runtime use its defaults.
        for key in list(env):
            if key.lower() == 'psmodulepath':
                del env[key]
        ps = Path(os.environ['SystemRoot']) / 'System32/WindowsPowerShell/v1.0/powershell.exe'
        result = subprocess.run([str(ps), '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
                                 str(folder / 'install-windows.ps1'), '-Action', 'RemoveDoomData',
                                 '-BackupPath', str(session)], input='REMOVE-DATA\n\n\n', text=True,
                                capture_output=True, timeout=30, env=env)
        self.assertTrue(log.exists(), result.stdout + result.stderr)
        calls = [line.split('|') for line in log.read_text().splitlines()]
        return result, calls

    def test_cleanup_orders_offline_checks_and_both_restores(self):
        result, calls = self.run_cleanup()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual([c[0] for c in calls], [
            'verify', 'audit-image', 'verify-nor', 'check-app', 'flash', 'start',
            'restore-nor', 'loader', 'flash', 'start',
        ])
        self.assertTrue(calls[4][2].endswith('v0.1.2-independent.mencrypt'))
        self.assertTrue(calls[8][2].endswith('restore.mencrypt'))

    def test_bad_nor_backup_stops_before_firmware_write(self):
        result, calls = self.run_cleanup('verify-nor')
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('flash', [c[0] for c in calls])

    def test_wrong_installed_app_stops_before_firmware_write(self):
        result, calls = self.run_cleanup('check-app')
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('flash', [c[0] for c in calls])

    def test_failed_nor_restore_never_flashes_stock(self):
        result, calls = self.run_cleanup('restore-nor')
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(sum(c[0] == 'flash' for c in calls), 1)
        self.assertEqual(calls[-1][0], 'restore-nor')


if __name__ == '__main__':
    unittest.main()
