"""Build the low-memory DOOM engine as an ARM hard-float static library."""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import os
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / 'vendor/dmc100-doom'
HOST = '--host-test' in sys.argv
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
sources += [ROOT / 'native/doom_bridge.c']
includes = ['engine/stub/include', 'engine', 'port/data', 'port/render', 'port/obj',
            'port/math', 'platform', 'port/game', 'port/ovl', 'gen']
flags = ['--target=thumbv7em-none-eabihf', '-mcpu=cortex-m4', '-mthumb',
         '-mfpu=fpv4-sp-d16', '-mfloat-abi=hard', '-Oz', '-std=gnu11',
         '-ffreestanding', '-fno-common', '-fno-builtin', '-fwrapv', '-fsigned-char',
         '-ffunction-sections', '-fdata-sections', '-DDMC_FW', '-DPLAT_HOST',
         '-DDMC_SINE_L1', '-w']
if HOST:
    flags = flags[5:]
    sources += [ROOT/'native/host_harness.c']
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
