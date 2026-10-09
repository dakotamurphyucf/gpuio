#!/usr/bin/env python3
"""Collect notices from an explicit installed switch and named vendored sources.

This is a conservative audit input, not a binary link inventory or license approval.
No install, update, switch selection or network command is performed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

from collect_rust_notices import NOTICE_NAME

ROOT = Path(__file__).resolve().parent.parent
IDENTIFIER = re.compile(r'^[A-Za-z0-9][A-Za-z0-9+_.~-]*$')


def digest(data):
    return hashlib.sha256(data).hexdigest()


def installed_packages(raw):
    result = []
    names = set()
    for line in raw.decode('utf-8').splitlines():
        if not line.strip():
            continue
        fields = [value.strip() for value in line.split('|')]
        if len(fields) != 2 or not all(IDENTIFIER.fullmatch(value) for value in fields):
            raise ValueError(f'invalid installed package row: {line!r}')
        name, version = fields
        if name in names:
            raise ValueError(f'duplicate installed package: {name}')
        names.add(name)
        result.append((name, version))
    if not result:
        raise ValueError('empty installed package inventory')
    return sorted(result)


def discover(directory):
    """Return exact local bytes; do not follow symlinks or infer parent notices."""
    issues, files = [], []
    if directory.is_symlink():
        return [], [f'symlink source directory: {directory}']
    if not directory.is_dir():
        return [], [f'missing source directory: {directory}']

    def fail(error):
        raise error

    for current, directories, filenames in os.walk(directory, followlinks=False, onerror=fail):
        current = Path(current)
        kept = []
        for name in sorted(directories):
            path = current / name
            if path.is_symlink():
                issues.append(f'skipped symlink directory: {path.relative_to(directory)}')
            elif name not in ('.git', '_build', 'target', 'scratch', '.opam-root'):
                kept.append(name)
        directories[:] = kept
        for name in sorted(filenames):
            path = current / name
            if not any(NOTICE_NAME.match(part) for part in path.relative_to(directory).parts):
                continue
            relative = str(path.relative_to(directory))
            if path.is_symlink():
                issues.append(f'symlink notice: {relative}')
                continue
            if not path.is_file():
                issues.append(f'non-file notice: {relative}')
                continue
            data = path.read_bytes()
            if not data:
                issues.append(f'empty notice: {relative}')
                continue
            files.append((relative, data))
    return files, issues


def package_record(kind, name, version, manifest, license_expression, locations):
    if manifest.is_symlink() or not manifest.is_file():
        raise ValueError(f'missing or symlink package metadata: {manifest}')
    metadata = manifest.read_bytes()
    identity = digest(f'{kind}\0{name}\0{version}\0{manifest}'.encode())
    record = {'kind': kind, 'name': name, 'version': version,
              'manifest_path': str(manifest), 'manifest_sha256': digest(metadata),
              'license_field': license_expression, 'notices': [], 'review_issues': []}
    outputs = [(f'packages/{identity}/opam', metadata)]
    record['manifest_file'] = outputs[0][0]
    for label, directory in locations:
        files, issues = discover(directory)
        record['review_issues'].extend(f'{label}: {issue}' for issue in issues)
        for relative, data in files:
            output = f'notices/{identity}/{label}/{relative}'
            outputs.append((output, data))
            record['notices'].append({'origin': label, 'source_directory': str(directory),
                                     'source_path': relative, 'file': output,
                                     'sha256': digest(data), 'bytes': len(data)})
    if not license_expression:
        record['review_issues'].append('missing declared license field')
    if not record['notices']:
        record['review_issues'].append('no collected notice text; classify metadata-only packages explicitly')
    return record, outputs


def write_inventory(output, records, files, evidence):
    if output.exists():
        raise ValueError(f'output already exists: {output}')
    # Validate every destination before creating even a partial inventory.
    seen = set()
    for name, _ in files:
        path = Path(name)
        if path.is_absolute() or '..' in path.parts or name in seen:
            raise ValueError(f'unsafe or duplicate output: {name}')
        seen.add(name)
    report = {**evidence, 'schema_version': 1, 'license_review_complete': False,
              'scope': 'All installed packages (including tools/build dependencies), plus explicit vendor roots; not exact linked content.',
              'packages': records,
              'packages_without_collected_text': sum(not row['notices'] for row in records),
              'remaining_review': ['Review applicability, copyright, license alternatives and additional notices for every package.',
                                   'Classify compiler configuration/virtual packages; absence of text is not automatic approval.',
                                   'Source/doc trees are local audit inputs, not verified unmodified upstream archives.',
                                   'Audit native/system libraries, assets and each shipped consumer separately.']}
    output.mkdir(parents=True)
    for name, data in files:
        path = output / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    (output / 'inventory.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--opam-root', required=True, type=Path)
    parser.add_argument('--switch', required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--vendor', action='append', default=[], type=Path,
                        help='Relative vendor directory under source-root; repeat as needed')
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError(f'output already exists: {args.output}')
    source_root = args.source_root.resolve()
    common = ['--root', str(args.opam_root.resolve()), '--switch', args.switch, '--color=never']
    commands = []

    def opam(command, *arguments):
        argv = ['opam', command, *common, *arguments]
        commands.append(argv)
        return subprocess.run(argv, check=True, stdout=subprocess.PIPE).stdout

    prefix = Path(opam('var', 'prefix').decode().strip())
    if not prefix.is_absolute() or not prefix.is_dir():
        raise ValueError(f'invalid installed switch prefix: {prefix}')
    raw = opam('list', '--installed', '--columns=name,version', '--separator=|', '--short')
    packages = installed_packages(raw)
    records, files = [], [('opam-installed.txt', raw)]
    for name, version in packages:
        identity = f'{name}.{version}'
        manifest = prefix / '.opam-switch/packages' / identity / 'opam'
        license_expression = opam('show', '--just-file', str(manifest), '--field=license', '--normalise').decode().strip()
        record, texts = package_record('installed', name, version, manifest, license_expression,
                                      [('installed-doc', prefix / 'doc' / name),
                                       ('source-tree', prefix / '.opam-switch/sources' / identity)])
        records.append(record)
        files.extend(texts)
    for relative in args.vendor:
        if relative.is_absolute() or '..' in relative.parts or relative == Path('.'):
            raise ValueError(f'unsafe vendor root: {relative}')
        directory = source_root / relative
        if directory.is_symlink() or not directory.resolve().is_relative_to(source_root):
            raise ValueError(f'symlink/outside vendor root: {relative}')
        manifests = sorted(directory.glob('*.opam'))
        if len(manifests) != 1:
            raise ValueError(f'vendor root must have exactly one opam manifest: {relative}')
        manifest = manifests[0]
        license_expression = opam('show', '--just-file', str(manifest), '--field=license', '--normalise').decode().strip()
        record, texts = package_record('vendor', str(relative), None, manifest, license_expression,
                                      [('vendor-source', directory)])
        records.append(record)
        files.extend(texts)
    source_inputs = []
    for relative in ('gpuio.opam', 'gpuio.opam.locked', 'dune-project', 'third_party/sources.json'):
        path = source_root / relative
        if path.is_symlink() or not path.is_file():
            raise ValueError(f'missing or symlink project provenance: {path}')
        data = path.read_bytes()
        output = f'project/{relative}'
        files.append((output, data))
        source_inputs.append({'source_path': relative, 'file': output, 'sha256': digest(data)})
    final_list = opam('list', '--installed', '--columns=name,version', '--separator=|', '--short')
    if installed_packages(final_list) != packages:
        raise ValueError('installed switch changed during notice collection; no output was created')
    evidence = {'opam_root': str(args.opam_root.resolve()), 'switch': args.switch,
                'switch_prefix': str(prefix), 'opam_version': subprocess.run(['opam', '--version'], check=True, stdout=subprocess.PIPE).stdout.decode().strip(),
                'commands': commands, 'source_inputs': source_inputs,
                'installed_list_sha256': digest(raw)}
    report = write_inventory(args.output, records, files, evidence)
    print(f"Collected {len(records)} installed/vendor packages; {report['packages_without_collected_text']} have no collected text. All require review.")


if __name__ == '__main__':
    main()
