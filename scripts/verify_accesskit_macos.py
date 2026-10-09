#!/usr/bin/env python3
"""Reconstruct the pinned macOS adapter from its verified registry archive.

No network access or repository mutation. Only upstream-listed regular files are
read from the archive; all scoped patches must reproduce the vendored sources.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def verify(archive, vendor, workspace):
    manifest = json.loads((vendor / 'UPSTREAM.json').read_text())
    if digest(archive.read_bytes()) != manifest['crate_sha256']:
        raise ValueError('Registry archive SHA256 differs from UPSTREAM.json')
    prefix = f"{manifest['crate']}-{manifest['version']}/"
    originals = manifest['original_files_sha256']
    with tarfile.open(archive) as bundle:
        for name, expected in originals.items():
            relative = Path(name)
            if relative.is_absolute() or '..' in relative.parts:
                raise ValueError(f'Invalid upstream file path: {name}')
            member = bundle.getmember(prefix + name)
            if not member.isfile() or member.size > 2 * 1024 * 1024:
                raise ValueError(f'Invalid upstream file: {name}')
            with bundle.extractfile(member) as source:
                data = source.read()
            if digest(data) != expected:
                raise ValueError(f'Upstream file SHA256 differs: {name}')
            target = workspace / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    for name in manifest['patches']:
        if Path(name).name != name:
            raise ValueError(f'Invalid patch name: {name}')
        result = subprocess.run(
            ['patch', '--batch', '--forward', '-p1', '-i', str(vendor / name)],
            cwd=workspace, text=True, capture_output=True, check=True,
        )
        # Offsets/fuzz hide drift in patch ordering; reconstruction must be exact.
        if 'offset' in result.stdout or 'fuzz' in result.stdout:
            raise ValueError(f'Patch applied with drift: {name}: {result.stdout}')
    for name in originals:
        if (workspace / name).read_bytes() != (vendor / name).read_bytes():
            raise ValueError(f'Reconstructed source differs: {name}')
    for name, expected in manifest['license_files_sha256'].items():
        if digest((vendor / name).read_bytes()) != expected:
            raise ValueError(f'License SHA256 differs: {name}')
    print(f"ACCESSKIT_RECONSTRUCTION_OK: {len(originals)} upstream files, "
          f"{len(manifest['patches'])} exact patches, "
          f"{len(manifest['license_files_sha256'])} license checksums")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True,
                        help='accesskit_macos 0.26.3 crate archive from UPSTREAM.json')
    args = parser.parse_args()
    vendor = Path(__file__).resolve().parent.parent / 'vendor' / 'accesskit-macos'
    with tempfile.TemporaryDirectory(prefix='gpuio-accesskit-verify-') as directory:
        verify(args.archive.resolve(), vendor, Path(directory))


if __name__ == '__main__':
    main()
