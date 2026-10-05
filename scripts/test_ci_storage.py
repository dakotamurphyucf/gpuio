#!/usr/bin/env python3
import tempfile
import subprocess
import sys
from pathlib import Path
import unittest

from ci_storage import prepare


class StorageTests(unittest.TestCase):
    def root(self, directory):
        root = Path(directory)
        (root / 'Cargo.lock').write_text('fixture')
        (root / 'dune-project').write_text('fixture')
        return root

    def test_local_discard_is_refused_and_reporting_is_read_only(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.root(directory)
            (root / 'target').mkdir()
            (root / 'target/keep').write_text('artifact')
            with self.assertRaisesRegex(RuntimeError, 'restricted'):
                prepare(root, discard_restored_target=True, environment={})
            result = prepare(root, discard_restored_target=False, environment={})
            self.assertFalse(result['discarded_restored_target'])
            self.assertTrue((root / 'target/keep').is_file())

    def test_only_owned_target_is_removed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.root(directory)
            (root / 'target').mkdir()
            (root / 'target/artifact').write_text('generated')
            (root / '_build').mkdir()
            (root / '_build/keep').write_text('unrelated to this cleanup')
            env = {'GITHUB_ACTIONS': 'true', 'GITHUB_WORKSPACE': str(root)}
            result = prepare(root, discard_restored_target=True, environment=env)
            self.assertTrue(result['discarded_restored_target'])
            self.assertFalse((root / 'target').exists())
            self.assertTrue((root / '_build/keep').exists())
            self.assertFalse(prepare(root, discard_restored_target=True,
                                     environment=env)['discarded_restored_target'])

    def test_symlink_and_wrong_workspace_are_refused(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryDirectory() as other:
            root = self.root(directory)
            (Path(other) / 'keep').write_text('external')
            (root / 'target').symlink_to(other, target_is_directory=True)
            with self.assertRaisesRegex(RuntimeError, 'restricted'):
                prepare(root, discard_restored_target=True,
                        environment={'GITHUB_ACTIONS': 'true', 'GITHUB_WORKSPACE': other})
            with self.assertRaisesRegex(RuntimeError, 'symlink'):
                prepare(root, discard_restored_target=True,
                        environment={'GITHUB_ACTIONS': 'true', 'GITHUB_WORKSPACE': str(root)})
            self.assertTrue((Path(other) / 'keep').exists())


class ConsumerWorkspaceTests(unittest.TestCase):
    def test_failed_consumer_preserves_default_and_cleans_only_when_requested(self):
        # Stop before any toolchain command. A subprocess exercises real atexit
        # cleanup, which does not run when runpy returns in this test process.
        driver = '''
import runpy, sys
from unittest.mock import patch
script, workspace, cleanup = sys.argv[1:]
sys.argv = [script] + (["--cleanup"] if cleanup == "yes" else [])
with patch("tempfile.mkdtemp", return_value=workspace), patch("subprocess.run", side_effect=RuntimeError("injected build failure")):
    runpy.run_path(script, run_name="__main__")
'''
        for cleanup in (False, True):
            with self.subTest(cleanup=cleanup), tempfile.TemporaryDirectory() as directory:
                workspace = Path(directory) / 'generated-consumer'
                workspace.mkdir()
                result = subprocess.run(
                    [sys.executable, '-c', driver,
                     str(Path(__file__).with_name('test_extension_consumer.py').resolve()),
                     str(workspace), 'yes' if cleanup else 'no'],
                    text=True, capture_output=True, timeout=15)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('injected build failure', result.stderr)
                self.assertEqual(workspace.exists(), not cleanup)


if __name__ == '__main__':
    unittest.main()
