#!/usr/bin/env python3
"""Portable AX readiness regressions; these do not replace native testing."""
from collections import Counter
import ctypes as C
import unittest
from unittest.mock import patch

import test_agent_chat_results as results
from test_tree_outline import Point


class Fixture(results.Results):
    def __init__(self, samples):
        self.samples = samples
        self.sample = -1
        self.refs = Counter()
        self.now = 0.
        self.overrun = False

    def retain(self, node):
        self.refs[node] += 1
        return node

    def release(self, node):
        assert self.refs[node] > 0, ('unowned release', node)
        self.refs[node] -= 1

    def find(self, *_args, **_kwargs):
        self.sample += 1
        self.current = self.samples[min(self.sample, len(self.samples) - 1)]
        return self.retain(('root', self.sample))

    def children(self, node):
        if node[0] == 'root' and self.current != 'absent':
            # A matching data cell must not be accepted as the column header.
            return [self.retain((kind, self.sample)) for kind in ('cell', 'header')]
        return []

    def text(self, node, name):
        if name == 'AXRole':
            return 'AXTable' if node[0] == 'root' else 'AXCell'
        if name == 'AXTitle':
            return 'RESULT'
        if name == 'AXValue' and node[0] == 'cell':
            return '000001'
        return None

    def attr(self, node, name):
        assert node[0] == 'header', 'data cell was mistaken for header'
        if self.current == name:
            return None
        return self.retain((name, self.sample))

    def value(self, raw, _kind, pointer):
        if self.current == 'undecodable':
            return False
        point = C.cast(pointer, C.POINTER(Point)).contents
        if raw[0] == 'AXPosition':
            point.x, point.y = 10., 20.
            if self.current == 'moving':
                point.x += self.sample
        else:
            point.x, point.y = 126., 33.
            if self.current == 'zero':
                point.x = 0.
            elif self.current == 'nonfinite':
                point.y = float('nan')
            if self.overrun:
                self.now += 6.
        return True

    def sleep(self, seconds):
        self.now += seconds

    def bounds_for_test(self):
        with patch.object(results.time, 'monotonic', lambda: self.now), \
                patch.object(results.time, 'sleep', self.sleep):
            return self.bounds('RESULT', 'AXColumnHeader')


class GeometryTests(unittest.TestCase):
    def assert_released(self, fixture):
        self.assertTrue(all(count == 0 for count in fixture.refs.values()))

    def test_fresh_objects_are_reacquired_until_geometry_exists(self):
        fixture = Fixture(['absent', 'AXPosition', 'AXSize', 'undecodable',
                           'zero', 'nonfinite', 'ready'])
        position, size = fixture.bounds_for_test()
        self.assertEqual((position.x, position.y, size.x, size.y), (10., 20., 126., 33.))
        self.assertGreaterEqual(fixture.sample, 10)
        self.assert_released(fixture)

    def test_valid_but_moving_geometry_is_not_ready(self):
        fixture = Fixture(['moving'])
        with self.assertRaisesRegex(RuntimeError, 'still settling'):
            fixture.bounds_for_test()
        self.assertLess(fixture.now, 5.04)
        self.assert_released(fixture)

    def test_persistent_failure_is_bounded_and_preserves_diagnostics(self):
        for state, reason in [('absent', 'Missing visible column header'),
                              ('AXPosition', 'Missing AXPosition'),
                              ('AXSize', 'Missing AXSize'),
                              ('undecodable', 'Missing AXPosition'),
                              ('zero', 'Invisible element'),
                              ('nonfinite', 'Nonfinite geometry')]:
            with self.subTest(state=state):
                fixture = Fixture([state])
                with self.assertRaisesRegex(RuntimeError, reason):
                    fixture.bounds_for_test()
                self.assertGreaterEqual(fixture.now, 5.)
                self.assertLess(fixture.now, 5.04)
                self.assert_released(fixture)

    def test_slow_os_read_does_not_admit_geometry_past_deadline(self):
        fixture = Fixture(['ready'])
        fixture.overrun = True
        with self.assertRaisesRegex(RuntimeError, 'did not become ready'):
            fixture.bounds_for_test()
        self.assertEqual(fixture.sample, 0)
        self.assert_released(fixture)

    def test_unexpected_traversal_error_is_not_silently_retried(self):
        fixture = Fixture(['ready'])
        with patch.object(fixture, 'attr', side_effect=ValueError('unexpected')):
            with self.assertRaisesRegex(ValueError, 'unexpected'):
                fixture.bounds_for_test()
        self.assertEqual(fixture.sample, 0)
        self.assert_released(fixture)


if __name__ == '__main__':
    unittest.main()
