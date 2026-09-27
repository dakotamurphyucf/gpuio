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

TITLE = 'GPUIO date picker'
INITIAL = '2024-02-27 – 2024-02-29'
SELECTED = '2024-02-20 – 2024-02-22'


class Picker(Mac):
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
        panel = self.wait_find(TITLE, 'Choose travel dates')
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

    def day(self, label):
        node = self.wait_find(TITLE, label, 'AXCheckBox')
        try:
            assert self.text(node, 'AXSubrole') == 'AXToggle'
            self.perform(node, 'AXPress')
        finally:
            self.release(node)

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
        node = self.wait_find(TITLE, 'Read-only', 'AXButton')
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
    mac.day('February 20, 2024')
    mac.button_state('Apply', 'AXEnabled', False)
    mac.day('February 22, 2024')
    mac.button_state('Apply', 'AXEnabled', True)
    mac.press(TITLE, 'Cancel')
    mac.closed(INITIAL)
    mac.open(INITIAL)
    mac.day('February 20, 2024')
    mac.button_state('Apply', 'AXEnabled', False)
    mac.day('February 22, 2024')
    mac.button_state('Apply', 'AXEnabled', True)
    mac.press(TITLE, 'Apply')
    mac.closed(SELECTED)
    mac.open(SELECTED)
    mac.key(53)  # Actual OS Escape; the calendar propagates to the overlay.
    mac.closed(SELECTED)
    mac.open(SELECTED)
    mac.click_outside_control()
    mac.closed(SELECTED, restored=False)
    mac.button_state('Allow selection', 'AXFocused', True)
    mac.press(TITLE, 'Allow selection')
    mac.open(SELECTED)
    mac.key(124)  # Right from the range start, February 20; Enter starts a new range.
    mac.key(36)
    mac.button_state('Apply', 'AXEnabled', False)
    mac.key(124)
    mac.key(36)
    mac.button_state('Apply', 'AXEnabled', True)
    mac.button_state('Apply', 'AXEnabled', True)
    mac.press(TITLE, 'Apply')
    mac.closed('2024-02-21 – 2024-02-22')
    mac.open('2024-02-21 – 2024-02-22')
    mac.press(TITLE, 'Clear')
    mac.button_state('Apply', 'AXEnabled', True)
    mac.press(TITLE, 'Apply')
    mac.closed('Choose dates')
    mac.press(TITLE, 'Reset dates')
    mac.wait_text(TITLE, 'Confirmed: ' + INITIAL)
    mac.press(TITLE, 'Read-only')
    mac.open(INITIAL)
    mac.button_state('Apply', 'AXEnabled', False)
    mac.press(TITLE, 'Cancel')
    mac.closed(INITIAL)
    mac.press(TITLE, 'Disable')
    mac.button_state(INITIAL, 'AXEnabled', False)
    mac.press(TITLE, 'Enable')
    mac.button_state(INITIAL, 'AXEnabled', True)
    mac.press(TITLE, 'Allow selection')
    mac.press(TITLE, 'Single date')
    mac.wait_text(TITLE, 'Confirmed: 2024-02-29')
    mac.open('2024-02-29')
    mac.day('March 1, 2024')
    mac.wait_text(TITLE, 'Confirmed: 2024-02-29')  # A native selection remains a draft.
    mac.button_state('Apply', 'AXEnabled', True)
    mac.contained()
    mac.capture('single-draft')
    mac.button_state('Apply', 'AXEnabled', True)
    mac.press(TITLE, 'Apply')
    mac.closed('2024-03-01')
    mac.open('2024-03-01')
    mac.day('February 21, 2024')
    mac.press(TITLE, 'Cancel')
    mac.closed('2024-03-01')
    mac.focused_press('Open dialog')
    mac.release(mac.wait_find(TITLE, 'Done', 'AXButton'))
    mac.open('2024-03-01')
    mac.contained()
    mac.capture('nested-dialog')
    mac.key(53)  # First Escape closes only the picker.
    mac.popup_gone()
    mac.release(mac.wait_find(TITLE, 'Done', 'AXButton'))
    mac.button_state('2024-03-01', 'AXFocused', True)
    mac.key(53)  # Second Escape closes the containing dialog.
    mac.button_state('Open dialog', 'AXFocused', True)
    mac.wait_text(TITLE, 'Confirmed: 2024-03-01')
    mac.focused_press('Open dialog')
    mac.release(mac.wait_find(TITLE, 'Done', 'AXButton'))
    mac.open('2024-03-01')
    mac.day('February 20, 2024')
    mac.button_state('Apply', 'AXEnabled', True)
    mac.press(TITLE, 'Apply')
    mac.popup_gone()
    mac.button_state('2024-02-20', 'AXFocused', True)
    mac.press(TITLE, 'Done')
    mac.wait_text(TITLE, 'Confirmed: 2024-02-20')
    mac.button_state('Open dialog', 'AXFocused', True)
    mac.press(TITLE, 'Right edge')
    mac.open('2024-02-20')
    mac.contained(clamped_from_anchor='2024-02-20')
    mac.capture('right-edge')
    mac.key(53)
    mac.closed('2024-02-20')
    mac.press(TITLE, 'Left edge')
    mac.press(TITLE, 'Date range')
    mac.wait_text(TITLE, 'Confirmed: ' + INITIAL)
    mac.open(INITIAL)
    mac.press(TITLE, 'Cancel')
    mac.closed(INITIAL)
    mac.press(TITLE, 'Close')


def main():
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/calendar/picker.exe')],
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
    print('GPUIO_DATE_PICKER_AX_OK: partial/complete, Apply/Cancel, Escape, '
          'outside pointer, restored trigger focus, OS day navigation, single/range modes, nested dialog, clamped placement, clear, readonly and disabled')


if __name__ == '__main__':
    main()
