#!/usr/bin/env python3
"""Public picker: macOS AX actions, actual OS keys, dismissal and focus restore."""
import ctypes as C
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time

from test_agent_chat import Mac
from test_numeric import Numeric

TITLE = 'GPUIO color picker'
INITIAL = '#7C6FF080'
SELECTED = '#FF000080'


class Picker(Mac):
    number = Numeric.number

    def bounds(self, node):
        class Point(C.Structure):
            _fields_ = [('x', C.c_double), ('y', C.c_double)]
        point, size = Point(), Point()
        get = self.ax.AXValueGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        for attr, kind, target in [('AXPosition', 1, point), ('AXSize', 2, size)]:
            value = self.attr(node, attr)
            try:
                if not value or not get(value, kind, C.byref(target)):
                    raise RuntimeError(f'Missing {attr} for placement check')
            finally:
                if value:
                    self.release(value)
        return point.x, point.y, size.x, size.y

    def contained(self, *, clamped_from_anchor=None):
        panel = self.wait_find(TITLE, 'Choose accent color')
        window = self.window(TITLE)
        try:
            x, y, width, height = self.bounds(panel)
            wx, wy, ww, wh = self.bounds(window)
            assert width > 200 and height > 300, (x, y, width, height)
            assert (x >= wx and y >= wy and x + width <= wx + ww + 1
                    and y + height <= wy + wh + 1), (
                        'popup outside window', (x, y, width, height), (wx, wy, ww, wh))
            if clamped_from_anchor:
                anchor = self.wait_find(TITLE, clamped_from_anchor, 'AXButton')
                try:
                    ax, _, _, _ = self.bounds(anchor)
                    assert x < ax - 20, ('right-edge popup did not clamp inward', x, ax)
                finally:
                    self.release(anchor)
        finally:
            self.release(panel)
            self.release(window)

    def capture(self, name):
        directory = os.environ.get('GPUIO_PICKER_SCREENSHOT_DIR')
        if directory:
            from test_canvas import screenshot
            screenshot(self, Path(directory) / (name + '.png'))

    def boolean(self, node, attribute):
        value = self.attr(node, attribute)
        if not value:
            raise RuntimeError(f'Missing {attribute}')
        try:
            get = self.cf.CFBooleanGetValue
            get.restype, get.argtypes = C.c_bool, [C.c_void_p]
            return bool(get(value))
        finally:
            self.release(value)

    def button_state(self, label, attribute, expected):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = self.wait_find(TITLE, label, 'AXButton')
            try:
                actual = self.boolean(node, attribute)
                if actual == expected:
                    return
            finally:
                self.release(node)
            time.sleep(.02)
        raise RuntimeError(f'{label} {attribute}: expected {expected}, got {actual}')

    def focused_press(self, label):
        node = self.wait_find(TITLE, label, 'AXButton')
        try:
            self.set(node, 'AXFocused', self.true)
            self.perform(node, 'AXPress')
        finally:
            self.release(node)

    def open(self, label):
        self.focused_press(label)
        self.release(self.wait_find(TITLE, 'Cancel', 'AXButton'))

    def click_outside_control(self):
        # A real pointer down/up outside the popup, at a verified child-owned point.
        class Point(C.Structure):
            _fields_ = [('x', C.c_double), ('y', C.c_double)]
        node = self.wait_find(TITLE, 'Make it yours', 'AXStaticText')
        get = self.ax.AXValueGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        position, size = Point(), Point()
        try:
            for attribute, kind, target in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                value = self.attr(node, attribute)
                try:
                    if not value or not get(value, kind, C.byref(target)):
                        raise RuntimeError('Outside control lacks accessible bounds')
                finally:
                    if value:
                        self.release(value)
        finally:
            self.release(node)
        create = self.cg.CGEventCreateMouseEvent
        create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, Point, C.c_int]
        center = Point(position.x + size.x / 2, position.y + size.y / 2)
        panel = self.wait_find(TITLE, 'Choose accent color')
        try:
            x, y, width, height = self.bounds(panel)
            assert not (x <= center.x <= x + width and y <= center.y <= y + height), 'Outside point is covered by the popup'
        finally:
            self.release(panel)
        system = self.ax.AXUIElementCreateSystemWide
        system.restype, system.argtypes = C.c_void_p, []
        hit_test = self.ax.AXUIElementCopyElementAtPosition
        hit_test.restype, hit_test.argtypes = C.c_int, [C.c_void_p, C.c_float, C.c_float, C.POINTER(C.c_void_p)]
        get_pid = self.ax.AXUIElementGetPid
        get_pid.restype, get_pid.argtypes = C.c_int, [C.c_void_p, C.POINTER(C.c_int)]
        root, hit, owner = system(), C.c_void_p(), C.c_int()
        try:
            if (hit_test(root, center.x, center.y, C.byref(hit)) or not hit.value
                    or get_pid(hit, C.byref(owner)) or owner.value != self.pid):
                raise RuntimeError('Test click is occluded by another application')
        finally:
            if hit.value:
                self.release(hit)
            self.release(root)
        post = self.cg.CGEventPost
        post.restype, post.argtypes = None, [C.c_int, C.c_void_p]
        print('OS_OUTSIDE_CLICK', center.x, center.y, flush=True)
        for kind in [1, 2]:
            event = create(None, kind, center, 0)
            try:
                post(0, event)
            finally:
                self.release(event)

    def popup_gone(self):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = self.find(TITLE, 'Cancel', 'AXButton')
            if not node:
                break
            self.release(node)
            time.sleep(.02)
        else:
            self.dump(TITLE)
            self.capture('failure-popup-remained')
            raise RuntimeError('Popup remained accessible after dismissal')

    def closed(self, label, *, restored=True):
        self.popup_gone()
        self.wait_text(TITLE, 'Confirmed: ' + label)
        if restored:
            self.button_state(label, 'AXFocused', True)


def exercise(mac):
    mac.wait_text(TITLE, 'Confirmed: ' + INITIAL)
    mac.open(INITIAL)
    mac.contained()
    mac.capture('color-picker-open')
    assert mac.field(TITLE, 'Hex color', 'AXTextField') == INITIAL
    for label, maximum in [('Hue', 360), ('Saturation', 100), ('Lightness', 100), ('Opacity', 100)]:
        node = mac.wait_find(TITLE, label, 'AXSlider')
        try:
            assert mac.number(node, 'AXMinValue') == 0
            assert mac.number(node, 'AXMaxValue') == maximum
        finally:
            mac.release(node)
    mac.field(TITLE, 'Hex color', 'AXTextField', '#ff000080')
    mac.key(36)  # Actual OS Enter commits native draft, not application value.
    mac.wait_text(TITLE, 'Confirmed: ' + INITIAL)
    mac.button_state('Apply', 'AXEnabled', True)
    mac.press(TITLE, 'Apply')
    mac.closed(SELECTED)

    mac.open(SELECTED)
    mac.field(TITLE, 'Hex color', 'AXTextField', '#12')
    mac.button_state('Apply', 'AXEnabled', False)
    mac.key(53)  # Popup dismissal takes precedence and discards the native draft.
    mac.closed(SELECTED)

    mac.open(SELECTED)
    node = mac.wait_find(TITLE, 'Hue', 'AXSlider')
    try:
        mac.set(node, 'AXFocused', mac.true)
        mac.perform(node, 'AXIncrement')
    finally:
        mac.release(node)
    mac.key(124)  # OS Right arrow on native channel rail.
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if mac.field(TITLE, 'Hue', 'AXTextField') == '2':
            break
        time.sleep(.02)
    else:
        raise RuntimeError('AX increment plus OS arrow did not adjust hue')
    mac.press(TITLE, 'Cancel')
    mac.closed(SELECTED)

    mac.open(SELECTED)
    mac.field(TITLE, 'Hex color', 'AXTextField', '#abcdef')
    mac.click_outside_control()
    mac.closed(SELECTED, restored=False)
    mac.press(TITLE, 'Right edge')
    mac.open(SELECTED)
    mac.contained(clamped_from_anchor=SELECTED)
    mac.capture('color-picker-right')
    mac.press(TITLE, 'Cancel')
    mac.closed(SELECTED)

    mac.press(TITLE, 'Read-only')
    mac.open(SELECTED)
    mac.button_state('Apply', 'AXEnabled', False)
    mac.press(TITLE, 'Cancel')
    mac.closed(SELECTED)
    mac.press(TITLE, 'Allow selection')
    mac.press(TITLE, 'Disable')
    mac.button_state(SELECTED, 'AXEnabled', False)
    mac.press(TITLE, 'Enable')
    mac.focused_press('Open dialog')
    mac.release(mac.wait_find(TITLE, 'Done', 'AXButton'))
    mac.open(SELECTED)
    mac.contained()
    mac.capture('color-picker-dialog')
    mac.press(TITLE, 'Clear color')
    mac.press(TITLE, 'Apply')
    mac.closed('Choose color')
    mac.press(TITLE, 'Done')
    mac.wait_text(TITLE, 'Confirmed: Choose color')
    mac.press(TITLE, 'Close')


def main():
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/color_input/picker.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT,
                                 text=True, start_new_session=True)
        mac = None
        try:
            mac = Picker(child.pid, child)
            exercise(mac)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Picker example exited unsuccessfully')
        finally:
            if mac:
                mac.release(mac.app)
            try:
                os.killpg(child.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
            log.seek(0)
            print(log.read(), end='')
    print('GPUIO_COLOR_PICKER_AX_OK: native field/slider AX, OS Enter/Escape/arrows, '
          'Apply/Cancel, outside dismissal, focus restore, readonly/disabled, nested dialog and clamped placement')


if __name__ == '__main__':
    main()
