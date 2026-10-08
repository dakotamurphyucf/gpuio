#!/usr/bin/env python3
"""Assemble audited local macOS reference bundles, without launching or installing.

All signing modes produce qualification inputs, not notarized releases.
Supplied notices are included and hashed; completeness needs a license review.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import tempfile

import macos_signing

ROOT = Path(__file__).resolve().parent.parent
APPS = {
    'agent_chat': ('GPUIO Agent Workspace', 'com.gpuio.agent-chat', 'gpuio-agent-chat'),
    'gallery': ('GPUIO Component Studio', 'com.gpuio.component-studio', 'gpuio-studio'),
    'signal_studio': ('GPUIO Signal Studio', 'com.gpuio.signal-studio', 'gpuio-signal'),
}


def run(*arguments):
    return subprocess.check_output(arguments, text=True, stderr=subprocess.PIPE, timeout=60).strip()


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def version(value):
    if not isinstance(value, str) or not re.fullmatch(r'\d+(?:\.\d+){1,2}', value):
        raise ValueError(f'Invalid macOS version: {value!r}')
    return tuple(int(part) for part in value.split('.')) + (0,) * (3 - len(value.split('.')))


def check_metadata(metadata, app):
    name, identifier, executable = APPS[app]
    expected = {'CFBundleName': name, 'CFBundleIdentifier': identifier,
                'CFBundleExecutable': executable, 'CFBundlePackageType': 'APPL',
                'LSMinimumSystemVersion': '14.4', 'NSHighResolutionCapable': True}
    for key, value in expected.items():
        if metadata.get(key) != value:
            raise ValueError(f'Unexpected {key}: {metadata.get(key)!r}')
    for key in ('CFBundleVersion', 'CFBundleShortVersionString'):
        if not isinstance(metadata.get(key), str) or not re.fullmatch(r'\d+(?:\.\d+){0,2}', metadata[key]):
            raise ValueError(f'Missing/invalid {key}')
    return metadata


def check_binary(loads, commands, architectures, minimum):
    """Fail closed on non-system dylibs and ambiguous/unsupported Mach-O slices."""
    arches = architectures.split()
    if not arches or len(set(arches)) != len(arches) or not set(arches) <= {'arm64', 'x86_64'}:
        raise ValueError(f'Unsupported or ambiguous architectures: {architectures}')
    dependencies = []
    for line in loads.splitlines():
        if not line.startswith((' ', '\t')):
            continue  # otool's file/architecture heading
        match = re.fullmatch(r'\s+(.+) \(compatibility version [^,]+, current version [^)]+\)', line)
        if not match:
            raise ValueError(f'Unrecognized dependency: {line}')
        path = match[1]
        if (not path.startswith(('/System/Library/', '/usr/lib/'))
                or '..' in Path(path).parts):
            raise ValueError(f'External runtime dependency must be resolved before packaging: {path}')
        dependencies.append(path)
    if not dependencies:
        raise ValueError('No inspectable system dependencies')
    if re.search(r'\bcmd LC_RPATH\b', commands):
        raise ValueError('Unexpected runtime search paths; review linker inputs before packaging')
    platforms = re.findall(r'^\s*platform (\d+)\s*$', commands, re.MULTILINE)
    minimums = re.findall(r'^\s*minos ([\d.]+)\s*$', commands, re.MULTILINE)
    if len(minimums) != len(arches) or platforms != ['1'] * len(arches):
        raise ValueError('Each slice must declare one macOS LC_BUILD_VERSION')
    if any(version(value) > version(minimum) for value in minimums):
        raise ValueError('Mach-O deployment target exceeds declared application minimum')
    return {'architectures': arches, 'minimum_versions': minimums,
            'system_dependencies': sorted(set(dependencies))}


def copy_notices(source, destination):
    if not source.is_dir() or source.is_symlink():
        raise ValueError('Notices must be a regular directory')
    files = []
    for path in sorted(source.rglob('*')):
        if path.is_symlink():
            raise ValueError(f'Notice symlinks are not accepted: {path}')
        if path.is_dir():
            continue
        if not path.is_file():
            raise ValueError(f'Unsupported notice input: {path}')
        files.append(path)
    if not files:
        raise ValueError('Notices directory is empty')
    manifest = {}
    for path in files:
        relative = path.relative_to(source)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
        manifest[str(relative)] = digest(target)
    return manifest


def package(app, binary, output, notices, sign, *, identity=None, team_id=None):
    macos_signing.validate_options(sign, identity, team_id)
    if platform.system() != 'Darwin':
        raise ValueError('Native Mach-O inspection/signing requires macOS')
    binary = binary.resolve(strict=True)
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError('Expected a built executable')
    if output.resolve().is_relative_to(notices.resolve()):
        raise ValueError("Output must not be inside the notices input")
    if output.exists():
        raise ValueError('Choose a fresh output directory; existing artifacts are preserved')
    # Builds may replace the original while metadata and load commands are being
    # inspected. Audit and package one private snapshot, never reread live output.
    with tempfile.TemporaryDirectory(prefix='gpuio-package-input-') as temporary:
        snapshot = Path(temporary) / binary.name
        shutil.copyfile(binary, snapshot)
        snapshot.chmod(0o700)
        return package_snapshot(app, snapshot, binary, output, notices, sign,
                                identity=identity, team_id=team_id)


def package_snapshot(app, binary, source, output, notices, sign, *, identity=None, team_id=None):
    macos_signing.validate_options(sign, identity, team_id)
    metadata = check_metadata(plistlib.loads(run(str(binary), '--print-info-plist').encode()), app)
    audit = check_binary(run('/usr/bin/otool', '-L', str(binary)),
                         run('/usr/bin/otool', '-l', str(binary)),
                         run('/usr/bin/lipo', '-archs', str(binary)),
                         metadata['LSMinimumSystemVersion'])
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'app': app, 'source_executable': str(source),
              'source_sha256': digest(binary), 'metadata': metadata, 'binary_audit': audit,
              'revision_scope': 'packaging checkout; binary build revision is not attested',
              'signing': sign, 'license_review': 'required; supplied notices are not a completeness attestation',
              'qualification': 'not launched; no clean-machine, notarization or Gatekeeper acceptance'}
    try:
        report['revision'] = run('git', '-C', str(ROOT), 'rev-parse', 'HEAD')
        report['dirty'] = bool(run('git', '-C', str(ROOT), 'status', '--porcelain'))
        bundle = output / (APPS[app][0] + '.app')
        contents = bundle / 'Contents'
        installed = contents / 'MacOS' / metadata['CFBundleExecutable']
        installed.parent.mkdir(parents=True)
        shutil.copyfile(binary, installed)
        installed.chmod(0o755)
        with (contents / 'Info.plist').open('wb') as stream:
            plistlib.dump(metadata, stream, sort_keys=True)
        report['notices'] = copy_notices(notices, contents / 'Resources' / 'Notices')
        provenance = contents / 'Resources' / 'Build.json'
        provenance.write_text(json.dumps({key: report[key] for key in
                                         ('app', 'source_sha256', 'revision', 'dirty', 'revision_scope', 'binary_audit',
                                          'license_review', 'qualification')}, indent=2) + '\n')
        # OCaml's linker may have already ad-hoc signed the Mach-O executable.
        # "unsigned" means this command does not sign/attest the complete bundle.
        if sign == 'ad-hoc':
            run('/usr/bin/codesign', '--force', '--sign', '-', '--timestamp=none',
                '--identifier', metadata['CFBundleIdentifier'], str(bundle))
            run('/usr/bin/codesign', '--verify', '--strict', '--verbose=2', str(bundle))
        elif sign == 'developer-id':
            report.update(signing_identity=identity, signing_team=team_id)
            report['signature'] = macos_signing.sign(
                bundle, identity, team_id, metadata['CFBundleIdentifier'])
        report['packaged_executable_sha256'] = digest(installed)
        report['bundle'] = bundle.name
        archive = output / (APPS[app][0] + '.zip')
        run('/usr/bin/ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', str(bundle), str(archive))
        report['archive'] = archive.name
        report['archive_sha256'] = digest(archive)
        report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (output / 'package.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', required=True, choices=APPS)
    parser.add_argument('--executable', type=Path, help='Built app, including an independent consumer')
    parser.add_argument('--output', required=True, type=Path, help='Fresh artifact directory')
    parser.add_argument('--notices', required=True, type=Path, help='Prepared license/notice directory; completeness requires review')
    parser.add_argument('--sign', choices=('unsigned', 'ad-hoc', 'developer-id'), default='unsigned')
    parser.add_argument('--identity', help='Developer ID Application certificate SHA-1 fingerprint')
    parser.add_argument('--team-id', help='Apple Developer team ID; required with developer-id')
    args = parser.parse_args()
    binary = args.executable or ROOT / '_build/default/examples' / args.app / 'main.exe'
    report = package(args.app, binary, args.output, args.notices, args.sign,
                     identity=args.identity, team_id=args.team_id)
    print(f'REFERENCE_PACKAGE_ASSEMBLED app={args.app} archive={args.output / report["archive"]}')


if __name__ == '__main__':
    main()
