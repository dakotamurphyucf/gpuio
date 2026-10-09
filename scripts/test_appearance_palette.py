"""Palette painting checks must be invariant to screenshot backing scale."""
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import unittest

from test_macos_system_appearance import palette


class Pixels:
    def __init__(self, scale, rgb):
        self.width, self.height = 1120 * scale, 848 * scale
        self.color = rgb

    def rgb(self, x, y):
        return self.color if x < self.width * .18 else (85, 85, 85)


class PaletteTests(unittest.TestCase):
    def test_already_painted_page_passes_at_both_scales(self):
        mac = SimpleNamespace(wait_find=lambda *args: 1, release=lambda _: None)
        for dark, expected in [(True, (16, 21, 29)), (False, (241, 244, 247))]:
            fractions = []
            for scale in (1, 2):
                with patch('test_macos_system_appearance.screenshot', side_effect=[None]), \
                        patch('test_macos_system_appearance.read_png', return_value=Pixels(scale, expected)):
                    result = palette(mac, Path('unused-fixture-directory'), 'painted', 'fixture', dark)
                fractions.append(result['matching_fraction'])
            self.assertAlmostEqual(fractions[0], fractions[1], delta=.01)

    def test_wrong_paint_still_fails_even_if_label_matches(self):
        mac = SimpleNamespace(wait_find=lambda *args: 1, release=lambda _: None)
        with patch('test_macos_system_appearance.screenshot'), \
                patch('test_macos_system_appearance.read_png', return_value=Pixels(2, (241, 244, 247))), \
                patch('test_macos_system_appearance.time.monotonic', side_effect=[0, 9]):
            with self.assertRaisesRegex(AssertionError, 'expected palette not painted'):
                palette(mac, Path('unused-fixture-directory'), 'wrong', 'fixture', True)


if __name__ == '__main__':
    unittest.main()
