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

    def open(self, label):
        node = self.wait_find(TITLE, label, 'AXButton')
        try:
            self.set(node, 'AXFocused', self.true)
            self.perform(node, 'AXPress')
        finally:
            self.release(node)
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

    def closed(self, label, *, restored=True):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = self.find(TITLE, 'Cancel', 'AXButton')
            if not node:
                break
            self.release(node)
            time.sleep(.02)
        else:
            raise RuntimeError('Popup remained accessible after dismissal')
        self.wait_text(TITLE, 'Confirmed: ' + label)
        if restored:
            self.button_state(label, 'AXFocused', True)


def exercise(mac):
    mac.wait_text(TITLE, 'Confirmed: ' + INITIAL)
    mac.open(INITIAL)
    mac.press(TITLE, 'February 20, 2024')
    mac.button_state('Apply', 'AXEnabled', False)
    mac.press(TITLE, 'February 22, 2024')
    mac.button_state('Apply', 'AXEnabled', True)
    mac.press(TITLE, 'Cancel')
    mac.closed(INITIAL)
    mac.open(INITIAL)
    mac.press(TITLE, 'February 20, 2024')
    mac.press(TITLE, 'February 22, 2024')
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
    mac.press(TITLE, 'Apply')
    mac.closed('2024-02-21 – 2024-02-22')
    mac.open('2024-02-21 – 2024-02-22')
    mac.press(TITLE, 'Clear')
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
          'outside pointer, restored trigger focus, OS day navigation, clear, readonly and disabled')


if __name__ == '__main__':
    main()
