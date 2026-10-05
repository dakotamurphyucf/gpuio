#!/usr/bin/env python3
"""Civil dates, review pagination and actual canvas annotation settings on macOS."""
import ctypes as C
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_settings import Settings
from test_agent_chat_review import TITLE, CONVERSATION
from test_tree_outline import Point


class DatesColors(Settings):
    def scroll_settings(self, pixels):
        window = self.window(TITLE)
        try:
            position, size = Point(), Point()
            for label, kind, target in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(window, label)
                try:
                    if not raw or not self.value(raw, kind, C.byref(target)):
                        raise RuntimeError('Missing settings window bounds')
                finally:
                    if raw:
                        self.release(raw)
            point = Point(position.x + size.x * .85, position.y + size.y * .65)
        finally:
            self.release(window)
        self.send(5, point)  # Verify this screen point belongs to our child.
        wheel = self.cg.CGEventCreateScrollWheelEvent
        wheel.restype, wheel.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
        location = self.cg.CGEventSetLocation
        location.restype, location.argtypes = None, [C.c_void_p, Point]
        event = wheel(None, 0, 1, pixels)
        if not event:
            raise RuntimeError('Cannot create settings scroll event')
        try:
            location(event, point)
            self.post(0, event)
        finally:
            self.release(event)
        time.sleep(.2)  # Let native scrolling present before capturing.

    def find(self, title, label, role=None, contains=False, search_files=False, *, deadline=None):
        return super().find(title, label, role, contains, search_files=True, deadline=deadline)

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
        try:
            return visit(root)
        finally:
            self.release(root)

    def day(self, group, number, *, enabled=True, month='October'):
        label = f'{month} {number}, 2026'
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            node = self.within(group, label, 'AXCheckBox')
            if node:
                try:
                    assert self.boolean(node, 'AXEnabled') == enabled, label
                    if enabled:
                        self.perform(node, 'AXPress')
                    return
                finally:
                    self.release(node)
            time.sleep(.03)
        self.dump(TITLE)
        raise RuntimeError(f'Missing {label} in {group}')

    def boolean(self, node, attribute):
        raw = self.attr(node, attribute)
        if not raw:
            raise RuntimeError(f'Missing {attribute}')
        try:
            fn = self.cf.CFBooleanGetValue
            fn.restype, fn.argtypes = C.c_bool, [C.c_void_p]
            return bool(fn(raw))
        finally:
            self.release(raw)

    def button_state(self, label, attribute, expected):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            node = self.wait_find(TITLE, label, 'AXButton')
            try:
                if self.boolean(node, attribute) == expected:
                    return
            finally:
                self.release(node)
            time.sleep(.03)
        raise RuntimeError(f'{label}: {attribute} did not become {expected}')

    def open_picker(self, label, cancel):
        node = self.wait_find(TITLE, label, 'AXButton')
        try:
            self.set(node, 'AXFocused', self.true)
            self.perform(node, 'AXPress')
        finally:
            self.release(node)
        self.release(self.wait_find(TITLE, cancel, 'AXButton'))

    def picker_closed(self, cancel, trigger):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            node = self.find(TITLE, cancel, 'AXButton')
            if not node:
                self.button_state(trigger, 'AXFocused', True)
                return
            self.release(node)
            time.sleep(.03)
        raise RuntimeError(f'{cancel} remains accessible after closing')

    def exercise(self):
        self.draft(TITLE, CONVERSATION, 'Civil dates and colors preserve my draft λ')
        self.set(self.app, 'AXFrontmost', self.true)
        self.press(TITLE, 'Settings')
        self.press(TITLE, 'Dates & reviews')
        self.wait_text(TITLE, '21 sample reviews · page 1 of 4')
        self.press(TITLE, 'Last reviews')
        self.wait_text(TITLE, '21 sample reviews · page 4 of 4')
        self.day('Review dates', 3, enabled=False)
        self.day('Review dates', 20, enabled=False)
        self.day('Review dates', 30, month='September', enabled=False)
        self.day('Review dates', 5)
        self.wait_text(TITLE, 'Date draft: 2026-10-05 → choose an end date')
        self.button_state('Apply review dates', 'AXEnabled', False)
        self.day('Review dates', 9)
        self.wait_text(TITLE, 'Date draft: 2026-10-05 – 2026-10-09')
        self.press(TITLE, 'Apply review dates')
        self.wait_text(TITLE, '5 sample reviews · page 1 of 1')
        self.press(TITLE, 'Single review day')
        self.wait_text(TITLE, 'Date draft: No date selected')
        self.day('Review dates', 7)
        self.wait_text(TITLE, 'Date draft: 2026-10-07')
        self.key(124)
        self.key(36)
        self.wait_text(TITLE, 'Date draft: 2026-10-08')
        self.press(TITLE, 'Apply review dates')
        self.wait_text(TITLE, '1 sample review · page 1 of 1')
        self.scroll_settings(-390)
        self.capture('settings-review-calendar-dark.png')
        self.scroll_settings(1000)
        self.press(TITLE, 'All review dates')
        self.wait_text(TITLE, '21 sample reviews · page 1 of 4')
        self.open_picker('Choose follow-up date', 'Cancel follow-up')
        self.day('Follow-up date', 14)
        self.wait_text(TITLE, 'Saved follow-up: No date selected')
        self.capture('settings-date-picker-dark.png')
        self.press(TITLE, 'Cancel follow-up')
        self.picker_closed('Cancel follow-up', 'Choose follow-up date')
        self.open_picker('Choose follow-up date', 'Cancel follow-up')
        self.day('Follow-up date', 15)
        self.press(TITLE, 'Save follow-up')
        self.wait_text(TITLE, 'Saved follow-up: 2026-10-15')
        self.picker_closed('Cancel follow-up', 'Choose follow-up date')
        self.open_picker('Choose follow-up date', 'Cancel follow-up')
        self.day('Follow-up date', 16)
        self.key(53)
        self.picker_closed('Cancel follow-up', 'Choose follow-up date')
        self.wait_text(TITLE, 'Saved follow-up: 2026-10-15')
        self.press(TITLE, 'Annotation color')
        self.wait_text(TITLE, 'Confirmed annotation: Theme accent')
        self.open_picker('Choose annotation color', 'Cancel annotation')
        swatch = self.wait_find(TITLE, 'Rose', 'AXRadioButton')
        try:
            self.perform(swatch, 'AXPress')
        finally:
            self.release(swatch)
        self.wait_text(TITLE, 'Preview annotation: #EF6B9580')
        self.wait_text(TITLE, 'Confirmed annotation: Theme accent')
        self.press(TITLE, 'Cancel annotation')
        self.picker_closed('Cancel annotation', 'Choose annotation color')
        self.open_picker('Choose annotation color', 'Cancel annotation')
        self.field(TITLE, 'Hex color', 'AXTextField', '#FF000080')
        self.key(36)
        self.wait_text(TITLE, 'Preview annotation: #FF000080')
        self.wait_text(TITLE, 'Confirmed annotation: Theme accent')
        self.capture('settings-color-picker-dark.png')
        self.press(TITLE, 'Apply annotation')
        self.wait_text(TITLE, 'Confirmed annotation: #FF000080')
        self.picker_closed('Cancel annotation', 'Choose annotation color')
        self.open_picker('Choose annotation color', 'Cancel annotation')
        self.expect_field('Opacity', '50.2')
        self.field(TITLE, 'Hex color', 'AXTextField', '#zz')
        self.button_state('Apply annotation', 'AXEnabled', False)
        self.key(53)
        self.picker_closed('Cancel annotation', 'Choose annotation color')
        self.wait_text(TITLE, 'Confirmed annotation: #FF000080')
        self.open_picker('Choose annotation color', 'Cancel annotation')
        self.slider('Opacity', value=25)
        self.wait_text(TITLE, 'Preview annotation: #FF000040')
        self.slider('Hue', key=124)
        self.button_state('Apply annotation', 'AXEnabled', True)
        self.press(TITLE, 'Cancel annotation')
        self.picker_closed('Cancel annotation', 'Choose annotation color')
        self.wait_text(TITLE, 'Confirmed annotation: #FF000080')
        self.capture('settings-annotation-dark.png')
        self.press(TITLE, 'Close settings')
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.press(TITLE, 'Explore run diagram')
        self.wait_text(TITLE, 'Diagram annotation: #FF000080')
        self.wait_text(TITLE, 'Select a stage to inspect the run')
        self.capture('diagram-annotation-dark.png')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        self.wait_text(TITLE, 'Diagram annotation: #FF000080')
        self.capture('diagram-annotation-light.png')
        self.press(TITLE, 'Settings')
        self.press(TITLE, 'Dates & reviews')
        self.wait_text(TITLE, 'Saved follow-up: 2026-10-15')
        self.open_picker('Choose follow-up date', 'Cancel follow-up')
        self.capture('settings-date-picker-light.png')
        self.press(TITLE, 'Cancel follow-up')
        self.picker_closed('Cancel follow-up', 'Choose follow-up date')
        self.press(TITLE, 'Annotation color')
        self.wait_text(TITLE, 'Confirmed annotation: #FF000080')
        self.open_picker('Choose annotation color', 'Cancel annotation')
        self.capture('settings-color-picker-light.png')
        self.press(TITLE, 'Cancel annotation')
        self.picker_closed('Cancel annotation', 'Choose annotation color')
        self.press(TITLE, 'Use theme accent')
        self.wait_text(TITLE, 'Confirmed annotation: Theme accent')
        self.press(TITLE, 'Close settings')
        self.wait_text(TITLE, 'Diagram annotation: Theme accent')
        self.press(TITLE, 'Close workspace inspector')
        assert self.draft(TITLE, CONVERSATION) == 'Civil dates and colors preserve my draft λ'
        self.press(TITLE, 'Settings')
        self.press(TITLE, 'Annotation color')
        self.open_picker('Choose annotation color', 'Cancel annotation')
        self.close(TITLE)  # Closing with a nested picker also releases its native draft.
        self.press(TITLE, 'Discard drafts and close')


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Dates/colors walkthrough exceeded 180 seconds')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(180)
            mac = DatesColors(child.pid, child)
            mac.exercise()
            if child.wait(timeout=15):
                raise RuntimeError('Chat exited unsuccessfully')
        finally:
            signal.alarm(0)
            if mac:
                mac.release(mac.app)
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            log.seek(0)
            print(log.read(), end='')
    print('GPUIO_CHAT_DATES_COLORS_APPKIT_OK: civil single/range/partial/disabled bounds, '
          'actual review filtering/page clamp, date confirm/cancel/focus, color swatches/hex/'
          'alpha/channel/malformed/cancel, actual diagram annotation/theme/reset, retained '
          'values, composer preservation and nested-picker window cleanup')


if __name__ == '__main__':
    main()
