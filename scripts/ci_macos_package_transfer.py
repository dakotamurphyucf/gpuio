#!/usr/bin/env python3
"""Bind internal macOS runtime-test archives to a CI source checkpoint.

The build job supplies provenance; a hash of an executable alone cannot infer
its source revision. This stages test artifacts, never a reviewed release.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile

from package_macos_reference import APPS, ROOT, digest, package
from test_macos_package_runtime import inspect_package


def check_revision(revision):
    if not re.fullmatch(r'[0-9a-f]{40}', revision):
        raise ValueError('Expected a complete Git source revision')
    actual = subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', 'HEAD'], text=True).strip()
    dirty = subprocess.check_output(['git', '-C', str(ROOT), 'status', '--porcelain'], text=True)
    if actual != revision or dirty:
        raise ValueError('Package transfer requires the expected clean source checkout')


def stage(output, revision):
    check_revision(revision)
    output.mkdir(parents=True, exist_ok=False)
    manifest = {'schema_version': 1, 'complete': False, 'source_revision': revision,
                'source_scope': 'build-job checkout; build job must build these executables at this revision',
                'github_run_id': os.environ.get('GITHUB_RUN_ID'),
                'github_run_attempt': os.environ.get('GITHUB_RUN_ATTEMPT'),
                'producer_platform': platform.platform(),
                'qualification': 'internal ad-hoc runtime inputs; notices incomplete; not release artifacts',
                'applications': {}}
    try:
        with tempfile.TemporaryDirectory(prefix='gpuio-transfer-build-') as temporary:
            work = Path(temporary)
            notices = work / 'notices'
            notices.mkdir()
            shutil.copyfile(ROOT / 'LICENSE', notices / 'LICENSE')
            (notices / 'QUALIFICATION-ONLY.txt').write_text(
                'Internal runtime qualification only. Incomplete notices; do not distribute as a release.\n')
            for app in APPS:
                binary = ROOT / '_build/default/examples' / app / 'main.exe'
                assembled = work / app
                report = package(app, binary, assembled, notices, 'ad-hoc')
                destination = output / app
                destination.mkdir()
                for name in ['package.json', report['archive']]:
                    shutil.copyfile(assembled / name, destination / name)
                manifest['applications'][app] = {
                    'package_sha256': digest(destination / 'package.json'),
                    'archive_sha256': report['archive_sha256'],
                    'executable_sha256': report['packaged_executable_sha256'],
                }
                # Bound temporary disk usage to one uncompressed bundle.
                shutil.rmtree(assembled)
        check_revision(revision)
        manifest['complete'] = True
        (output / 'transfer.json').write_text(json.dumps(manifest, indent=2) + '\n')
        verify(output, revision)
    except BaseException as error:
        manifest['complete'] = False
        manifest['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (output / 'transfer.json').write_text(json.dumps(manifest, indent=2) + '\n')
    return manifest


def verify(directory, revision):
    manifest = json.loads((directory / 'transfer.json').read_text())
    if (manifest['schema_version'] != 1 or manifest['complete'] is not True
            or manifest['source_revision'] != revision
            or set(manifest['applications']) != set(APPS)):
        raise ValueError('Wrong source revision or incomplete package transfer')
    for field, key in [('github_run_id', 'GITHUB_RUN_ID'), ('github_run_attempt', 'GITHUB_RUN_ATTEMPT')]:
        if os.environ.get(key) and manifest.get(field) != os.environ[key]:
            raise ValueError('Package transfer belongs to a different CI run/attempt')
    observations = {}
    for app, expected in manifest['applications'].items():
        location = directory / app
        if location.is_symlink() or (location / 'package.json').is_symlink():
            raise ValueError('Transfer package paths must not be symlinks')
        if digest(location / 'package.json') != expected['package_sha256']:
            raise ValueError(f'Transferred package metadata hash mismatch: {app}')
        report, _ = inspect_package(location)
        if (report['app'] != app or report['revision'] != revision or report['dirty']
                or report['archive_sha256'] != expected['archive_sha256']
                or report['packaged_executable_sha256'] != expected['executable_sha256']):
            raise ValueError(f'Transferred package identity/provenance mismatch: {app}')
        observations[app] = expected
    return {'source_revision': revision, 'verified': observations,
            'qualification': manifest['qualification']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=('stage', 'verify'))
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}', args.revision):
        parser.error('Expected a complete Git revision')
    operation = stage if args.operation == 'stage' else verify
    print(json.dumps(operation(args.directory.resolve(), args.revision), indent=2))


if __name__ == '__main__':
    main()
