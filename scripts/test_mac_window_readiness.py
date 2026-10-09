#!/usr/bin/env python3
"""Portable pointer readiness regressions; actual OS evidence is separate."""
import unittest
import contextlib
import io
from types import SimpleNamespace
from unittest.mock import patch

import test_macos_window_lifecycle as lifecycle
from test_gallery import GalleryMouse
from test_macos_hit_testing import PointRouting


class Clock:
    now = 0.

    def sleep(self, seconds):
        self.now += seconds


class Pointer(GalleryMouse):
    def __init__(self, owner):
        self.mac = SimpleNamespace(pid=42, app=object())
        self.owner = owner
        self.checked = []
        self.lose_on_check = False

    def owner_at(self, point):
        pid, subrole = self.owner()
        return {'expected_pid': 42, 'actual_pid': pid, 'found': True,
                'hit_status': 0, 'pid_status': 0, 'role': 'AXButton', 'subrole': subrole}

    def check_owner(self, point):
        self.checked.append(point)
        if self.lose_on_check:
            self.owner = lambda: (99, None)
        super().check_owner(point)


class IdentityTests(unittest.TestCase):
    """A ready process is insufficient: semantic identity must still be exact."""

    def measure(self, hit, *, pid=42):
        routing = PointRouting.__new__(PointRouting)
        released, queries = [], []
        routing.mac = SimpleNamespace(
            pid=42, wait_find=lambda *args: 2,
            text=lambda *args: 'test node', release=released.append,
            attr=lambda node, attr: {3: 2, 4: 1}.get(node.value))
        routing.root, routing.observations = 100, []
        routing.equal = lambda node, target: node.value == target

        def query(root, x, y, result):
            queries.append((x, y))
            result._obj.value = hit
            return 0

        def owner(node, result):
            result._obj.value = pid
            return 0

        routing.hit, routing.pid = query, owner
        with patch('test_macos_hit_testing.ready_pointer'), \
                patch('test_macos_hit_testing.GalleryMouse'), \
                patch('test_macos_hit_testing.element_rect', return_value=(0, 0, 20, 20)), \
                contextlib.redirect_stdout(io.StringIO()):
            try:
                routing.check('Target')
            finally:
                self.assertEqual(queries, [(10, 10)], 'Never retry semantic identity')
                self.assertIn(2, released, 'Release the requested target even on failure')
        return routing.observations[0]['match']

    def test_exact_control_and_real_descendant_pass(self):
        self.assertEqual(self.measure(2), 'exact')
        self.assertEqual(self.measure(3), 'descendant')

    def test_window_ancestor_and_unrelated_control_fail_without_retry(self):
        for hit in (1, 4):
            with self.subTest(hit=hit), self.assertRaisesRegex(AssertionError, 'Point did not resolve'):
                self.measure(hit)

    def test_foreign_process_after_readiness_still_fails(self):
        with self.assertRaises(AssertionError):
            self.measure(2, pid=99)


class ReadinessTests(unittest.TestCase):
    def test_fixture_fits_display_including_later_drag(self):
        for display in [(0, 0, 1600, 1200), (0, 0, 1440, 900),
                        (-1600, -900, 1060, 790)]:
            x, y, width, height = lifecycle.fixture_geometry(display)
            self.assertGreaterEqual(x, display[0])
            self.assertGreaterEqual(y, display[1])
            self.assertLess(x+width+35, display[0]+display[2])
            self.assertLess(y+height+25, display[1]+display[3])
            self.assertGreaterEqual(width, 960)
            self.assertGreaterEqual(height, 650)

    def test_fixture_rejects_display_too_small_for_the_walkthrough(self):
        for display in [(0, 0, 1059, 900), (0, 0, 1440, 789)]:
            with self.assertRaisesRegex(RuntimeError, '1060x790'):
                lifecycle.fixture_geometry(display)

    def wait(self, clock, mouse, report, *, point=lambda: (10., 20.), timeout=.5, foreground=True):
        deadlines = []

        def position(mac, label, role, *, deadline):
            deadlines.append(deadline)
            return point()

        with patch.object(lifecycle.time, 'monotonic', lambda: clock.now), \
                patch.object(lifecycle.time, 'sleep', clock.sleep), \
                patch.object(lifecycle, 'point_for', position), \
                patch.object(lifecycle, 'raise_gallery'), \
                patch.object(lifecycle, 'boolean', return_value=foreground), \
                patch.object(lifecycle.subprocess, 'run',
                             return_value=SimpleNamespace(stdout='/Applications/Occluder.app/Occluder\n')):
            result = lifecycle.ready_pointer(mouse.mac, mouse, 'Fullscreen', 'AXButton', report, timeout)
        self.assertTrue(all(value == timeout for value in deadlines))
        return result

    def test_occlusion_and_moving_coordinates_must_settle_before_admission(self):
        clock, report = Clock(), {}
        pointer = Pointer(lambda: (99 if clock.now < .05 else 42, None))
        point = lambda: (1., 2.) if clock.now < .15 else (10., 20.)
        self.assertEqual(self.wait(clock, pointer, report, point=point), (10., 20.))
        self.assertGreaterEqual(clock.now, .25)
        self.assertEqual(pointer.checked, [(10., 20.)])
        samples = report['pointer_readiness'][0]
        self.assertTrue(samples['ready'])
        self.assertEqual(samples['transitions'][0]['actual_pid'], 99)
        self.assertEqual(samples['transitions'][0]['process'], 'Occluder')

    def test_persistent_occlusion_fails_without_admitting_pointer_input(self):
        clock, report = Clock(), {}
        pointer = Pointer(lambda: (99, None))
        with self.assertRaisesRegex(RuntimeError, 'Pointer target did not become ready'):
            self.wait(clock, pointer, report, timeout=.2)
        self.assertLess(clock.now, .25)
        self.assertEqual(pointer.checked, [])
        self.assertFalse(report['pointer_readiness'][0]['ready'])

    def test_window_controls_remain_rejected_even_when_the_pid_matches(self):
        clock, report = Clock(), {}
        pointer = Pointer(lambda: (42, 'AXCloseButton'))
        with self.assertRaisesRegex(RuntimeError, 'Pointer target did not become ready'):
            self.wait(clock, pointer, report, timeout=.2)
        self.assertEqual(pointer.checked, [])
        with self.assertRaisesRegex(AssertionError, 'window control'):
            pointer.check_owner((10., 20.))

    def test_last_moment_ownership_change_still_fails_the_original_guard(self):
        clock, report = Clock(), {}
        pointer = Pointer(lambda: (42, None))
        pointer.lose_on_check = True
        with self.assertRaisesRegex(AssertionError, 'Pointer target is occluded'):
            self.wait(clock, pointer, report)
        self.assertFalse(report['pointer_readiness'][0]['ready'])

    def test_background_target_is_not_admitted(self):
        clock, report = Clock(), {}
        pointer = Pointer(lambda: (42, None))
        with self.assertRaisesRegex(RuntimeError, 'Pointer target did not become ready'):
            self.wait(clock, pointer, report, timeout=.2, foreground=False)
        self.assertEqual(pointer.checked, [])

    def test_continuously_moving_target_times_out_with_bounded_diagnostics(self):
        clock, report = Clock(), {}
        pointer = Pointer(lambda: (42, None))
        with self.assertRaisesRegex(RuntimeError, 'Pointer target did not become ready'):
            self.wait(clock, pointer, report, point=lambda: (clock.now, 20.), timeout=1.)
        observation = report['pointer_readiness'][0]
        self.assertEqual(len(observation['transitions']), 24)
        self.assertGreater(observation['dropped'], 0)
        self.assertEqual(pointer.checked, [])

    def test_native_lookup_overrunning_deadline_does_not_admit_a_late_click(self):
        clock, report = Clock(), {}
        pointer = Pointer(lambda: (42, None))
        calls = 0

        def point():
            nonlocal calls
            calls += 1
            if calls == 3:
                clock.now += 1.  # An OS call itself is not preemptible here.
            return (10., 20.)

        with self.assertRaisesRegex(RuntimeError, 'Pointer target did not become ready'):
            self.wait(clock, pointer, report, point=point)
        self.assertEqual(pointer.checked, [])


if __name__ == '__main__':
    unittest.main()
