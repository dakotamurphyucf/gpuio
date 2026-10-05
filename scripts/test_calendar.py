#!/usr/bin/env python3
"""Inline calendar public app: AppKit values/actions and OS keyboard behavior."""
import ctypes as C
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
from test_agent_chat import Mac

TITLE = 'GPUIO calendar lab'
SINGLE = 'Appointment date'
RANGE = 'Travel dates'
SHIFT = 1 << 17


class Calendars(Mac):
    def within(self, group, label, role=None):
        root = self.find(TITLE, group, 'AXGroup')
        if not root:
            return None
        def visit(node):
            if (self.text(node, 'AXTitle') == label
                    and (role is None or self.text(node, 'AXRole') == role)):
                return self.retain(node)
            children = self.children(node)
            try:
                for child in children:
                    found = visit(child)
                    if found:
                        return found
            finally:
                for child in children:
                    self.release(child)
            return None
        try:
            return visit(root)
        finally:
            self.release(root)

    def cell(self, label, group=SINGLE):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = self.within(group, label)
            if node:
                return node
            time.sleep(.02)
        self.dump(TITLE)
        raise RuntimeError(f'Missing {label!r} in {group}')

    def button(self, label, group=SINGLE, *, focus=False):
        node = self.cell(label, group)
        try:
            if focus:
                self.set(node, 'AXFocused', self.true)
            else:
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

    def dump_focus(self):
        root = self.window(TITLE)
        def visit(node):
            focused = self.attr(node, 'AXFocused')
            if focused:
                try:
                    get = self.cf.CFBooleanGetValue
                    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
                    if get(focused):
                        print('AX_FOCUSED', self.text(node, 'AXRole'), self.text(node, 'AXTitle'), flush=True)
                finally:
                    self.release(focused)
            children = self.children(node)
            try:
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    self.release(child)
        if root:
            try:
                visit(root)
            finally:
                self.release(root)

    def state(self, label, attribute, expected, group=SINGLE):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = self.cell(label, group)
            try:
                actual = self.boolean(node, attribute)
                if actual == expected:
                    return
            finally:
                self.release(node)
            time.sleep(.02)
        self.dump_focus()
        raise RuntimeError(f'{label}: {attribute} expected {expected}, got {actual}')

    def value(self, expected, group=SINGLE):
        deadline = time.monotonic() + 5
        actual = None
        while time.monotonic() < deadline:
            node = self.find(TITLE, group, 'AXGroup')
            if node:
                try:
                    actual = self.text(node, 'AXValue')
                    if actual == expected:
                        return
                finally:
                    self.release(node)
            time.sleep(.02)
        self.dump(TITLE)
        raise RuntimeError(f'{group}: expected value {expected!r}, got {actual!r}')


def exercise(mac):
    mac.value('2024-02-29')
    mac.state('February 29, 2024, Today', 'AXValue', True)
    mac.button('February 28, 2024', focus=True)  # Reveal without selecting.
    mac.state('February 28, 2024', 'AXFocused', True)
    mac.value('2024-02-29')
    mac.key(124)  # Leap day.
    mac.state('February 29, 2024, Today', 'AXFocused', True)
    mac.key(124)  # March boundary.
    mac.state('March 1, 2024', 'AXFocused', True)
    mac.key(36)
    mac.value('2024-03-01')
    mac.state('March 1, 2024', 'AXValue', True)
    mac.button('March 5, 2024', focus=True)
    # AX requests and CG keys use different queues. Acknowledge the starting
    # cursor before sending ordered Right/Return keys; the resulting rejection
    # proves activation reached the disabled March 6 date.
    mac.state('March 5, 2024', 'AXFocused', True)
    mac.key(124)
    mac.state('March 6, 2024', 'AXEnabled', False)
    mac.key(36)
    mac.wait_text(TITLE, 'Disabled_date')
    mac.value('2024-03-01')
    mac.button('Choose month')
    mac.button('February')
    mac.value('2024-03-01')  # Navigation is not selection.
    mac.button('Choose year')
    mac.button('2025')
    mac.button('February')
    mac.button('February 28, 2025')
    mac.value('2025-02-28')
    mac.key(116, SHIFT)  # Shift+PageUp reveals February 2024.
    mac.state('February 28, 2024', 'AXFocused', True)
    mac.value('2025-02-28')
    mac.key(115)  # Home -> Monday February 26.
    mac.state('February 26, 2024', 'AXFocused', True)
    mac.key(119)  # End -> Sunday March 3.
    mac.state('March 3, 2024', 'AXFocused', True)
    mac.key(48)  # Composite calendar is one Tab stop; next is the range.
    mac.state('February 1, 2024', 'AXFocused', True, RANGE)
    mac.key(36)
    mac.value('2024-02-01 – …', RANGE)
    mac.key(124)
    mac.key(36)
    mac.value('2024-02-01 – 2024-02-02', RANGE)
    mac.key(48, SHIFT)  # Shift+Tab returns to the single calendar.
    mac.state('March 3, 2024', 'AXFocused', True)
    mac.key(36)
    mac.value('2024-03-03')
    mac.press(TITLE, 'Read-only')
    mac.button('March 4, 2024', focus=True)
    mac.key(36)
    mac.value('2024-03-03')
    mac.state('March 4, 2024', 'AXEnabled', False)
    mac.button('Choose month')  # Read-only still permits browsing.
    mac.button('April')
    mac.value('2024-03-03')
    mac.press(TITLE, 'Disable')
    mac.press(TITLE, 'Leap day')  # Explicit replacement remains allowed.
    mac.value('2024-02-29')
    mac.press(TITLE, 'Enable')
    mac.press(TITLE, 'Allow selection')
    mac.press(TITLE, 'Unmount')
    mac.release(mac.wait_find(TITLE, 'Remount', 'AXButton'))
    if node := mac.find(TITLE, SINGLE, 'AXGroup'):
        mac.release(node)
        raise RuntimeError('Unmounted calendar remains accessible')
    mac.press(TITLE, 'Remount')
    mac.value('2024-02-29')
    mac.button('Clear')
    mac.value('')
    mac.press(TITLE, 'Close')


def main():
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/calendar/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT,
                                 text=True, start_new_session=True)
        mac = None
        try:
            mac = Calendars(child.pid, child)
            exercise(mac)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Calendar example exited unsuccessfully')
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
    print('GPUIO_CALENDAR_AX_OK: values, active cursor, selection, month/year actions, '
          'OS keys, civil boundaries, Tab order, range, policy, remount and close')


if __name__ == '__main__':
    main()
