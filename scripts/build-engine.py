"""Build the low-memory DOOM engine as an ARM hard-float static library."""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import argparse
import hashlib
import json
import os
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / 'vendor/dmc100-doom'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--host-test', action='store_true')
parser.add_argument('--opt', choices=['Oz', 'O2', 'O3'], default='O2')
parser.add_argument('--cache-kib', type=int, choices=[0,8], default=8)
parser.add_argument('--legacy-input-queue', action='store_true', help='comparison only: build input commands after drawing')
parser.add_argument('--square', action='store_true', help='240x208 displayed view plus 32-row status panel')
parser.add_argument('--no-pinned-pages', action='store_true', help='compare external NOR traffic without the internal flash copy')
parser.add_argument('--arena', choices=['sram', 'sramh'], default='sramh')
args = parser.parse_args()
HOST = args.host_test
if not HOST and args.arena == 'sram' and (args.square or args.cache_kib):
    parser.error('ordinary SRAM comparisons require --cache-kib 0 and no --square')
OUT = ROOT / ('artifacts/host-engine' if HOST else 'artifacts/engine')
LLVM = Path(os.environ.get('APDO_LLVM_BIN', ROOT / 'tools/llvm-mingw-20261020-ucrt-x86_64/bin'))
OUT.mkdir(parents=True, exist_ok=True)
excluded = {'am_map.c', 'w_wad.c', 'w_file.c', 'z_zone.c'}
sources = [p for p in (SRC / 'engine').glob('*.c') if p.name not in excluded]
for folder in ['port/data', 'port/render', 'port/obj', 'port/math']:
    sources += [p for p in (SRC / folder).glob('*.c')
                if p.name not in {'img.c', 'img_check.c', 'fat_write.c', 'fat12.c', 'extents.c'}]
sources += [SRC / p for p in ['port/game/fe_glue.c', 'port/game/l_menu.c',
    'port/game/l_wi.c', 'port/game/l_finale.c', 'port/game/i_input.c',
    'port/game/libc_dmc.c', 'platform/rt.c', 'platform/text.c', 'platform/font8x8.c']]
sources += list((SRC / 'gen').glob('*.c'))
sources += [ROOT / 'native/doom_bridge.c', ROOT / 'native/square_view.c']
includes = ['engine/stub/include', 'engine', 'port/data', 'port/render', 'port/obj',
            'port/math', 'platform', 'port/game', 'port/ovl', 'gen']
flags = ['--target=thumbv7em-none-eabihf', '-mcpu=cortex-m4', '-mthumb',
         '-mfpu=fpv4-sp-d16', '-mfloat-abi=hard', '-'+args.opt, '-std=gnu11',
         '-ffreestanding', '-fno-common', '-fno-builtin', '-fwrapv', '-fsigned-char',
         '-ffunction-sections', '-fdata-sections', '-DDMC_FW', '-DPLAT_HOST',
         '-DDMC_SINE_L1', '-w']
flags += ['-DKM_CACHE_KIB='+str(args.cache_kib)]
if args.square and not args.no_pinned_pages:
    pack = (SRC / 'build/packs/E1M1/DOOM.DAT').read_bytes()
    game = json.loads((ROOT / 'SOURCE_LOCK.json').read_text())['game_data']
    if hashlib.sha256(pack).hexdigest() != game['sha256']:
        parser.error('Pinned pages require the matching Freedoom E1M1 pack')
    pages = game['internal_flash_pages_4k']
    table = [0] * ((len(pack) + 4095) // 4096)
    header = [f'#define PINNED_PACK_BYTES {len(pack)}u',
              'static const uint8_t pinned_data[][4096] = {']
    for slot, page in enumerate(pages, start=1):
        table[page] = slot
        data = pack[page * 4096:(page + 1) * 4096]
        for pos in range(0, len(data), 32):
            header.append('"' + ''.join(f'\\x{b:02x}' for b in data[pos:pos + 32]) + '"')
        header[-1] += ','
    header += ['};', 'static const uint8_t pinned_slot[] = {',
               ','.join(map(str, table)), '};']
    (OUT / 'pinned_pages.h').write_text('\n'.join(header) + '\n', encoding='ascii')
    flags += ['-DKM_PINNED_PAGES', '-I', str(OUT)]
if not args.legacy_input_queue:
    flags += ['-DKM_INPUT_AT_FRAME']
if args.square:
    flags += ['-DKM_SQUARE_VIEW']
if HOST:
    flags = flags[5:]
    sources += [ROOT/'native/host_harness.c']
elif args.arena == 'sramh':
    flags += ['-DKM_FAST_ARENA']
for inc in includes:
    flags += ['-I', str(SRC / inc)]

def compile_one(src):
    obj = OUT / (str(src.relative_to(ROOT)).replace('\\', '_').replace('/', '_') + '.o')
    extra=['-U_WIN32'] if HOST and src.name!='host_harness.c' else []
    run = subprocess.run([str(LLVM/'clang.exe'), *flags, *extra, '-c', str(src), '-o', str(obj)],
                         capture_output=True, text=True)
    if run.returncode:
        raise RuntimeError(f'{src}\n{run.stdout}{run.stderr}')
    return str(obj)

with ThreadPoolExecutor(max_workers=min(os.cpu_count() or 4, 8)) as workers:
    objects = list(workers.map(compile_one, sources))
archive = OUT / 'libkmdoom.a'
if HOST:
    library=OUT/'kmdoom-test.dll'
    subprocess.run([str(LLVM/'clang.exe'),'-shared','-Wl,--gc-sections',*objects,'-o',str(library)],check=True)
    print(f'Built {library} ({len(objects)} objects)',flush=True)
    raise SystemExit(0)
if archive.exists():
    archive.unlink()
subprocess.run([str(LLVM/'llvm-ar.exe'), 'rcs', str(archive), *objects], check=True)
print(f'Built {archive} ({len(objects)} objects)', flush=True)
