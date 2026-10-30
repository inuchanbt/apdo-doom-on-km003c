# SPDX-License-Identifier: MIT
"""Verify saved recovery artifacts without opening USB or modifying the device."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def verify_external(stock, external):
    data = (external / 'external-4MiB.bin').read_bytes()
    metadata = json.loads((external / 'verified.json').read_text())
    require(len(data) == 0x400000 and digest(data) == metadata['sha256'], 'Full NOR length/hash mismatch')
    require(metadata['matching_reads'] == 2 and metadata['stock_prefix_matches'] is True, 'Two matching full NOR reads and stock-prefix evidence required')
    require(data[:0x300000] == (stock / 'external-first-3MiB.bin').read_bytes(), 'Full NOR differs from stock prefix')
    require(digest(data[0x300000:]) == metadata['protected_tail_sha256'], 'Protected NOR tail hash mismatch')
    return metadata


def verified_app_record(directory, application):
    names = ['application-readback.bin', 'loader-before.bin', 'loader-after.bin', 'verified.json']
    if not all((directory / name).is_file() for name in names):
        return False
    record = json.loads((directory / 'verified.json').read_text())
    return ((directory / 'application-readback.bin').read_bytes() == application
            and (directory / 'loader-before.bin').read_bytes() == (directory / 'loader-after.bin').read_bytes()
            and record.get('loader_unchanged') is True)


def verify_legacy(stock, external, root):
    for sidecar in stock.glob('*.bin.sha256'):
        expected, name = sidecar.read_text().strip().split('  ', 1)
        require(name == Path(name).name and sidecar.name == name + '.sha256', 'Invalid legacy sidecar filename')
        require(digest((stock / name).read_bytes()) == expected, f'Hash mismatch: {name}')
        record = json.loads((stock / (name + '.verified.json')).read_text())
        require(record['matching_reads'] == 2, f'Two reads required: {name}')
    metadata = verify_external(stock, external)
    require((stock / 'loader-00000004-00003fff.bin').read_bytes() == (stock / 'dfu-recovery-check/loader.bin').read_bytes(), 'Legacy DFU loader differs')
    application = (stock / 'stock-app.bin').read_bytes()
    require(verified_app_record(root / 'artifacts/stock-restore-check', application), 'Original stock restoration evidence incomplete')
    # Ignore incomplete later attempts when selecting a completed historical pair.
    for record_path in sorted((root / 'artifacts').glob('restore-nor-*/verified.json'), reverse=True):
        record = json.loads(record_path.read_text())
        app_dir = record_path.parent.parent / record_path.parent.name.replace('restore-nor-', 'restore-stock-')
        if (record.get('full_external_restore_verified') is True
                and record.get('protected_tail_unchanged') is True
                and record.get('sha256') == metadata['sha256']
                and verified_app_record(app_dir, application)):
            return record_path.parent.name
    raise ValueError('Complete matching NOR/application restoration evidence missing')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, default=ROOT / 'local/device.json', help='Local configuration; historical evidence uses its original configuration')
    config = json.loads(parser.parse_args().config.read_text(encoding='utf-8-sig'))
    stock = ROOT / config['stock_backup']
    external = ROOT / config['external_backup']
    if (stock / 'manifest.json').exists():
        manifest = json.loads((stock / 'manifest.json').read_text(encoding='utf-8'))
        require(manifest['identity']['serial'] == config['device_serial'], 'Configured serial differs from stock manifest')
        subprocess.run([str(ROOT / 'bootstrap/target/release/km003c-backup.exe'), 'verify', '--stock', str(stock), '--require-dfu'], check=True)
        verify_external(stock, external)
        require((stock / 'stock-restore.mencrypt').is_file(), 'Stock restore package missing')
        print('PASS: qualified stock/DFU snapshots and full 4 MiB NOR backup; completed hardware restoration is not asserted')
    else:
        completed = verify_legacy(stock, external, ROOT)
        print(f'PASS: original backups and a complete historical stock restoration ({completed})')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'Error: {error}') from error
