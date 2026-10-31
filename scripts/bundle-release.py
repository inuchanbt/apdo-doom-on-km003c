# SPDX-License-Identifier: MIT
"""Assemble an allowlisted release source tree, then seal assets after verification.

No stock/recovery directory, old board firmware/overlays or proprietary image is
copied. Dependency inputs come from vendor-release.py; git/registry origins stay
separate. Build outputs made during the offline verification are not archived.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TAG = 'v0.1.0-independent'
PREFIX = 'apdo-doom-' + TAG
SCRIPT_NAMES = {
    'env.ps1', 'build-host.ps1', 'build-backup.ps1', 'build-engine.py',
    'build-firmware.ps1', 'extract-splash.py', 'prepare-sources.py',
    'pc-controller.py', 'play.ps1', 'render-frame.py', 'test-engine.py',
    'test-input-cadence.py', 'test-prepare-sources.py', 'vendor-release.py',
    'bundle-release.py',
}
DOC_NAMES = {'BUILD.md', 'FIRST-BACKUP.md', 'INDEPENDENT.md', 'BINARY-RELEASE.md',
             'INSTALL-PREBUILT.md', 'RELEASE-BUILD.md'}


def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def copy(src, dst):
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dst)


def project_files():
    files = subprocess.check_output(['git', '-C', str(ROOT), 'ls-files', '-z'], text=True).split('\0')
    for name in sorted(set(filter(None, files))):
        p = Path(name)
        if (p.parts[0] in {'independent', 'bootstrap', 'native', 'web', 'notices', 'tests'}
            or name in {'README.md', 'README.ja.md', 'LICENSE', 'COPYING', 'SOURCE_LOCK.json', 'THIRD_PARTY_NOTICES.md', '.gitattributes', '.gitignore'}
            or p.parent == Path('scripts') and p.name in SCRIPT_NAMES
            or p.parent == Path('docs') and p.name in DOC_NAMES
            or p.parent == Path('docs/images')
            or name == 'patches/dmc100-doom.patch'):
            yield name


def snapshot_project(destination):
    for name in project_files(): copy(ROOT / name, destination / name)


def copy_git(repo, target):
    files = subprocess.check_output(['git', '-C', str(repo), 'ls-files', '-z'], text=True).split('\0')
    for name in filter(None, files):
        path = repo / name
        if path.is_file(): copy(path, target / name)


def stage(args):
    dst = args.output.resolve()
    dst.mkdir(parents=True, exist_ok=False)
    snapshot_project(dst)
    for entry in args.dependencies.iterdir():
        if entry.is_dir(): shutil.copytree(entry, dst / entry.name, dirs_exist_ok=True)
        else: copy(entry, dst / entry.name)
    dmc = ROOT / 'vendor/dmc100-doom'
    copy_git(dmc, dst / 'vendor/dmc100-doom')
    copy_git(dmc / 'oracle/src/doomgeneric-repo', dst / 'vendor/dmc100-doom/oracle/src/doomgeneric-repo')
    shutil.copytree(dmc / 'gen', dst / 'vendor/dmc100-doom/gen')
    copy(dmc / 'build/packs/E1M1/DOOM.DAT', dst / 'vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT')
    archive = args.freedoom_zip
    assert sha(archive) == '3f9b264f3e3ce503b4fb7f6bdcb1f419d93c7b546f4df3e874dd878db9688f59'
    copy(archive, dst / 'vendor/dmc100-doom/dl/freedoom-0.13.0.zip')
    shutil.copytree(args.rust_source, dst / 'toolchain/rust-library')
    shutil.copytree(args.rust_notices / 'licenses', dst / 'toolchain/rust-notices/licenses')
    copy(args.rust_notices / 'COPYRIGHT-library.html', dst / 'toolchain/rust-notices/COPYRIGHT-library.html')
    # Freeze the allowed file list BEFORE any check build adds target/artifacts.
    files = sorted(p.relative_to(dst).as_posix() for p in dst.rglob('*') if p.is_file())
    (dst / 'SOURCE-FILES.json').write_text(json.dumps(files, indent=2) + '\n')
    print(f'Staged {len(files)} source/notice files; no device recovery material')


def archive_zip(path, entries):
    with zipfile.ZipFile(path, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for source, name in entries:
            z.write(source, name)


def seal(args):
    dst = args.source.resolve()
    subprocess.run(['git', '-C', str(ROOT), 'diff', '--exit-code', '--quiet', 'HEAD'], check=True)
    build=json.loads((dst/'artifacts/offline-build-verified.json').read_text())
    assert all(build.get(k) is True for k in ['host','arm','cargo_offline','fresh_cargo_home'])
    for name in project_files():
        if name.startswith(('independent/','native/','bootstrap/')):
            assert (ROOT/name).read_bytes()==(dst/name).read_bytes(),'Code changed after offline source build: '+name
    subprocess.run(['git','-C',str(ROOT),'diff','--exit-code','--quiet','adfb93b','--',
                    'native','independent/firmware','independent/hardware','independent/protocol',
                    'patches/dmc100-doom.patch','scripts/build-engine.py'],check=True)
    snapshot_project(dst)
    revision = subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip()
    files = sorted(set(json.loads((dst / 'SOURCE-FILES.json').read_text())) | set(project_files()))
    record = {'release': TAG, 'source_commit': revision, 'files': {n: sha(dst / n) for n in files}}
    (dst / 'SOURCE-MANIFEST.json').write_text(json.dumps(record, indent=2) + '\n')
    assets = args.output.resolve()
    assets.mkdir(parents=True, exist_ok=False)
    firmware = assets / f'apdo-doom-km003c-hw2.3-{TAG}.mencrypt'
    copy(args.firmware, firmware)
    audit = json.loads(subprocess.check_output([str(args.audit_host), 'audit-image', str(firmware), str(args.readback)], text=True))
    assert audit['readback_verified'] and audit['model'] == 'KM003C' and audit['version'] == '2.0.7-apdo2'
    evidence = json.loads(args.readback.with_name('verified.json').read_text())
    assert evidence['loader_unchanged'] and evidence['application_sha256'] == audit['installed_sha256']
    assert args.readback.with_name('loader-before.bin').read_bytes() == args.readback.with_name('loader-after.bin').read_bytes()
    all_files = files + ['SOURCE-FILES.json', 'SOURCE-MANIFEST.json']
    source_archive = assets / f'{PREFIX}-source.tar.gz'
    with tarfile.open(source_archive, 'x:gz') as tar:
        for name in all_files:
            tar.add(dst / name, arcname=PREFIX + '/' + name, recursive=False)
    notice_files = [(ROOT / 'COPYING', 'COPYING'), (ROOT / 'LICENSE', 'PROJECT-LICENSE-MIT'),
                    (ROOT / 'THIRD_PARTY_NOTICES.md', 'THIRD_PARTY_NOTICES.md'),
                    (ROOT / 'vendor/dmc100-doom/CREDITS.md', 'DMC100-CREDITS.md')]
    for folder in ['notices', 'independent/hardware']:
        for p in (ROOT / folder).rglob('*'):
            if p.is_file() and (folder == 'notices' or p.name == 'LICENSE-MIT'):
                notice_files.append((p, p.relative_to(ROOT).as_posix()))
    for p in (args.dependencies / 'notices').rglob('*'):
        if p.is_file(): notice_files.append((p, 'rust/' + p.relative_to(args.dependencies / 'notices').as_posix()))
    for p in (ROOT / 'vendor/dmc100-doom/LICENSES').rglob('*'):
        if p.is_file(): notice_files.append((p, 'DMC100-LICENSES/' + p.name))
    for p in (args.rust_notices / 'licenses').iterdir():
        if p.is_file(): notice_files.append((p, 'rust-toolchain/licenses/' + p.name))
    notice_files.append((args.rust_notices / 'COPYRIGHT-library.html', 'rust-toolchain/COPYRIGHT-library.html'))
    archive_zip(assets / f'{PREFIX}-notices.zip', notice_files)
    pack = ROOT / 'vendor/dmc100-doom/build/packs/E1M1/DOOM.DAT'
    assert sha(pack) == '6b25a4f1adb6f1f1f5f42226c272b93ccff4f356ac8d7034cf83712b2da70c08'
    archive_zip(assets / f'{PREFIX}-freedoom-e1m1.zip', [(pack, 'DOOM.DAT'), *notice_files[:4],
        *[(p, 'freedoom/' + p.name) for p in (ROOT / 'notices/freedoom-0.13.0').iterdir() if p.is_file()]])
    copy(ROOT / 'docs/INSTALL-PREBUILT.md', assets / 'INSTALL-PREBUILT.md')
    archive_zip(assets / f'{PREFIX}-firmware.zip',
                [(firmware, firmware.name),
                 (ROOT / 'docs/INSTALL-PREBUILT.md','INSTALL-PREBUILT.md'),
                 *notice_files])
    firmware_revision=subprocess.check_output(['git','-C',str(ROOT),'rev-parse','adfb93b'],text=True).strip()
    manifest = {'release': TAG, 'source_commit': revision, 'firmware_code_commit': firmware_revision,
                'status': 'experimental prerelease', 'tested_hardware': 'one KM003C HW2.3, originally stock 2.0.7',
                'native_flash_readback': audit, 'native_loader_unchanged': True,
                'full_independent_first_install_restore_hardware_qualified': False,
                'offline_source_build':build,
                'toolchain': {'rustc': '1.99.1', 'rust_commit': '58cf8f9c35b355199727627d07ae34a4e2846d67',
                              'target': 'thumbv7em-none-eabihf', 'clang': '23.1.4', 'optimization': 'O2',
                              'square': True, 'cache_kib': 8, 'arena': 'sramh', 'safe_nor': False}}
    manifest['assets'] = {p.name: {'bytes': p.stat().st_size, 'sha256': sha(p)} for p in assets.iterdir() if p.is_file()}
    (assets / 'RELEASE-MANIFEST.json').write_text(json.dumps(manifest, indent=2) + '\n')
    checksum = ''.join(f'{sha(p)}  {p.name}\n' for p in sorted(assets.iterdir()) if p.is_file())
    (assets / 'SHA256SUMS.txt').write_text(checksum)
    print(json.dumps({'source_commit': revision, 'files': len(all_files), 'assets': list(manifest['assets'])}, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    p = sub.add_parser('stage')
    p.add_argument('--dependencies', type=Path, required=True)
    p.add_argument('--freedoom-zip', type=Path, required=True)
    p.add_argument('--rust-source', type=Path, required=True)
    p.add_argument('--rust-notices', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p = sub.add_parser('seal')
    for name in ['source', 'dependencies', 'firmware', 'readback', 'audit-host', 'rust-notices', 'output']:
        p.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    (stage if args.command == 'stage' else seal)(args)


if __name__ == '__main__':
    main()
