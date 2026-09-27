#!/usr/bin/env python3
"""First M5 chat flow: independent native review component and inspector lifetime.

This does not claim full showcase acceptance. Input stays within the spawned
application; the total deadline and cleanup close/reap it on every handled exit.
"""
import ctypes as C
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_tree_outline import Outline, Point
from test_canvas import screenshot

TITLE = 'GPUIO · Agent workspace 1'
CONVERSATION = 'Build a native app'


class Review(Outline):
    def click_counter(self, value):
        node = self.wait_find(TITLE, f'Increment counter, current value {value}', 'AXButton')
        try:
            position, size = Point(), Point()
            for name, kind, result in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, name)
                try:
                    if not raw or not self.value(raw, kind, C.byref(result)):
                        raise RuntimeError('Native component has no accessible geometry')
                finally:
                    if raw:
                        self.release(raw)
            if size.x <= 0 or size.y <= 0:
                raise RuntimeError('Native component has empty bounds')
            point = Point(position.x + size.x / 2, position.y + size.y / 2)
        finally:
            self.release(node)
        self.send(5, point)
        self.send(1, point)
        try:
            time.sleep(.03)
        finally:
            self.send(2, point)

    def exercise(self):
        self.draft(TITLE, CONVERSATION, 'Draft survives workspace inspection λ')
        self.set(self.app, 'AXFrontmost', self.true)
        window = self.window(TITLE)
        try:
            self.perform(window, 'AXRaise')
        finally:
            self.release(window)
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, '0 of 100 checkpoints')
        self.click_counter(0)
        self.wait_text(TITLE, '1 of 100 checkpoints')
        self.key(49)  # Space targets the focused native component through AppKit.
        self.wait_text(TITLE, '2 of 100 checkpoints')
        self.press(TITLE, '+5 per checkpoint')
        self.click_counter(2)
        self.wait_text(TITLE, '7 of 100 checkpoints')
        images = os.environ.get('GPUIO_SCREENSHOT_DIR')
        if images:
            time.sleep(.2)  # Let the compositor present the AX-observed frame.
            screenshot(self, Path(images) / 'review-dark.png')
        self.press(TITLE, 'Close workspace inspector')
        # Hidden native content must disappear from macOS semantics.
        deadline = time.monotonic() + 10
        while True:
            node = self.find(TITLE, 'Increment counter, current value 7', 'AXButton')
            if not node:
                break
            self.release(node)
            if time.monotonic() > deadline:
                raise RuntimeError('Closed inspector retains native counter semantics')
            time.sleep(.05)
        assert self.draft(TITLE, CONVERSATION) == 'Draft survives workspace inspection λ'
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, '7 of 100 checkpoints')
        self.click_counter(7)
        self.wait_text(TITLE, '12 of 100 checkpoints')
        self.press(TITLE, 'Reset review')
        self.wait_text(TITLE, 'Review reset')  # Requires the native command acknowledgement.
        self.wait_text(TITLE, '0 of 100 checkpoints')
        self.click_counter(0)
        self.wait_text(TITLE, '5 of 100 checkpoints')
        self.press(TITLE, 'Close workspace inspector')
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, '5 of 100 checkpoints')
        self.click_counter(5)  # An acknowledged reset must not replay on remount.
        self.wait_text(TITLE, '10 of 100 checkpoints')
        self.press(TITLE, 'Reset review')
        self.wait_text(TITLE, 'Review reset')
        self.wait_text(TITLE, '0 of 100 checkpoints')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        if images:
            time.sleep(.2)
            screenshot(self, Path(images) / 'review-light.png')
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This test exercises AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Chat review exceeded its 120-second budget')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(120)
            mac = Review(child.pid, child)
            mac.exercise()
            if child.wait(timeout=15):
                raise RuntimeError('Chat example failed')
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
    print('GPUIO_CHAT_REVIEW_APPKIT_OK: separate native component pointer/keyboard events, '
          'properties, acknowledged reset, unmount/reopen, draft preservation, themes and window close')


if __name__ == '__main__':
    main()
