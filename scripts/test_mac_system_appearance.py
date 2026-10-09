#!/usr/bin/env python3
"""Portable restoration control flow; does not access actual macOS settings."""
import json
from pathlib import Path
import signal
import tempfile
import unittest
from unittest.mock import patch

from mac_system_appearance import Appearance, checked_state, preserved_appearance


class Fake:
    def __init__(self, automatic, fail=False):
        self.state = {'dark': True, 'automatic': automatic}
        self.fail = fail
    def snapshot(self):
        return self.state.copy()
    def set_dark(self, value):
        self.state = {'dark': value, 'automatic': False}
    def restore(self, saved):
        if self.fail:
            raise RuntimeError('restore failed')
        self.state = saved.copy()


class Preservation(unittest.TestCase):
    def test_native_restore_orders_dark_then_exact_automatic_state(self):
        class Stub(Appearance):
            def __init__(self):
                self.mode, self.auto = False, None
                self.operations = []
            def dark(self):
                return self.mode
            def automatic(self):
                return self.auto
            def set_dark(self, value):
                self.operations.append('dark')
                self.mode, self.auto = value, False
        for original in (None, True, False):
            appearance = Stub()
            saved = {'dark': True, 'automatic': original}
            def write(arguments, **kwargs):
                self.assertEqual(appearance.operations, ['dark'])
                appearance.operations.append('automatic')
                appearance.auto = None if arguments[1] == 'delete' else arguments[-1] == 'true'
            with patch('mac_system_appearance.subprocess.run', side_effect=write) as writes:
                appearance.restore(saved)
            self.assertEqual(appearance.snapshot(), saved)
            self.assertEqual(writes.call_count, int(original is not False))
        appearance = Stub()
        with patch('mac_system_appearance.subprocess.run'):
            with self.assertRaisesRegex(RuntimeError, 'not restored'):
                appearance.restore({'dark': True, 'automatic': True})

    def test_success_failure_interrupt_and_all_auto_states_restore(self):
        for automatic in (None, False, True):
            for failure in (None, ValueError('test failed'), KeyboardInterrupt()):
                with tempfile.TemporaryDirectory() as temporary:
                    path = Path(temporary) / 'appearance.json'
                    fake = Fake(automatic)
                    saved = fake.snapshot()
                    handler = signal.getsignal(signal.SIGTERM)
                    try:
                        with preserved_appearance(path, lambda: fake) as active:
                            self.assertEqual(json.loads(path.read_text())['original'], saved)
                            self.assertFalse(json.loads(path.read_text())['restored'])
                            active.set_dark(False)
                            if failure:
                                raise failure
                    except BaseException as error:
                        self.assertIs(error, failure)
                    self.assertEqual(fake.snapshot(), saved)
                    self.assertTrue(json.loads(path.read_text())['restored'])
                    self.assertEqual(signal.getsignal(signal.SIGTERM), handler)

    def test_restore_failure_leaves_recovery_record_incomplete(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'appearance.json'
            fake = Fake(None, fail=True)
            with self.assertRaisesRegex(RuntimeError, 'restore failed'):
                with preserved_appearance(path, lambda: fake):
                    fake.set_dark(False)
            record = json.loads(path.read_text())
            self.assertFalse(record['restored'])
            self.assertEqual(record['original'], {'dark': True, 'automatic': None})

    def test_bad_snapshot_or_existing_recovery_prevents_test_start(self):
        for value in ({}, {'dark': 1, 'automatic': None}, {'dark': True, 'automatic': 'false'}):
            with self.assertRaises(ValueError):
                checked_state(value)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'appearance.json'
            path.write_text('original recovery')
            fake = Fake(True)
            with self.assertRaises(FileExistsError), preserved_appearance(path, lambda: fake):
                self.fail('Must not overwrite recovery state')
            self.assertEqual(path.read_text(), 'original recovery')
            self.assertTrue(fake.state['dark'])


if __name__ == '__main__':
    unittest.main()
