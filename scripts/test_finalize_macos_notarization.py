#!/usr/bin/env python3
"""Portable final-artifact tests with simulated Apple tools, never network/upload."""
from contextlib import ExitStack
import hashlib
import json
from pathlib import Path
import plistlib
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import finalize_macos_notarization as finalizer
from package_macos_reference import digest
import test_macos_package_runtime as runtime
import test_package_runtime_inputs as runtime_inputs

SUBMISSION = '01234567-89ab-cdef-0123-456789abcdef'


class Finalization(unittest.TestCase):
    def fixture(self, root):
        package = root / 'input'
        package.mkdir()
        report, archive = runtime_inputs.RuntimeInputs().package(package)
        binary = b'executable fixture'
        report.update(signing='developer-id', signing_identity='AB' * 20,
                      signing_team='AB12345678', notices={},
                      packaged_executable_sha256=hashlib.sha256(binary).hexdigest())
        with zipfile.ZipFile(archive, 'a') as zipped:
            zipped.writestr(report['bundle'] + '/Contents/Info.plist', plistlib.dumps(report['metadata']))
        report['archive_sha256'] = digest(archive)
        (package / 'package.json').write_text(json.dumps(report))
        return package, report

    def tools(self, stack, report, *, verdict=None, drop_ticket=False, reject_signature=False):
        calls = []
        log = {'logFormatVersion': 1, 'jobId': SUBMISSION.upper(), 'status': 'Accepted',
               'sha256': report['archive_sha256'], 'issues': None}
        log.update(verdict or {})

        def extract_command(args, **kwargs):
            self.assertEqual(args[:3], ['/usr/bin/ditto', '-x', '-k'])
            with zipfile.ZipFile(args[3]) as zipped:
                zipped.extractall(args[4])

        def tool(*args):
            calls.append(args)
            if args[1:3] == ('notarytool', 'log'):
                Path(args[-1]).write_text(json.dumps(log))
            elif args[1:3] == ('stapler', 'staple'):
                (Path(args[-1]) / 'Contents/ticket').write_bytes(b'simulated ticket')
            elif args[1:3] == ('stapler', 'validate'):
                if not (Path(args[-1]) / 'Contents/ticket').is_file():
                    raise ValueError('Ticket missing from extracted archive')
            elif args[:2] == ('/usr/bin/ditto', '-c'):
                bundle = Path(args[-2])
                with zipfile.ZipFile(args[-1], 'w') as zipped:
                    for p in bundle.rglob('*'):
                        if p.is_file() and not (drop_ticket and p.name == 'ticket'):
                            zipped.write(p, p.relative_to(bundle.parent))
            else:
                raise AssertionError(args)
            return 'simulated Apple tool output'

        stack.enter_context(patch.object(finalizer.platform, 'system', return_value='Darwin'))
        stack.enter_context(patch.object(runtime.subprocess, 'run', side_effect=extract_command))
        stack.enter_context(patch.object(finalizer, 'run', side_effect=tool))
        verify = stack.enter_context(patch.object(finalizer.macos_signing, 'verify',
                                                  return_value={'arm64': {'simulated': True}}))
        if reject_signature:
            verify.side_effect = ValueError('Certificate verification failed')
        return calls

    def test_final_archive_round_trip_keeps_ticket_and_preserves_input(self):
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            root = Path(temporary)
            package, report = self.fixture(root)
            before = (package / report['archive']).read_bytes()
            calls = self.tools(stack, report, verdict={'issues': [{'severity': 'warning', 'message': 'review me'}]})
            output = root / 'final'
            result = finalizer.finalize(package, output, SUBMISSION, 'test profile')
            self.assertTrue(result['complete'])
            self.assertEqual(len(result['warnings']), 1)
            final, archive = runtime.inspect_package(output)
            self.assertEqual(final['notarization']['submitted_archive_sha256'], report['archive_sha256'])
            self.assertEqual(digest(archive), result['final_archive_sha256'])
            self.assertNotEqual(digest(archive), report['archive_sha256'])
            self.assertEqual((package / report['archive']).read_bytes(), before)
            self.assertEqual(sum(args[1:3] == ('stapler', 'validate') for args in calls), 2)
            self.assertFalse(any('submit' in args for args in calls))
            with self.assertRaisesRegex(ValueError, 'fresh output'):
                finalizer.finalize(package, output, SUBMISSION, 'test profile')

    def test_wrong_submission_hash_or_status_never_staples(self):
        for verdict in [{'status': 'Invalid'}, {'status': 'In Progress'},
                        {'sha256': '0' * 64}, {'jobId': 'abcdefab-89ab-cdef-0123-456789abcdef'},
                        {'issues': [{'severity': 'error'}]}, {'logFormatVersion': 2},
                        {'logFormatVersion': True}, {'jobId': None}, {'sha256': None}]:
            with self.subTest(verdict=verdict), tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
                root = Path(temporary)
                package, report = self.fixture(root)
                calls = self.tools(stack, report, verdict=verdict)
                output = root / 'final'
                with self.assertRaises(ValueError):
                    finalizer.finalize(package, output, SUBMISSION, 'test profile')
                self.assertFalse(json.loads((output / 'package.json').read_text())['complete'])
                self.assertFalse(json.loads((output / 'notarization.json').read_text())['complete'])
                self.assertFalse(any('stapler' in args for args in calls))
                self.assertFalse((output / report['archive']).exists())

    def test_ticket_lost_in_archive_does_not_mark_output_complete(self):
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            root = Path(temporary)
            package, report = self.fixture(root)
            self.tools(stack, report, drop_ticket=True)
            output = root / 'final'
            with self.assertRaisesRegex(ValueError, 'Ticket missing'):
                finalizer.finalize(package, output, SUBMISSION, 'test profile')
            self.assertFalse(json.loads((output / 'package.json').read_text())['complete'])
            self.assertFalse(json.loads((output / 'notarization.json').read_text())['complete'])

    def test_signature_rejection_precedes_network_lookup(self):
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            root = Path(temporary)
            package, report = self.fixture(root)
            calls = self.tools(stack, report, reject_signature=True)
            with self.assertRaisesRegex(ValueError, 'Certificate verification'):
                finalizer.finalize(package, root / 'final', SUBMISSION, 'test profile')
            self.assertEqual(calls, [])

    def test_adhoc_package_is_not_a_notarization_candidate(self):
        with tempfile.TemporaryDirectory() as temporary, ExitStack() as stack:
            root = Path(temporary)
            package, report = self.fixture(root)
            report['signing'] = 'ad-hoc'
            report.pop('signing_identity')
            report.pop('signing_team')
            (package / 'package.json').write_text(json.dumps(report))
            calls = self.tools(stack, report)
            with self.assertRaisesRegex(ValueError, 'Developer ID'):
                finalizer.finalize(package, root / 'final', SUBMISSION, 'test profile')
            self.assertEqual(calls, [])
            self.assertFalse((root / 'final').exists())


if __name__ == '__main__':
    unittest.main()
