#!/usr/bin/env python3
"""Test the real AppKit event adapter in an isolated copy of the pinned crate."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    if sys.platform != 'darwin':
        raise SystemExit('AppKit event conversion requires macOS')
    root = Path(__file__).resolve().parent.parent
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    crate = output / 'gpui-macos'
    vendor = root / 'vendor/gpui-macos'
    shutil.copytree(vendor, crate, ignore=shutil.ignore_patterns('target', 'Cargo.lock'))
    sources = {str(p.relative_to(crate)): hashlib.sha256(p.read_bytes()).hexdigest()
               for p in crate.rglob('*') if p.is_file()}
    # Excluded vendor crates cannot run their dev-dependencies through `-p` in
    # the main workspace. Keep source bytes intact, with a local manifest/lock.
    workspace = tomllib.loads((root / 'Cargo.toml').read_text())
    manifest = (crate / 'Cargo.toml').read_text()
    # Cargo resolves all features of a workspace root into its lockfile, even
    # disabled ones. This test harness does not expose benchmark support: its
    # optional Criterion graph is absent from the application's pinned lock.
    manifest = '\n'.join(line for line in manifest.split('\n')
                         if not line.startswith('bench-support = '))
    manifest += '\n[workspace]\n\n[profile.dev]\ndebug = 0\n\n[profile.dev.package."*"]\nopt-level = 1\n'
    for registry, entries in workspace['patch'].items():
        manifest += '\n[patch.' + json.dumps(registry) + ']\n'
        for name, entry in entries.items():
            path = crate if name == 'gpui_macos' else root / entry['path']
            manifest += name + ' = { path = ' + json.dumps(str(path)) + ' }\n'
    (crate / 'Cargo.toml').write_text(manifest)
    shutil.copy2(root / 'Cargo.lock', crate / 'Cargo.lock')
    command = [str(root / 'scripts/gpuio'), 'exec', 'cargo', 'test', '--offline',
               '-j', os.environ.get('GPUIO_JOBS', '2'), '--manifest-path', str(crate / 'Cargo.toml'),
               '--features', 'runtime_shaders', '--lib', 'events::scroll_phase_tests']
    report = {'command': command, 'source_sha256': sources,
              'scope': 'AppKit factory and production converter; not WindowServer or hardware input'}
    with (output / 'cargo.log').open('w') as log:
        result = subprocess.run(command, cwd=root, stdout=log, stderr=subprocess.STDOUT)
    report['exit'] = result.returncode
    # Local resolution may prune unrelated workspace packages, but must not
    # change any registry/git dependency version or source pin.
    def dependencies(path):
        return {(p['name'], p['version'], p['source'], p.get('checksum'))
                for p in tomllib.loads(path.read_text())['package'] if 'source' in p}
    unexpected = dependencies(crate / 'Cargo.lock') - dependencies(root / 'Cargo.lock')
    report['unexpected_dependencies'] = sorted(unexpected)
    report['copied_sources_unchanged'] = all(
        hashlib.sha256((crate / name).read_bytes()).hexdigest() == digest
        for name, digest in sources.items() if name != 'Cargo.toml')
    report['complete'] = result.returncode == 0 and not unexpected and report['copied_sources_unchanged']
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    if not report['complete']:
        raise SystemExit('Native scroll conversion failed; inspect ' + str(output / 'report.json'))
    print('MACOS_SCROLL_PHASES_OK: AppKit start/move/end/cancel, both axes and zero deltas')


if __name__ == '__main__':
    main()
