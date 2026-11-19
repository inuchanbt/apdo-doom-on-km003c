"""Replay E1M1 on the host and record physical NOR requests and RGB565 output."""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import struct


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', nargs='?', type=Path)
    parser.add_argument('--trace', type=Path, help='binary little-endian (offset, length) u32 pairs')
    parser.add_argument('--reference', type=Path, help='check every frame against a baseline replay')
    parser.add_argument('--check-fallback', action='store_true', help='exercise a mismatched pinned page')
    args = parser.parse_args()
    if not args.check_fallback and not args.output:
        parser.error('specify an output JSON file')
    root = Path(__file__).resolve().parents[1]
    dll = C.CDLL(str(root / 'artifacts/host-engine/kmdoom-test.dll'))
    dll.km_host_error.restype = C.c_char_p
    dll.km_host_screen.restype = C.c_void_p
    pack = (root / 'vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT').read_bytes()
    data = C.create_string_buffer(pack)
    if args.check_fallback:
        pages = json.loads((root / 'SOURCE_LOCK.json').read_text())['game_data']['internal_flash_pages_4k']
        end = min(len(pack), (pages[-1] + 1) * 4096)
        data[end - 1] = pack[end - 1] ^ 1
    assert dll.km_host_init(data, len(pack)) == 0, dll.km_host_error()
    assert dll.km_host_width() == 240, 'Build the host engine with --square'
    reads = bytearray()
    calls = total = 0

    @C.CFUNCTYPE(None, C.c_uint32, C.c_uint32)
    def observe(offset, length):
        nonlocal calls, total
        calls += 1
        total += length
        if args.trace:
            reads.extend(struct.pack('<II', offset, length))

    dll.km_host_observe_reads(observe)
    if args.check_fallback:
        buf = C.create_string_buffer(64)
        for at in (pages[0] * 4096, end - 64):
            assert dll.km_host_read(at, buf, len(buf)) == 0, dll.km_host_error()
            assert buf.raw == data[at:at + len(buf)]
        assert (calls, total) == (2, 128), 'mismatch must disable the entire hot set'
        print('PASS: mismatched last byte falls back to external NOR for both pages')
        return
    frames = []
    for keys, count in [(0, 35), (2, 35), (8, 30), (1, 25), (0, 15)]:
        for _ in range(count):
            calls = total = 0
            assert dll.km_host_step(keys) == 0, dll.km_host_error()
            frame = C.string_at(dll.km_host_screen(), 320 * 240 * 2)
            frames.append(dict(reads=calls, bytes=total, sha256=hashlib.sha256(frame).hexdigest()))
    args.output.write_text(json.dumps(frames, indent=2) + '\n')
    if args.reference:
        previous = json.loads(args.reference.read_text())
        assert [f['sha256'] for f in frames] == [f['sha256'] for f in previous], 'frame mismatch'
        print('All RGB565 frame hashes match the reference')
    if args.trace:
        args.trace.write_bytes(reads)
    print(f'{len(frames)} frames, {sum(f["reads"] for f in frames)} reads, '
          f'{sum(f["bytes"] for f in frames)} bytes')


if __name__ == '__main__':
    main()
