# SPDX-License-Identifier: MIT
"""Extract the original Freedoom menu logo from its verified release archive.

The generated pixels retain Freedoom's BSD license (not this script's MIT).
No commercial IWAD input or unverified release is accepted.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import struct
import zipfile

ROOT = Path(__file__).resolve().parents[1]
ZIP_SHA = '3f9b264f3e3ce503b4fb7f6bdcb1f419d93c7b546f4df3e874dd878db9688f59'
MEMBER = 'freedoom-0.13.0/freedoom1.wad'


def extract(archive):
    if hashlib.sha256(archive).hexdigest() != ZIP_SHA:
        raise ValueError('Expected the pinned official Freedoom 0.13.0 archive')
    wad = zipfile.ZipFile(io.BytesIO(archive)).read(MEMBER)
    if wad[:4] != b'IWAD':
        raise ValueError('Invalid Freedoom IWAD')
    count, directory = struct.unpack_from('<II', wad, 4)
    if count > 65536 or directory + count * 16 > len(wad):
        raise ValueError('Invalid WAD directory')
    lumps = {}
    for i in range(count):
        start, length, name = struct.unpack_from('<II8s', wad, directory + i * 16)
        if start + length > len(wad):
            raise ValueError('Lump outside WAD')
        lumps[name.rstrip(b'\0')] = wad[start:start + length]
    patch, palette = lumps[b'M_DOOM'], lumps[b'PLAYPAL'][:768]
    width, height = struct.unpack_from('<HH', patch)
    if (width, height) != (159, 37) or len(palette) != 768:
        raise ValueError('Unexpected Freedoom banner geometry or palette')
    pixels = [0] * (width * height)  # Transparent posts sit on black.
    for x in range(width):
        at = struct.unpack_from('<I', patch, 8 + x * 4)[0]
        last = -1
        while at < len(patch) and patch[at] != 255:
            if at + 3 > len(patch):
                raise ValueError('Truncated patch post')
            top, length = patch[at:at + 2]
            top = top + last if top <= last else top
            last = top
            at += 3
            if top + length > height or at + length + 1 > len(patch):
                raise ValueError('Invalid patch post')
            for y, index in enumerate(patch[at:at + length], top):
                r, g, b = palette[index * 3:index * 3 + 3]
                pixels[y * width + x] = (r >> 3) << 11 | (g >> 2) << 5 | (b >> 3)
            at += length + 1
        if at >= len(patch):
            raise ValueError('Missing patch terminator')
    image = struct.pack('<HH', width, height) + struct.pack(f'<{len(pixels)}H', *pixels)
    return image, {
        'asset': 'Freedoom Phase 1 menu logo', 'release': '0.13.0',
        'release_url': 'https://github.com/freedoom/freedoom/releases/download/v0.13.0/freedoom-0.13.0.zip',
        'release_sha256': ZIP_SHA, 'wad_member': MEMBER, 'lump': 'M_DOOM',
        'lump_sha256': hashlib.sha256(patch).hexdigest(),
        'width': width, 'height': height,
        'format': 'little-endian u16 width, height, then row-major RGB565; transparency flattened onto black',
        'sha256': hashlib.sha256(image).hexdigest(),
        'license': 'Freedoom modified BSD; see notices/freedoom-0.13.0/COPYING.txt and CREDITS.txt',
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--zip', type=Path, default=ROOT / 'vendor/dmc100-doom/dl/freedoom-0.13.0.zip')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    image, metadata = extract(args.zip.read_bytes())
    target = ROOT / 'independent/firmware/assets/freedoom-banner.rgb565'
    manifest = target.with_suffix('.json')
    if args.check:
        if target.read_bytes() != image or json.loads(manifest.read_text()) != metadata:
            raise SystemExit('Freedoom banner differs from the verified original')
        print('Verified original Freedoom banner pixels and provenance')
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(image)
        manifest.write_text(json.dumps(metadata, indent=2) + '\n')
        print(f'Extracted M_DOOM: 159x37, {len(image)} bytes, SHA-256 {metadata["sha256"]}')


if __name__ == '__main__':
    main()
