#!/usr/bin/env python3
"""Collect review inputs, not license approval or an exact static-link inventory."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


NOTICE_NAME = re.compile(
    r'^(licen[sc]es?|notices?|copying|copyright|authors|credits|acknowledg[e]?ments)'
    r'(?:$|[._-])', re.I)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def load_supplemental(path, source_root):
    """Load explicitly attributed texts; never guess a parent license."""
    raw = path.read_bytes()
    document = json.loads(raw)
    if document['schema_version'] != 1:
        raise ValueError('unsupported supplemental notice schema')
    source_root = source_root.resolve()
    entries = []
    keys = set()
    for entry in document['entries']:
        selector = entry['package']
        key = (selector['name'], selector['version'], selector['source'])
        if key in keys:
            raise ValueError(f'duplicate supplemental package: {key}')
        keys.add(key)
        if not entry['rationale'].strip() or not entry['files']:
            raise ValueError(f'missing supplemental rationale/texts: {key}')
        files = []
        for item in entry['files']:
            relative = Path(item['path'])
            resolved = source_root / relative
            if relative.is_absolute() or '..' in relative.parts:
                raise ValueError(f'unsafe supplemental path: {relative}')
            if any(p.is_symlink() for p in [resolved, *resolved.parents]
                   if p.is_relative_to(source_root)):
                raise ValueError(f'symlink supplemental path: {relative}')
            data = resolved.read_bytes()
            if not data or digest(data) != item['sha256']:
                raise ValueError(f'supplemental notice hash/contents mismatch: {relative}')
            if not item['provenance'].strip():
                raise ValueError(f'missing supplemental provenance: {relative}')
            files.append((item, data))
        entries.append((entry, files))
    return {'sha256': digest(raw), 'entries': entries, 'raw': raw}


def dependency_closure(metadata, roots):
    """Keep normal/build edges, including proc macros; exclude dev-only edges."""
    packages = {p['id']: p for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    pending = []
    for name in roots:
        matches = [p['id'] for p in packages.values() if p['name'] == name]
        if len(matches) != 1:
            raise ValueError(f'root must identify exactly one package: {name}')
        pending.extend(matches)
    selected = set()
    while pending:
        identity = pending.pop()
        if identity in selected:
            continue
        if identity not in packages or identity not in nodes:
            raise ValueError(f'incomplete resolution for {identity}')
        selected.add(identity)
        for edge in nodes[identity]['deps']:
            kinds = edge['dep_kinds']
            if not kinds or any(k['kind'] not in (None, 'build', 'dev') for k in kinds):
                raise ValueError(f'unknown dependency kind: {edge}')
            if any(k['kind'] in (None, 'build') for k in kinds):
                pending.append(edge['pkg'])
    return [packages[key] for key in sorted(selected)]


def notice_files(package):
    """Never infer a nested crate's license from a differently licensed parent."""
    directory = Path(package['manifest_path']).parent.resolve()
    found = set()
    issues = []
    def fail_walk(error):
        raise error

    for current, dirs, files in os.walk(directory, followlinks=False, onerror=fail_walk):
        # Build outputs and VCS internals are never notice inputs.
        dirs[:] = sorted(d for d in dirs if d not in ('.git', 'target', '_build',
                                                     'scratch', '.opam-root'))
        for name in files:
            path = Path(current) / name
            if any(NOTICE_NAME.match(part) for part in path.relative_to(directory).parts):
                found.add(path)
        for name in dirs[:]:
            path = Path(current) / name
            if path.is_symlink():
                issues.append(f'symlink directory not followed: {path.relative_to(directory)}')
                dirs.remove(name)
    declared = package.get('license_file')
    if declared:
        path = Path(declared)
        if not path.is_absolute():
            path = directory / path
        # Keep external declarations visible for manual collection, not implicit reads.
        if not path.resolve().is_relative_to(directory):
            issues.append(f'declared license_file outside package: {declared}')
        else:
            found.add(path)
    regular = []
    for path in sorted(found):
        if path.is_symlink() or not path.resolve().is_relative_to(directory):
            issues.append(f'symlink notice not copied: {path}')
        elif not path.is_file():
            issues.append(f'missing declared notice: {path}')
        else:
            regular.append(path)
    if not regular:
        issues.append('no package-local license/notice text found; review upstream/workspace source')
    if not package.get('license') and not declared:
        issues.append('no declared license expression or license_file')
    return directory, regular, issues


def collect(metadata, roots, output, supplemental=None):
    packages = dependency_closure(metadata, roots)
    inputs = [(package, notice_files(package)) for package in packages]
    supplemental = supplemental or {'sha256': None, 'entries': []}
    additions = {}
    unused = []
    # Check all applicable source/manifest pins before creating any output.
    for entry, files in supplemental['entries']:
        selector = entry['package']
        matches = [p for p in packages if all(p[k] == selector[k]
                   for k in ('name', 'version', 'source'))]
        if not matches:
            unused.append(selector)
            continue
        if len(matches) != 1:
            raise ValueError(f'ambiguous supplemental package: {selector}')
        package = matches[0]
        if (package.get('license') != selector['license_expression'] or
                digest(Path(package['manifest_path']).read_bytes()) != selector['manifest_sha256']):
            raise ValueError(f'stale supplemental declaration/manifest: {selector["name"]}')
        additions[package['id']] = (entry, files)
    output.mkdir(parents=True, exist_ok=False)
    rows = []
    for package, (directory, files, issues) in inputs:
        identity = package['id']
        key = digest(identity.encode())
        notices = []
        for path in files:
            relative = path.relative_to(directory)
            destination = Path('notices') / key / relative
            data = path.read_bytes()
            target = output / destination
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            notices.append({'source_path': str(relative), 'file': str(destination),
                            'sha256': digest(data), 'bytes': len(data)})
            if not data:
                issues.append(f'empty notice text: {relative}')
        extra_notices = []
        if identity in additions:
            entry, extra_files = additions[identity]
            for item, data in extra_files:
                destination = Path('supplemental') / key / item['path']
                target = output / destination
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                extra_notices.append({**item, 'file': str(destination), 'bytes': len(data),
                                      'rationale': entry['rationale']})
        rows.append({'id': identity, 'name': package['name'], 'version': package['version'],
                     'source': package['source'], 'manifest_path': package['manifest_path'],
                     'manifest_sha256': digest(Path(package['manifest_path']).read_bytes()),
                     'license_expression': package.get('license'),
                     'license_file': package.get('license_file'),
                     'notices': notices, 'supplemental_notices': extra_notices,
                     'review_issues': issues})
    return {'schema_version': 2, 'license_review_complete': False,
            'scope': 'conservative resolved normal/build closure; not exact linked contents',
            'roots': roots, 'packages': rows,
            'packages_with_review_issues': sum(bool(p['review_issues']) for p in rows),
            'packages_with_supplemental_text': len(additions),
            'packages_without_collected_text': sum(
                not p['notices'] and not p['supplemental_notices'] for p in rows),
            'supplemental_manifest_sha256': supplemental['sha256'],
            'unused_supplemental_packages': unused,
            'remaining_review': [
                'Review every package expression and text; no license alternative is selected.',
                'Resolve missing texts and parent/workspace license declarations explicitly.',
                'Review embedded/vendored assets, generated code and native/system dependencies.',
                'Collect OCaml runtime/library and first-party notices separately.',
                'Repeat for each independent consumer lockfile, platform and feature selection.',
                'Cargo workspace feature unification may include more than the shipped binary.',
                'Target-filtered metadata can retain non-target normal/build kinds on dev-admitted edges.',
            ]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest-path', type=Path, default=Path('Cargo.toml'))
    parser.add_argument('--target', required=True)
    parser.add_argument('--root', action='append', required=True)
    parser.add_argument('--features', action='append', default=[])
    parser.add_argument('--no-default-features', action='store_true')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--supplemental', type=Path,
                        help='explicit attribution manifest; file paths are relative to this repository')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('--output must be a new directory')
    command = ['cargo', 'metadata', '--offline', '--locked', '--format-version', '1',
               '--filter-platform', args.target, '--manifest-path', str(args.manifest_path.resolve())]
    for features in args.features:
        command += ['--features', features]
    if args.no_default_features:
        command.append('--no-default-features')
    raw = subprocess.check_output(command)
    metadata = json.loads(raw)
    # A lockfile and metadata snapshot bind the inventory to the resolved input.
    lockfile = Path(metadata['workspace_root']) / 'Cargo.lock'
    lock_bytes = lockfile.read_bytes()
    supplemental = (load_supplemental(args.supplemental, Path(__file__).resolve().parents[1])
                    if args.supplemental else None)
    report = collect(metadata, args.root, args.output, supplemental)
    report.update({'command': command, 'target': args.target,
                   'cargo_version': subprocess.check_output(['cargo', '--version'], text=True).strip(),
                   'metadata_sha256': digest(raw), 'lockfile_sha256': digest(lock_bytes)})
    (args.output / 'cargo-metadata.json').write_bytes(raw)
    (args.output / 'Cargo.lock').write_bytes(lock_bytes)
    if supplemental:
        (args.output / 'supplemental-sources.json').write_bytes(supplemental['raw'])
    (args.output / 'inventory.json').write_text(json.dumps(report, indent=2) + '\n')
    print(f"Collected {len(report['packages'])} packages; "
          f"{report['packages_without_collected_text']} have no collected text; "
          f"{report['packages_with_supplemental_text']} have explicit supplemental texts. "
          "All require license review.")


if __name__ == '__main__':
    main()
