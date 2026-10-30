"""Build the low-memory DOOM engine as an ARM hard-float static library."""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import argparse
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
parser.add_argument('--square', action='store_true', help='240x200 world plus complete 40-row HUD')
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
