#!/usr/bin/env python3
"""Portable artifact/isolation admission checks; launches no app or sandbox."""
import json
from pathlib import Path
import stat
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from package_macos_reference import digest
from test_macos_package_runtime import deny_profile, inspect_package, minimal_environment


class RuntimeInputs(unittest.TestCase):
    def test_developer_id_archive_requires_explicit_certificate_and_team(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report, archive = self.package(root)
            report['signing'] = 'developer-id'
            for identity, team in [(None, None), ('A1' * 20, None), ('-', 'AB12345678')]:
                report.update(signing_identity=identity, signing_team=team)
                (root / 'package.json').write_text(json.dumps(report))
                with self.assertRaises(ValueError):
                    inspect_package(root)
            report.update(signing_identity='A1' * 20, signing_team='AB12345678')
            (root / 'package.json').write_text(json.dumps(report))
            # Admission is not signature verification; extract verifies the real bundle.
            self.assertEqual(inspect_package(root), (report, archive))

    def package(self, root, extra=None):
        name = 'GPUIO Component Studio'
        archive = root / (name + '.zip')
        with zipfile.ZipFile(archive, 'w') as zipped:
            zipped.writestr(name + '.app/Contents/MacOS/gpuio-studio', b'executable fixture')
            if extra:
                zipped.writestr(*extra)
        report = {
            'complete': True, 'app': 'gallery', 'bundle': name + '.app',
            'signing': 'ad-hoc',
            'archive': archive.name, 'archive_sha256': digest(archive),
            'metadata': {'CFBundleName': name, 'CFBundleIdentifier': 'com.gpuio.component-studio',
                         'CFBundleExecutable': 'gpuio-studio', 'CFBundlePackageType': 'APPL',
                         'CFBundleVersion': '1', 'CFBundleShortVersionString': '0.1.0',
                         'LSMinimumSystemVersion': '14.4', 'NSHighResolutionCapable': True},
        }
        (root / 'package.json').write_text(json.dumps(report))
        return report, archive

    def test_archive_hash_and_complete_identity_are_required(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report, archive = self.package(root)
            self.assertEqual(inspect_package(root), (report, archive))
            archive.write_bytes(archive.read_bytes() + b'changed after assembly')
            with self.assertRaisesRegex(ValueError, 'hash/type'):
                inspect_package(root)
            report, _ = self.package(root)
            report['complete'] = False
            (root / 'package.json').write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, 'Incomplete'):
                inspect_package(root)

    def test_archive_cannot_escape_or_install_links_and_devices(self):
        symlink = zipfile.ZipInfo('GPUIO Component Studio.app/linked')
        symlink.create_system = 3
        symlink.external_attr = (stat.S_IFLNK | 0o777) << 16
        fifo = zipfile.ZipInfo('GPUIO Component Studio.app/pipe')
        fifo.create_system = 3
        fifo.external_attr = (stat.S_IFIFO | 0o600) << 16
        cases = [('../outside', b'bad'), ('/absolute', b'bad'),
                 ('GPUIO Component Studio.app/../../outside', b'bad'),
                 ('GPUIO Component Studio.app/Contents/./MacOS/gpuio-studio', b'duplicate'),
                 (symlink, b'/outside'), (fifo, b'')]
        for extra in cases:
            with self.subTest(entry=str(extra[0])), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                self.package(root, extra)
                with self.assertRaisesRegex(ValueError, 'Unsafe'):
                    inspect_package(root)

    def test_profile_escapes_paths_and_environment_drops_loader_overrides(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'quoted"path'
            profile = deny_profile([path, path])
            self.assertEqual(profile.count('(deny '), 1)
            self.assertIn('quoted\\"path', profile)
        for paths in [[], ['/'], ['/tmp/line\nbreak']]:
            with self.assertRaises(ValueError):
                deny_profile(paths)
        with patch.dict('os.environ', {'HOME': '/real-home', 'USER': 'test',
                                      'DYLD_LIBRARY_PATH': '/developer/lib',
                                      'CAML_LD_LIBRARY_PATH': '/developer/ocaml',
                                      'OPAM_SWITCH_PREFIX': '/unrelated/switch'}, clear=True):
            environment = minimal_environment()
            self.assertEqual(environment['HOME'], '/real-home')
            self.assertEqual(set(environment), {'HOME', 'USER', 'PATH'})


if __name__ == '__main__':
    unittest.main()
