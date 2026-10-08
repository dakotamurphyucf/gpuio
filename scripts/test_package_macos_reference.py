#!/usr/bin/env python3
"""Portable input-admission tests for the reference packager; no OS launch/signing."""
import plistlib
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import macos_signing
import package_macos_reference as packager
from package_macos_reference import check_binary, check_metadata, copy_notices

LOADS = '/tmp/app:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1345.100.2)'
COMMANDS = '      cmd LC_BUILD_VERSION\n platform 1\n    minos 14.4\n      sdk 14.4\n'

IDENTITY = 'A1' * 20
TEAM = 'AB12345678'
IDENTIFIER = 'com.gpuio.agent-chat'
DETAILS = (f'Identifier={IDENTIFIER}\nCodeDirectory v=20500 size=123 flags=0x10000(runtime)\n'
           f'TeamIdentifier={TEAM}\nTimestamp=Oct 8, 2026 at 12:00:00 PM\n')


class DeveloperIdSigning(unittest.TestCase):
    @unittest.skipUnless(platform.system() == 'Darwin', 'Requires native codesign/csreq')
    def test_native_requirement_parses_and_rejects_an_adhoc_bundle(self):
        constraint = macos_signing.requirement(IDENTITY, TEAM, IDENTIFIER)
        subprocess.run(['/usr/bin/csreq', '-r', '=' + constraint, '-t'],
                       check=True, capture_output=True, timeout=30)
        with tempfile.TemporaryDirectory(prefix='gpuio-signing-fixture-') as temporary:
            bundle = Path(temporary) / 'Fixture.app'
            (bundle / 'Contents/MacOS').mkdir(parents=True)
            executable = bundle / 'Contents/MacOS/fixture'
            shutil.copyfile('/usr/bin/true', executable)
            executable.chmod(0o700)
            (bundle / 'Contents/Info.plist').write_bytes(plistlib.dumps({
                'CFBundleExecutable': 'fixture', 'CFBundleIdentifier': IDENTIFIER,
                'CFBundlePackageType': 'APPL'}))
            macos_signing.run('/usr/bin/codesign', '--force', '--sign', '-',
                             '--options', 'runtime', '--timestamp=none',
                             '--identifier', IDENTIFIER, str(bundle))
            # Verify command syntax against the real tool; synthetic parser tests
            # alone could mistake a command-line error for certificate rejection.
            macos_signing.run('/usr/bin/codesign', '--display', '--verbose=4',
                             '--architecture', platform.machine(), str(bundle))
            with self.assertRaises(subprocess.CalledProcessError) as rejected:
                macos_signing.verify(bundle, IDENTITY, TEAM, IDENTIFIER)
            self.assertIn('failed to satisfy specified code requirement', rejected.exception.output)

    def test_invalid_options_fail_before_snapshot_or_signing(self):
        cases = [('developer-id', None, TEAM), ('developer-id', '-', TEAM),
                 ('developer-id', IDENTITY, None), ('developer-id', IDENTITY, 'bad"team'),
                 ('ad-hoc', IDENTITY, TEAM), ('unsigned', None, TEAM), ('typo', None, None)]
        for mode, identity, team in cases:
            with self.subTest(mode=mode, identity=identity, team=team), \
                    patch.object(packager.tempfile, 'TemporaryDirectory') as snapshot, \
                    patch.object(macos_signing, 'run') as run, self.assertRaises(ValueError):
                try:
                    packager.package('agent_chat', Path('/missing'), Path('/unused'),
                                     Path('/notices'), mode, identity=identity, team_id=team)
                finally:
                    snapshot.assert_not_called()
                    run.assert_not_called()

    def test_constraint_binds_apple_certificate_kind_leaf_team_and_app(self):
        constraint = macos_signing.requirement(IDENTITY, TEAM, IDENTIFIER)
        for part in ('anchor apple generic', '1.2.840.113635.100.6.2.6',
                     '1.2.840.113635.100.6.1.13', f'leaf = H"{IDENTITY}"',
                     f'subject.OU] = "{TEAM}"', f'identifier "{IDENTIFIER}"'):
            self.assertIn(part, constraint)
        with self.assertRaises(ValueError):
            macos_signing.requirement(IDENTITY, TEAM, 'app" or true')

    def test_missing_runtime_timestamp_or_wrong_identity_is_rejected(self):
        self.assertEqual(macos_signing.check_details(DETAILS, TEAM, IDENTIFIER)['TeamIdentifier'], TEAM)
        cases = [DETAILS.replace('0x10000(runtime)', '0x0(none)'),
                 DETAILS.replace('Timestamp=', 'Signed Time='),
                 DETAILS.replace('Oct 8, 2026 at 12:00:00 PM', 'none'),
                 DETAILS.replace(TEAM, 'ZZ12345678'), DETAILS.replace(IDENTIFIER, 'other.app'),
                 DETAILS + f'TeamIdentifier={TEAM}\n', DETAILS + 'Timestamp=duplicate\n']
        for details in cases:
            with self.subTest(details=details), self.assertRaises(ValueError):
                macos_signing.check_details(details, TEAM, IDENTIFIER)

    def test_verification_checks_runtime_and_timestamp_for_each_slice(self):
        with tempfile.TemporaryDirectory() as temporary:
            bundle = Path(temporary) / 'fixture.app'
            (bundle / 'Contents').mkdir(parents=True)
            (bundle / 'Contents/Info.plist').write_bytes(
                plistlib.dumps({'CFBundleExecutable': 'fixture'}))

            def run(*args):
                if args[0] == '/usr/bin/lipo':
                    return 'arm64 x86_64'
                if '--display' in args:
                    return DETAILS if args[args.index('--architecture') + 1] == 'arm64' else DETAILS.replace(
                        'Timestamp=', 'Signed Time=')
                self.assertIn('--all-architectures', args)
                self.assertIn('-R', args)
                self.assertTrue(args[args.index('-R') + 1].startswith('=anchor apple generic'))
                return ''

            with patch.object(macos_signing, 'run', side_effect=run), \
                    self.assertRaisesRegex(ValueError, 'Timestamp'):
                macos_signing.verify(bundle, IDENTITY, TEAM, IDENTIFIER)
            with patch.object(macos_signing, 'run', side_effect=['', 'arm64 x86_64', DETAILS, DETAILS]):
                self.assertEqual(set(macos_signing.verify(bundle, IDENTITY, TEAM, IDENTIFIER)),
                                 {'arm64', 'x86_64'})

    def test_signing_failure_leaves_incomplete_report_and_no_archive(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / 'fixture'
            binary.write_bytes(b'unsigned input')
            notices = root / 'notices'
            notices.mkdir()
            (notices / 'LICENSE').write_text('Fixture only')
            output = root / 'output'
            metadata = {'CFBundleName': 'GPUIO Agent Workspace',
                        'CFBundleIdentifier': IDENTIFIER, 'CFBundleExecutable': 'gpuio-agent-chat',
                        'CFBundlePackageType': 'APPL', 'CFBundleVersion': '1',
                        'CFBundleShortVersionString': '0.1.0',
                        'LSMinimumSystemVersion': '14.4', 'NSHighResolutionCapable': True}
            commands = []

            def run(*args):
                commands.append(args)
                if args[-1] == '--print-info-plist':
                    return plistlib.dumps(metadata).decode()
                if args[0] == 'git':
                    return 'fixture' if args[-1] == 'HEAD' else ''
                return {'-L': LOADS, '-l': COMMANDS, '-archs': 'arm64'}[args[1]]

            with patch.object(packager, 'run', side_effect=run), \
                    patch.object(macos_signing, 'sign', side_effect=ValueError('signature rejected')), \
                    self.assertRaisesRegex(ValueError, 'signature rejected'):
                packager.package_snapshot('agent_chat', binary, binary, output, notices,
                                          'developer-id', identity=IDENTITY, team_id=TEAM)
            report = json.loads((output / 'package.json').read_text())
            self.assertFalse(report['complete'])
            self.assertEqual(report['signing_identity'], IDENTITY)
            self.assertEqual(report['signing_team'], TEAM)
            self.assertIn('signature rejected', report['error'])
            self.assertFalse(list(output.glob('*.zip')))
            self.assertFalse(any(command[0] == '/usr/bin/ditto' for command in commands))
            self.assertEqual(binary.read_bytes(), b'unsigned input')


class BinaryAdmission(unittest.TestCase):
    def test_system_only_slice_and_universal_minimums(self):
        result = check_binary(LOADS, COMMANDS, 'arm64', '14.4')
        self.assertEqual(result['architectures'], ['arm64'])
        self.assertEqual(result['minimum_versions'], ['14.4'])
        result = check_binary(LOADS + '\n' + LOADS, COMMANDS * 2, 'arm64 x86_64', '14.4')
        self.assertEqual(result['system_dependencies'], ['/usr/lib/libSystem.B.dylib'])

    def test_unbundled_dependencies_and_search_paths_are_rejected(self):
        for dependency in ['/opt/homebrew/lib/libssl.dylib', '@rpath/plugin.dylib',
                           '@executable_path/../lib/plugin.dylib', '/usr/lib/../../private/plugin.dylib']:
            with self.subTest(dependency=dependency), self.assertRaises(ValueError):
                check_binary(LOADS.replace('/usr/lib/libSystem.B.dylib', dependency), COMMANDS, 'arm64', '14.4')
        with self.assertRaisesRegex(ValueError, 'search paths'):
            check_binary(LOADS, COMMANDS + ' cmd LC_RPATH\n', 'arm64', '14.4')

    def test_wrong_platform_missing_slice_and_higher_minimum_rejected(self):
        for commands, arches in [(COMMANDS.replace('minos 14.4', 'minos 15.0'), 'arm64'),
                                 (COMMANDS.replace('platform 1', 'platform 2'), 'arm64'),
                                 (COMMANDS, 'arm64 x86_64'), ('', 'arm64'),
                                 (COMMANDS, 'arm64 arm64'), (COMMANDS, 'i386')]:
            with self.subTest(commands=commands, arches=arches), self.assertRaises(ValueError):
                check_binary(LOADS, commands, arches, '14.4')

    def test_malformed_dependency_output_is_not_silently_skipped(self):
        for loads in ['/tmp/app:\n', LOADS + '\n\tunknown format']:
            with self.assertRaises(ValueError):
                check_binary(loads, COMMANDS, 'arm64', '14.4')


class BundleInputs(unittest.TestCase):
    def test_identity_and_executable_match_the_selected_reference(self):
        metadata = {'CFBundleName': 'GPUIO Agent Workspace',
                    'CFBundleIdentifier': 'com.gpuio.agent-chat',
                    'CFBundleExecutable': 'gpuio-agent-chat', 'CFBundlePackageType': 'APPL',
                    'CFBundleVersion': '1', 'CFBundleShortVersionString': '0.1.0',
                    'LSMinimumSystemVersion': '14.4', 'NSHighResolutionCapable': True}
        self.assertEqual(check_metadata(plistlib.loads(plistlib.dumps(metadata)), 'agent_chat'), metadata)
        for key, value in [('CFBundleExecutable', '../escape'), ('CFBundleIdentifier', 'other.app'),
                           ('LSMinimumSystemVersion', '13.0'), ('CFBundleVersion', 'bad')]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                check_metadata({**metadata, key: value}, 'agent_chat')

    def test_notice_bytes_are_preserved_hashed_and_symlinks_rejected(self):
        with tempfile.TemporaryDirectory(prefix='gpuio-package-notices-') as temporary:
            root = Path(temporary)
            source = root / 'source'
            source.mkdir()
            with self.assertRaisesRegex(ValueError, 'empty'):
                copy_notices(source, root / 'empty-target')
            (source / 'library').mkdir()
            (source / 'library/LICENSE').write_bytes(b'Original copyright\n')
            manifest = copy_notices(source, root / 'target')
            self.assertEqual((root / 'target/library/LICENSE').read_bytes(), b'Original copyright\n')
            self.assertEqual(len(manifest['library/LICENSE']), 64)
            (source / 'link').symlink_to(source / 'library/LICENSE')
            with self.assertRaisesRegex(ValueError, 'symlinks'):
                copy_notices(source, root / 'rejected')
            self.assertFalse((root / 'rejected').exists())


class PackagingSnapshot(unittest.TestCase):
    def test_concurrent_rebuild_cannot_replace_the_audited_packaged_bytes(self):
        with tempfile.TemporaryDirectory(prefix='gpuio-package-race-') as temporary:
            root = Path(temporary)
            binary = root / 'main.exe'
            original = b'original executable fixture'
            binary.write_bytes(original)
            binary.chmod(0o700)
            notices = root / 'notices'
            notices.mkdir()
            (notices / 'LICENSE').write_text('Fixture license\n')
            output = root / 'output'
            metadata = {'CFBundleName': 'GPUIO Agent Workspace',
                        'CFBundleIdentifier': 'com.gpuio.agent-chat',
                        'CFBundleExecutable': 'gpuio-agent-chat', 'CFBundlePackageType': 'APPL',
                        'CFBundleVersion': '1', 'CFBundleShortVersionString': '0.1.0',
                        'LSMinimumSystemVersion': '14.4', 'NSHighResolutionCapable': True}
            inspected = []

            def run(*args):
                if args[-1] == '--print-info-plist':
                    inspected.append(Path(args[0]))
                    self.assertNotEqual(inspected[0], binary)
                    self.assertEqual(inspected[0].read_bytes(), original)
                    # Model the build tool atomically replacing its live output.
                    replacement = root / 'replacement.exe'
                    replacement.write_bytes(b'rebuilt, uninspected executable')
                    replacement.replace(binary)
                    return plistlib.dumps(metadata).decode()
                if args[0] in ('/usr/bin/otool', '/usr/bin/lipo'):
                    self.assertEqual(Path(args[-1]), inspected[0])
                    self.assertEqual(Path(args[-1]).read_bytes(), original)
                    return {'-L': LOADS, '-l': COMMANDS, '-archs': 'arm64'}[args[1]]
                if args[0] == 'git':
                    return 'fixture-revision' if args[-1] == 'HEAD' else ' M fixture'
                if args[0] == '/usr/bin/ditto':
                    Path(args[-1]).write_bytes(b'archive fixture')
                    return ''
                self.fail(f'Unexpected external command: {args}')

            with patch.object(packager.platform, 'system', return_value='Darwin'), \
                    patch.object(packager, 'run', side_effect=run):
                report = packager.package('agent_chat', binary, output, notices, 'unsigned')
            installed = output / report['bundle'] / 'Contents/MacOS/gpuio-agent-chat'
            self.assertEqual(installed.read_bytes(), original)
            self.assertNotEqual(binary.read_bytes(), original)
            self.assertFalse(inspected[0].exists(), 'private copy must be cleaned up')
            self.assertTrue(report['complete'])
            self.assertEqual(report['source_executable'], str(binary.resolve()))
            self.assertEqual(report['source_sha256'], hashlib.sha256(original).hexdigest())
            self.assertEqual(report['source_sha256'], report['packaged_executable_sha256'])
            self.assertEqual(json.loads((output / 'package.json').read_text()), report)
            provenance = json.loads((installed.parent.parent / 'Resources/Build.json').read_text())
            self.assertEqual(provenance['source_sha256'], report['source_sha256'])
            self.assertEqual(provenance['revision_scope'], report['revision_scope'])

    def test_failed_snapshot_inspection_cleans_up_without_creating_output(self):
        with tempfile.TemporaryDirectory(prefix='gpuio-package-inspect-') as temporary:
            root = Path(temporary)
            binary = root / 'main.exe'
            binary.write_bytes(b'fixture')
            binary.chmod(0o700)
            inspected = []

            def fail(*args):
                inspected.append(Path(args[0]))
                raise ValueError('fixture inspection failed')

            with patch.object(packager.platform, 'system', return_value='Darwin'), \
                    patch.object(packager, 'run', side_effect=fail), \
                    self.assertRaisesRegex(ValueError, 'inspection failed'):
                packager.package('agent_chat', binary, root / 'output', root / 'notices', 'unsigned')
            self.assertEqual(len(inspected), 1)
            self.assertFalse(inspected[0].exists())
            self.assertFalse((root / 'output').exists())
            self.assertEqual(binary.read_bytes(), b'fixture')


if __name__ == '__main__':
    unittest.main()
