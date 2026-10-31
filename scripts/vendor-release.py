# SPDX-License-Identifier: MIT
"""Bundle pinned Cargo sources by origin, preserving same-name registry/git crates.

Run cargo metadata/fetch first. Directory replacement is isolated per upstream
source, so embassy-futures 0.1.2 from crates.io and git remain distinct packages.
Original git manifests are retained; build manifests expand workspace inheritance
and express intra-repository path dependencies using the same pinned git source.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tomllib
from urllib.parse import urlsplit, parse_qs


def toml(data):
    lines = []
    def key(k): return json.dumps(k, ensure_ascii=False)
    def value(v):
        if isinstance(v, bool): return str(v).lower()
        if isinstance(v, str): return key(v)
        if isinstance(v, (int, float)): return str(v)
        if isinstance(v, list): return '[' + ', '.join(value(x) for x in v) + ']'
        raise ValueError(f'Unsupported TOML value {v!r}')
    def table(d, path=(), array=False):
        if path:
            label = '.'.join(map(key, path))
            lines.append(('[' if not array else '[[') + label + (']' if not array else ']]'))
        for k, v in d.items():
            if not isinstance(v, dict) and not (isinstance(v, list) and v and isinstance(v[0], dict)):
                lines.append(key(k) + ' = ' + value(v))
        for k, v in d.items():
            if isinstance(v, dict): table(v, path + (k,))
            elif isinstance(v, list) and v and isinstance(v[0], dict):
                for item in v: table(item, path + (k,), True)
        lines.append('')
    table(data)
    result = '\n'.join(lines)
    assert tomllib.loads(result) == data
    return result


def git_root(path):
    return Path(subprocess.check_output(['git', '-C', str(path), 'rev-parse', '--show-toplevel'], text=True).strip())


def git_details(source):
    u = urlsplit(source[4:])
    params = parse_qs(u.query)
    fields = {'git': u._replace(query='', fragment='').geturl()}
    for key in ('rev', 'branch', 'tag'):
        if key in params: fields[key] = params[key][0]
    return fields


def normalize(package, source, root, original):
    d = tomllib.loads(original)
    inherited = d.get('package', {})
    for key, val in list(inherited.items()):
        if isinstance(val, dict) and val.get('workspace'):
            field = key.replace('-', '_')
            if package.get(field) is not None: inherited[key] = package[field]
            else: del inherited[key]
    d.pop('workspace', None)
    if d.get('lints', {}).get('workspace'):
        workspace = tomllib.loads((root / 'Cargo.toml').read_text()).get('workspace', {})
        d['lints'] = workspace.get('lints', {})
    def deps(table, kind, target=None):
        for alias, entry in list(table.items()):
            if not isinstance(entry, dict): continue
            if entry.get('workspace'):
                match = next(x for x in package['dependencies']
                             if (x.get('rename') or x['name']) == alias and x['kind'] == kind and x['target'] == target)
                entry = {'version': match['req'], 'features': match['features'],
                         'default-features': match['uses_default_features'], 'optional': match['optional']}
                if match.get('rename'): entry['package'] = match['name']
                dep_source = match.get('source')
                if match.get('path'): entry.update(git_details(source))
                elif dep_source and dep_source.startswith('git+'): entry.update(git_details(dep_source))
                table[alias] = entry
            elif 'path' in entry:
                del entry['path']
                entry.update(git_details(source))
    for label, kind in [('dependencies', None), ('build-dependencies', 'build'), ('dev-dependencies', 'dev')]:
        deps(d.get(label, {}), kind)
        for target, target_data in d.get('target', {}).items():
            deps(target_data.get(label, {}), kind, target)
    return toml(d)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--metadata', type=Path, action='append', required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--extend',action='store_true',help='add newly resolved packages to an existing staging directory')
    args = p.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=args.extend)
    packages = {}
    project = Path(__file__).resolve().parents[1]
    checksums = {}
    for lock in [project/'independent/Cargo.lock', project/'bootstrap/Cargo.lock']:
        for pkg in tomllib.loads(lock.read_text())['package']:
            if 'checksum' in pkg: checksums[(pkg['name'],pkg['version'],pkg['source'])]=pkg['checksum']
    for path in args.metadata:
        for pkg in json.loads(path.read_text(encoding='utf-8-sig'))['packages']:
            if pkg['source']: packages[pkg['id']] = pkg
    sources, notices, inventory = {}, out / 'notices', []
    if args.extend and (out/'DEPENDENCIES.json').exists():
        inventory=json.loads((out/'DEPENDENCIES.json').read_text())
        sources=tomllib.loads((out/'.cargo/config.toml').read_text())['source']
    known={(p['name'],p['version'],p['source']) for p in inventory}
    for pkg in sorted(packages.values(), key=lambda x: x['id']):
        source = pkg['source']
        if (pkg['name'],pkg['version'],source) in known:continue
        group = 'registry' if source.startswith('registry+') else 'git-' + hashlib.sha256(source.encode()).hexdigest()[:12]
        name = pkg['name'] + '-' + pkg['version']
        src = Path(pkg['manifest_path']).parent
        dst = out / 'vendor/rust' / group / name
        dst.parent.mkdir(parents=True, exist_ok=True)
        if source.startswith('registry+'):
            crate = src.parent.parent.parent/'cache'/src.parent.name/(name+'.crate')
            digest=checksums[(pkg['name'],pkg['version'],source)]
            assert hashlib.sha256(crate.read_bytes()).hexdigest()==digest,'Registry archive differs from Cargo.lock'
            dst.mkdir()
            with tarfile.open(crate,'r:gz') as archive:
                for item in archive.getmembers():
                    if not item.isfile(): continue
                    relative=Path(item.name)
                    assert relative.parts[0]==name and '..' not in relative.parts
                    target=dst/Path(*relative.parts[1:])
                    assert target.resolve().is_relative_to(dst.resolve())
                    target.parent.mkdir(parents=True,exist_ok=True)
                    target.write_bytes(archive.extractfile(item).read())
            filesums={x.relative_to(dst).as_posix():hashlib.sha256(x.read_bytes()).hexdigest()
                      for x in dst.rglob('*') if x.is_file()}
            (dst/'.cargo-checksum.json').write_text(json.dumps({'files':filesums,'package':digest}))
            root = dst
            sources['crates-io'] = {'replace-with': 'release-registry'}
            sources['release-registry'] = {'directory': 'vendor/rust/registry'}
        elif source.startswith('git+'):
            root = git_root(src)
            subprocess.run(['git', '-C', str(root), 'diff', '--exit-code', '--quiet'], check=True)
            revision = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
            assert revision == source.split('#')[1], 'Git source revision differs from metadata'
            files = subprocess.check_output(['git', '-C', str(root), 'ls-files', '-z'], text=True).split('\0')
            dst.mkdir()
            for name_in_repo in filter(None, files):
                original = root / name_in_repo
                if original.is_file() and original.is_relative_to(src):
                    relative = original.relative_to(src)
                    target = dst / relative
                    target.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(original, target)
            manifest = (src / 'Cargo.toml').read_text()
            (dst / 'Cargo.toml.orig').write_text(manifest)
            (dst / 'Cargo.toml').write_text(normalize(pkg, source, root, manifest))
            filesums = {x.relative_to(dst).as_posix(): hashlib.sha256(x.read_bytes()).hexdigest()
                        for x in sorted(dst.rglob('*')) if x.is_file()}
            (dst / '.cargo-checksum.json').write_text(json.dumps({'files': filesums, 'package': None}))
            source_key = source.split('#')[0]
            sources[source_key] = {**git_details(source), 'replace-with': group}
            sources[group] = {'directory': 'vendor/rust/' + group}
        else:
            raise ValueError('Unexpected source ' + source)
        notice = notices / group / name
        notice.mkdir(parents=True)
        for directory in dict.fromkeys([dst, root]):
            for item in directory.iterdir():
                if item.is_file() and item.name.upper().startswith(('LICENSE', 'COPYING', 'COPYRIGHT', 'NOTICE', 'AUTHORS')):
                    target = notice / item.name
                    if not target.exists(): shutil.copyfile(item, target)
        inventory.append({'name': pkg['name'], 'version': pkg['version'], 'source': source,
                          'license': pkg['license'], 'directory': dst.relative_to(out).as_posix()})
    (out / '.cargo').mkdir(exist_ok=args.extend)
    (out / '.cargo/config.toml').write_text(toml({'source': sources, 'net': {'offline': True}}))
    (out / 'DEPENDENCIES.json').write_text(json.dumps(inventory, indent=2) + '\n')
    print(f'Bundled {len(packages)} pinned packages by source; retained upstream notices')


if __name__ == '__main__':
    main()
