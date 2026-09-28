#!/usr/bin/env python3
"""Public chart non-color labels, light styling, mixed/oriented plots and native legend wheel."""
import argparse
import ctypes as C
from pathlib import Path
import subprocess
import time
from test_tree_outline import Outline, Point
from test_charts import TITLE
from test_canvas import screenshot


class Charts(Outline):
    def try_bounds(self, node):
        position, size = Point(), Point()
        for label, kind, target in [('AXPosition', 1, position), ('AXSize', 2, size)]:
            raw = self.attr(node, label)
            try:
                if not raw or not self.value(raw, kind, C.byref(target)):
                    return None
            finally:
                if raw:
                    self.release(raw)
        return position.x, position.y, size.x, size.y

    def bounds(self, node):
        result = self.try_bounds(node)
        if result is None:
            raise RuntimeError('Missing native chart bounds')
        return result

    def legend_row_is_visible(self, label):
        node = self.find(TITLE, label, 'AXStaticText')
        if not node:
            return False
        legend = self.find(TITLE, 'Chart legend', 'AXGroup')
        try:
            # A reset can retire an AX row between lookup and geometry reads.
            # The bounded visibility wait must reacquire the new row, not fail
            # immediately or treat an unavailable rectangle as visible.
            row_bounds = self.try_bounds(node)
            legend_bounds = self.try_bounds(legend) if legend else None
            if row_bounds is None or legend_bounds is None:
                return False
            x, y, w, h = row_bounds
            lx, ly, lw, lh = legend_bounds
            return w > 0 and h > 0 and x >= lx - 1 and x + w <= lx + lw + 1 and y >= ly - 1 and y + h <= ly + lh + 1
        finally:
            self.release(node)
            if legend:
                self.release(legend)

    def wait_visible(self, label):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if self.legend_row_is_visible(label):
                return
            time.sleep(.05)
        raise RuntimeError(f'Legend row is not in the visible native viewport: {label}')

    def wheel_legend(self, pixels):
        legend = self.wait_find(TITLE, 'Chart legend', 'AXGroup')
        try:
            x, y, width, height = self.bounds(legend)
            point = Point(x + width / 2, y + height / 2)
        finally:
            self.release(legend)
        self.send(5, point)  # Verify the target is owned by our foreground child.
        wheel = self.cg.CGEventCreateScrollWheelEvent
        wheel.restype, wheel.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
        location = self.cg.CGEventSetLocation
        location.restype, location.argtypes = None, [C.c_void_p, Point]
        event = wheel(None, 0, 1, pixels)
        if not event:
            raise RuntimeError('Cannot create chart legend wheel event')
        try:
            location(event, point)
            self.post(0, event)
        finally:
            self.release(event)

    def exercise(self, output):
        self.wait_text(TITLE, '48 values')
        self.press(TITLE, 'Mixed layers')
        self.wait_text(TITLE, '72 values')
        for index, name in enumerate(['Capacity', 'Throughput', 'Demand']):
            self.wait_text(TITLE, f'Series {index + 1} · {name}')
        self.press(TITLE, 'Monochrome')
        self.wait_text(TITLE, 'Palette')
        time.sleep(.2)
        screenshot(self, output / 'mixed-monochrome.png', title=TITLE)
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        time.sleep(.2)
        screenshot(self, output / 'mixed-light.png', title=TITLE)
        self.press(TITLE, 'View data')
        self.wait_text(TITLE, 'Original data · all values')
        self.wait_text(TITLE, 'Back to chart')
        screenshot(self, output / 'data-light.png', title=TITLE)
        self.press(TITLE, 'Back to chart')
        self.press(TITLE, 'Horizontal')
        self.wait_text(TITLE, '48 values')
        for index, name in enumerate(['Atlas', 'Nova']):
            self.wait_text(TITLE, f'Series {index + 1} · {name}')
        time.sleep(.2)
        screenshot(self, output / 'horizontal-light.png', title=TITLE)
        self.press(TITLE, 'Radar')
        self.wait_text(TITLE, '10 values')
        for index, name in enumerate(['Atlas', 'Nova']):
            self.wait_text(TITLE, f'Series {index + 1} · {name}')
        time.sleep(.2)
        screenshot(self, output / 'radar-monochrome.png', title=TITLE)
        self.press(TITLE, 'Pie')
        self.wait_text(TITLE, '4 values')
        for name in ['Reasoning', 'Code', 'Research', 'Other']:
            self.wait_text(TITLE, name)
        time.sleep(.2)
        screenshot(self, output / 'pie-monochrome.png', title=TITLE)
        self.press(TITLE, 'Dense legend')
        self.wait_text(TITLE, '128 values')
        self.wait_visible('Channel 001')
        assert not self.legend_row_is_visible('Channel 128')
        self.wheel_legend(-2000)
        self.wait_visible('Channel 128')
        self.wait_text(TITLE, 'Select a value ·')  # Wheel is not a plot selection.
        screenshot(self, output / 'legend-scrolled.png', title=TITLE)
        self.press(TITLE, 'Update data ↗')
        self.wait_text(TITLE, '129 values')
        self.wait_visible('Channel 128')
        self.press(TITLE, 'Dense legend')  # Reset generation, same chart/source handle.
        self.wait_text(TITLE, '129 values')
        self.wait_visible('Channel 001')
        assert not self.legend_row_is_visible('Channel 128')
        self.close(TITLE)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('.cache/chart-visual-acceptance'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with (args.output / 'application.log').open('w') as log:
        child = subprocess.Popen(['_build/default/examples/charts/main.exe'], stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Charts(child.pid, child)
            mac.exercise(args.output)
            assert child.wait(timeout=15) == 0
            print('CHART_VISUAL_OK: mixed/horizontal identifiers, monochrome/light/data surfaces, real legend wheel, retained update/reset position, no wheel selection, OS close', flush=True)
        finally:
            if mac:
                mac.release(mac.app)
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()


if __name__ == '__main__':
    main()
