#!/usr/bin/env python3
"""Portable failure-path checks for the OS-setting scope; no macOS calls."""
import json
from pathlib import Path
import tempfile
import unittest

from mac_input_source import MODE, PARENT, japanese_source


class FakeSources:
    enable = 'enable'
    select = 'select'

    def __init__(self, failure=None):
        self.failure = failure
        self.calls = []
        self.original = {'selected': 'original', 'enabled': {PARENT: False, MODE: True}}
        self.output = None

    def selected(self):
        return MODE if ('select', MODE) in self.calls else 'original'

    def enabled(self):
        return self.original['enabled'].copy()

    def checked(self, operation, source):
        # Recovery evidence must exist even if the very first mutation fails.
        assert json.loads((self.output / 'original.json').read_text()) == self.original
        self.calls.append((operation, source))
        if self.failure == (operation, source):
            raise RuntimeError('injected mutation failure')

    def restore(self, original, output):
        assert original == self.original
        assert output == self.output
        self.calls.append(('restore', original['selected']))


class InputSourceScopeTests(unittest.TestCase):
    def test_success_restores_original(self):
        with tempfile.TemporaryDirectory() as directory:
            sources = FakeSources()
            sources.output = Path(directory)
            with japanese_source(sources, sources.output):
                self.assertEqual(sources.selected(), MODE)
            self.assertEqual(sources.calls, [('enable', PARENT), ('enable', MODE),
                                            ('select', MODE), ('restore', 'original')])

    def test_partial_mutation_failures_restore(self):
        for failure in [('enable', PARENT), ('enable', MODE), ('select', MODE)]:
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                sources = FakeSources(failure)
                sources.output = Path(directory)
                with self.assertRaisesRegex(RuntimeError, 'injected'):
                    with japanese_source(sources, sources.output):
                        self.fail('Scope body must not run after failed mutation')
                self.assertEqual(sources.calls[-1], ('restore', 'original'))

    def test_keyboard_interrupt_restores(self):
        with tempfile.TemporaryDirectory() as directory:
            sources = FakeSources()
            sources.output = Path(directory)
            with self.assertRaises(KeyboardInterrupt):
                with japanese_source(sources, sources.output):
                    raise KeyboardInterrupt()
            self.assertEqual(sources.calls[-1], ('restore', 'original'))

    def test_missing_method_does_not_mutate(self):
        with tempfile.TemporaryDirectory() as directory:
            sources = FakeSources()
            sources.original['enabled'] = {}
            sources.output = Path(directory)
            with self.assertRaisesRegex(RuntimeError, 'Installed Japanese'):
                with japanese_source(sources, sources.output):
                    self.fail('Unavailable source must not enter the scope')
            self.assertEqual(sources.calls, [])


if __name__ == '__main__':
    unittest.main()
