#!/usr/bin/env python3
"""Settings integration through actual macOS AX, keyboard and clipboard input."""
import ctypes as C
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_review import Review, TITLE, CONVERSATION
from test_numeric import Numeric
from test_tree_outline import Point
from test_canvas import screenshot


class Settings(Review):
    number = Numeric.number

    def expect_field(self, label, expected):
        deadline = time.monotonic() + 8
        actual = None
        while time.monotonic() < deadline:
            actual = self.field(TITLE, label, 'AXTextField')
            if actual == expected:
                return
            time.sleep(.03)
        raise RuntimeError(f'{label}: expected {expected!r}, got {actual!r}')

    def slider(self, label, value=None, key=None):
        node = self.wait_find(TITLE, label, 'AXSlider')
        try:
            self.set(node, 'AXFocused', self.true)
            if value is not None:
                create = self.cf.CFNumberCreate
                create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, C.c_void_p]
                number = C.c_double(value)
                encoded = create(None, 13, C.byref(number))
                try:
                    self.set(node, 'AXValue', encoded)
                finally:
                    self.release(encoded)
            if key is not None:
                self.key(key)
        finally:
            self.release(node)

    def drag_cancel_interval(self):
        node = self.wait_find(TITLE, 'Stream interval', 'AXSlider')
        try:
            position, size = Point(), Point()
            for label, kind, target in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, label)
                try:
                    if not raw or not self.value(raw, kind, C.byref(target)):
                        raise RuntimeError('Missing slider thumb geometry')
                finally:
                    if raw:
                        self.release(raw)
            start = Point(position.x + size.x / 2, position.y + size.y / 2)
        finally:
            self.release(node)
        self.send(5, start)
        self.send(1, start)
        end = start
        try:
            for step in range(1, 9):
                end = Point(start.x + step * 10, start.y)
                self.send(6, end)
                time.sleep(.02)
            self.wait_text(TITLE, 'Preview:')
            self.key(53)
            self.wait_text(TITLE, 'Stream interval preview cancelled.')
        finally:
            self.send(2, end)
        self.wait_text(TITLE, 'Saved interval: 90 ms')

    def capture(self, name):
        directory = os.environ.get('GPUIO_SCREENSHOT_DIR')
        if directory:
            time.sleep(.2)
            screenshot(self, Path(directory) / name)

    def exercise(self):
        self.draft(TITLE, CONVERSATION, 'Settings leave my draft intact λ')
        self.set(self.app, 'AXFrontmost', self.true)
        window = self.window(TITLE)
        try:
            self.perform(window, 'AXRaise')
        finally:
            self.release(window)
        self.press(TITLE, 'Settings')
        self.wait_text(TITLE, 'Make it your workspace.')
        self.expect_field('Stream chunk size', '17')
        self.field(TITLE, 'Stream chunk size', 'AXTextField', '-')
        self.key(36)
        self.wait_text(TITLE, 'Finish the number, or press Escape to restore it.')
        self.key(53)  # Numeric Escape restores its draft before modal dismissal.
        self.expect_field('Stream chunk size', '17')
        self.field(TITLE, 'Stream chunk size', 'AXTextField', '64')
        self.key(36)
        self.wait_text(TITLE, 'Saved chunk size: 64 bytes')
        self.press(TITLE, 'Stacked steppers')
        self.press(TITLE, 'Stream chunk size increase')
        self.wait_text(TITLE, 'Saved chunk size: 65 bytes')
        self.press(TITLE, 'Keyboard only')
        node = self.wait_find(TITLE, 'Stream chunk size', 'AXTextField')
        try:
            self.set(node, 'AXFocused', self.true)
        finally:
            self.release(node)
        self.key(126)
        self.wait_text(TITLE, 'Saved chunk size: 66 bytes')
        self.slider('Stream interval', value=80)
        self.wait_text(TITLE, 'Saved interval: 80 ms')
        self.slider('Stream interval', key=124)
        self.wait_text(TITLE, 'Saved interval: 90 ms')
        self.drag_cancel_interval()
        self.slider('Minimum result score', value=80)
        self.slider('Maximum result score', value=90)
        self.wait_text(TITLE, 'Ready interval: 80–90%')
        self.press(TITLE, 'Apply score interval')
        self.wait_text(TITLE, 'Score filter applied.')
        self.capture('settings-generation-dark.png')
        self.press(TITLE, 'Close settings')
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.press(TITLE, 'Explore results')
        self.wait_text(TITLE, '6 results loaded')
        self.wait_text(TITLE, 'Filter: score 80–90%')
        self.press(TITLE, 'Close workspace inspector')
        self.press(TITLE, 'Settings')
        self.expect_field('Stream chunk size', '66')
        self.wait_text(TITLE, 'Saved interval: 90 ms')
        self.field(TITLE, 'Stream chunk size', 'AXTextField', '-')
        self.press(TITLE, 'Close settings')
        self.press(TITLE, 'Settings')
        self.expect_field('Stream chunk size', '66')
        self.press(TITLE, 'Reset generation preferences')
        self.wait_text(TITLE, 'Reset generation preferences?')
        self.press(TITLE, 'Keep preferences')
        self.expect_field('Stream chunk size', '66')
        self.press(TITLE, 'Reset generation preferences')
        self.press(TITLE, 'Reset preferences')
        self.expect_field('Stream chunk size', '17')
        self.wait_text(TITLE, 'Saved interval: 20 ms')
        self.wait_text(TITLE, 'Ready interval: 80–90%')
        self.press(TITLE, 'Connection demo')
        self.expect_field('Simulated connection code', '')
        subprocess.run(['pbcopy'], input=b'123-456', check=True)
        node = self.wait_find(TITLE, 'Simulated connection code', 'AXTextField')
        try:
            self.set(node, 'AXFocused', self.true)
        finally:
            self.release(node)
        self.key(9, 1 << 20)  # Command-V exercises native paste normalization.
        self.expect_field('Simulated connection code', '123456')
        self.wait_text(TITLE, 'Demo complete')
        self.capture('settings-connection-dark.png')
        self.press(TITLE, 'Clear simulated code')
        self.expect_field('Simulated connection code', '')
        self.wait_text(TITLE, '0 of 6 digits')
        self.field(TITLE, 'Simulated connection code', 'AXTextField', '12')
        self.press(TITLE, 'Generation')
        self.expect_field('Stream chunk size', '17')
        self.press(TITLE, 'Connection demo')
        self.expect_field('Simulated connection code', '12')
        self.press(TITLE, 'Close settings')
        assert self.draft(TITLE, CONVERSATION) == 'Settings leave my draft intact λ'
        self.press(TITLE, 'Light theme')
        self.press(TITLE, 'Settings')
        self.wait_text(TITLE, 'Make it your workspace.')
        self.capture('settings-generation-light.png')
        self.press(TITLE, 'Connection demo')
        self.capture('settings-connection-light.png')
        self.press(TITLE, 'Close settings')
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Settings walkthrough exceeded 150 seconds')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(150)
            mac = Settings(child.pid, child)
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
    print('GPUIO_CHAT_SETTINGS_APPKIT_OK: numeric draft/commit/cancel, stepper layouts, '
          'sliders and real query filter, remount retention, nested reset, native OTP paste/clear, '
          'theme, composer preservation and window cleanup')


if __name__ == '__main__':
    main()
