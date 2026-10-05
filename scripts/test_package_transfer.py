#!/usr/bin/env python3
"""Portable cross-job transfer checks; no signing, app or toolchain launch."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import ci_macos_package_transfer as transfer
from package_macos_reference import APPS, digest

REVISION = 'a' * 40


def fixture(directory):
    applications = {}
    for app, (name, identity, executable) in APPS.items():
        location = directory / app
        location.mkdir()
        archive = location / (name + '.zip')
        binary = ('executable fixture ' + app).encode()
        with zipfile.ZipFile(archive, 'w') as zipped:
            zipped.writestr(name + '.app/Contents/MacOS/' + executable, binary)
        report = {'app': app, 'complete': True, 'signing': 'ad-hoc', 'dirty': False,
                  'revision': REVISION, 'bundle': name + '.app', 'archive': archive.name,
                  'archive_sha256': digest(archive),
                  'packaged_executable_sha256': hashlib.sha256(binary).hexdigest(),
                  'metadata': {'CFBundleName': name, 'CFBundleIdentifier': identity,
                               'CFBundleExecutable': executable, 'CFBundlePackageType': 'APPL',
                               'CFBundleVersion': '1', 'CFBundleShortVersionString': '0.1.0',
                               'LSMinimumSystemVersion': '14.4', 'NSHighResolutionCapable': True}}
        (location / 'package.json').write_text(json.dumps(report))
        applications[app] = {'package_sha256': digest(location / 'package.json'),
                             'archive_sha256': report['archive_sha256'],
                             'executable_sha256': report['packaged_executable_sha256']}
    manifest = {'schema_version': 1, 'complete': True, 'source_revision': REVISION,
                'github_run_id': os.environ.get('GITHUB_RUN_ID'),
                'github_run_attempt': os.environ.get('GITHUB_RUN_ATTEMPT'),
                'applications': applications, 'qualification': 'test fixture'}
    (directory / 'transfer.json').write_text(json.dumps(manifest))
    return manifest


class TransferTests(unittest.TestCase):
    def test_exact_revision_and_all_applications_required(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            manifest = fixture(directory)
            self.assertEqual(set(transfer.verify(directory, REVISION)['verified']), set(APPS))
            with self.assertRaisesRegex(ValueError, 'Wrong source'):
                transfer.verify(directory, 'b' * 40)
            manifest['applications'].pop('gallery')
            (directory / 'transfer.json').write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, 'incomplete'):
                transfer.verify(directory, REVISION)

    def test_changed_metadata_or_archive_is_rejected(self):
        for target in ['package.json', APPS['gallery'][0] + '.zip']:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temporary:
                directory = Path(temporary)
                fixture(directory)
                path = directory / 'gallery' / target
                path.write_bytes(path.read_bytes() + b'changed after transfer')
                with self.assertRaisesRegex(ValueError, 'hash'):
                    transfer.verify(directory, REVISION)

    def test_a_different_run_cannot_reuse_same_source_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            fixture(directory)
            with patch.dict(os.environ, {'GITHUB_RUN_ID': 'different-run'}):
                with self.assertRaisesRegex(ValueError, 'different CI run'):
                    transfer.verify(directory, REVISION)

    def test_failed_staging_retains_incomplete_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'transfer'
            with patch.object(transfer, 'check_revision'), patch.object(
                    transfer, 'package', side_effect=RuntimeError('assembly failed')):
                with self.assertRaisesRegex(RuntimeError, 'assembly failed'):
                    transfer.stage(output, REVISION)
            report = json.loads((output / 'transfer.json').read_text())
            self.assertFalse(report['complete'])
            self.assertEqual(report['applications'], {})
            self.assertIn('assembly failed', report['error'])

    def test_failed_final_verification_revokes_completion(self):
        def assemble(app, binary, output, notices, sign):
            output.mkdir()
            report = {'archive': 'fixture.zip', 'archive_sha256': 'archive-hash',
                      'packaged_executable_sha256': 'binary-hash'}
            (output / 'package.json').write_text(json.dumps(report))
            (output / report['archive']).write_bytes(b'fixture')
            return report

        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'transfer'
            with patch.object(transfer, 'check_revision'), patch.object(
                    transfer, 'package', side_effect=assemble), patch.object(
                    transfer, 'verify', side_effect=ValueError('final verification failed')):
                with self.assertRaisesRegex(ValueError, 'final verification failed'):
                    transfer.stage(output, REVISION)
            report = json.loads((output / 'transfer.json').read_text())
            self.assertFalse(report['complete'])
            self.assertEqual(set(report['applications']), set(APPS))
            self.assertIn('final verification failed', report['error'])


if __name__ == '__main__':
    unittest.main()
