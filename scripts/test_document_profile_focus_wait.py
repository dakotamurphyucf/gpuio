#!/usr/bin/env python3
"""Portable focus-acknowledgement checks; no OS input or application launch."""
from contextlib import ExitStack
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import test_macos_document_profile as profile


class Mac:
    app = object()

    def __init__(self, states):
        self.states = iter(states)
        self.current = None
        self.released = []
        self.cf = SimpleNamespace(CFBooleanGetValue=lambda value: value)

    def attr(self, node, attribute):
        if node is self.app:
            assert attribute == 'AXFocusedUIElement'
            self.current = next(self.states, self.current)
            return self.current
        assert attribute == 'AXFocused'
        return node[2]

    def text(self, node, attribute):
        return node[0 if attribute == 'AXRole' else 1]

    def release(self, value):
        self.released.append(value)


class FocusWait(unittest.TestCase):
    def run_wait(self, states, observations):
        now = [0.]
        def advance(seconds):
            now[0] += seconds
        mac = Mac(states)
        with ExitStack() as stack:
            stack.enter_context(patch.object(profile.time, 'monotonic', lambda: now[0]))
            stack.enter_context(patch.object(profile.time, 'sleep', advance))
            result = profile.wait_for_tab_exit(mac, observations, timeout=.06)
        return result, mac

    def test_delayed_acknowledgement_waits_without_accepting_origin(self):
        origin = ('AXButton', 'Show review end', True)
        destination = ('AXButton', 'Next control', True)
        observations = []
        result, mac = self.run_wait([origin, origin, destination], observations)
        self.assertEqual(result, 'Next control')
        self.assertEqual([x['title'] for x in observations],
                         ['Show review end', 'Show review end', 'Next control'])
        self.assertEqual(mac.released.count(destination), 1)

    def test_clipped_focus_is_failure_even_if_a_later_target_would_be_valid(self):
        with self.assertRaisesRegex(AssertionError, 'clipped'):
            self.run_wait([('AXButton', 'Open scroll review', True),
                           ('AXButton', 'Next control', True)], [])

    def test_stuck_origin_and_window_or_unfocused_objects_never_pass(self):
        for state in [('AXButton', 'Show review end', True),
                      ('AXWindow', 'Gallery', True), ('AXButton', 'Next control', False), None]:
            with self.subTest(state=state), self.assertRaisesRegex(AssertionError, 'did not leave'):
                self.run_wait([state], [])


if __name__ == '__main__':
    unittest.main()
