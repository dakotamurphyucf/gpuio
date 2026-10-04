#!/usr/bin/env python3
"""Portable input-admission tests for the reference packager; no OS launch/signing."""
import plistlib
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package_macos_reference as packager
from package_macos_reference import check_binary, check_metadata, copy_notices

LOADS = '/tmp/app:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1345.100.2)'
COMMANDS = '      cmd LC_BUILD_VERSION\n platform 1\n    minos 14.4\n      sdk 14.4\n'


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
