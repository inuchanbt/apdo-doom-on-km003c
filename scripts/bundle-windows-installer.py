# SPDX-License-Identifier: MIT
"""Package the Windows workflow with the verified public DOOM payloads."""

import argparse
import hashlib
from pathlib import Path
import struct
import subprocess
from zipfile import ZIP_DEFLATED, ZipFile


ROOT = Path(__file__).resolve().parents[1]
FIRMWARE = 'apdo-doom-km003c-v0.1.2-independent.mencrypt'
NOTICES = 'apdo-doom-v0.1.2-independent-notices.zip'
PAYLOAD_HASHES = {
    NOTICES: '59ba198c9405aa64b90021a9be9f4410f682dcf2c423475e8d784e9bd9afed20',
    FIRMWARE: '666129410162fff3b1ef69888393e17edcd82ed6376b59b32b4d2d9b13f346a9',
    'DOOM.DAT': '6b25a4f1adb6f1f1f5f42226c272b93ccff4f356ac8d7034cf83712b2da70c08',
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def windows_executable(path):
    data = path.read_bytes()
    if len(data) < 64 or data[:2] != b'MZ':
        raise ValueError(f'Expected a Windows executable: {path}')
    header = struct.unpack_from('<I', data, 0x3c)[0]
    if data[header:header + 6] != b'PE\0\0\x64\x86':
        raise ValueError(f'Expected a Windows x64 executable: {path}')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--release-assets', type=Path, required=True)
    parser.add_argument('--tools', type=Path, required=True, help='Directory of both built Windows EXEs')
    parser.add_argument('--game', type=Path, required=True, help='Exact public E1M1 DOOM.DAT')
    parser.add_argument('--output', type=Path, required=True, help='New installer ZIP path')
    parser.add_argument('--hardware', choices=('2.3',), default='2.3')
    args = parser.parse_args()

    sources = {
        FIRMWARE: args.release_assets / FIRMWARE,
        NOTICES: args.release_assets / NOTICES,
        'DOOM.DAT': args.game,
    }
    entries = {name: path.read_bytes() for name, path in sources.items()}
    for name, expected in PAYLOAD_HASHES.items():
        if digest(entries[name]) != expected:
            parser.error(f'Unrecognized release payload: {name}')
    for name in ('apdo-host.exe', 'km003c-backup.exe'):
        path = (args.tools / name).resolve()
        entries[name] = windows_executable(path)
        built_for = subprocess.check_output([str(path), 'target-hardware'], text=True).strip()
        if built_for != args.hardware:
            parser.error(f'{name} targets HW {built_for}. Build both tools for HW {args.hardware}.')
    for name in ('Install-APDO-DOOM.cmd', 'Remove-Leftover-DOOM-Data.cmd', 'install-windows.ps1'):
        entries[name] = (ROOT / 'scripts' / name).read_bytes()
    guide = 'INSTALL-WINDOWS.md'
    if args.hardware == '2.1':
        launcher = entries['Install-APDO-DOOM.cmd']
        original = b'-File "%~dp0install-windows.ps1"'
        if launcher.count(original) != 1:
            parser.error('Cannot set the target hardware in the installer launcher')
        entries['Install-APDO-DOOM.cmd'] = launcher.replace(original, original + b' -TargetHardware 2.1')
        guide = 'INSTALL-WINDOWS-HW21-TRIAL.ja.md'
    entries['START-HERE.md'] = (ROOT / 'docs' / guide).read_bytes()
    entries['REMOVE-DOOM-DATA.md'] = (ROOT / 'docs/REMOVE-DOOM-DATA.md').read_bytes()
    entries['SHA256SUMS.txt'] = ''.join(f'{digest(data)}  {name}\n' for name, data in entries.items()).encode('ascii')

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with ZipFile(args.output, 'x', ZIP_DEFLATED, compresslevel=9) as archive:
        for name, data in entries.items():
            archive.writestr(name, data)
    checksum = args.output.with_suffix('.zip.sha256')
    with checksum.open('x', encoding='ascii') as file:
        file.write(f'{digest(args.output.read_bytes())}  {args.output.name}\n')
    print(args.output)


if __name__ == '__main__':
    main()
