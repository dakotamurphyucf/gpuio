#!/usr/bin/env python3
"""Integrated chat canvas/navigation through real AppKit events; always reaps its app."""
import ctypes as C
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_review import Review, TITLE, CONVERSATION
from test_tree_outline import Point
from test_canvas import screenshot


class Diagram(Review):
    def stage_x(self, name):
        node = self.wait_find(TITLE, name, 'AXStaticText')
        try:
            value = self.attr(node, 'AXPosition')
            try:
                point = Point()
                if not value or not self.value(value, 1, C.byref(point)):
                    raise RuntimeError('Missing stage position')
                return point.x
            finally:
                if value:
                    self.release(value)
        finally:
            self.release(node)

    def focus_stage(self, name):
        boolean = self.cf.CFBooleanGetValue
        boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
        deadline = time.monotonic() + 15
        enabled_since = None
        while True:
            node = self.wait_find(TITLE, name, 'AXStaticText')
            value = self.attr(node, 'AXEnabled')
            try:
                enabled = value and boolean(value)
            finally:
                if value:
                    self.release(value)
            if enabled:
                if enabled_since is None:
                    enabled_since = time.monotonic()
                if time.monotonic() - enabled_since >= .12:
                    break
            else:
                enabled_since = None
            self.release(node)
            if time.monotonic() > deadline:
                raise RuntimeError(f'Canvas stage never became enabled: {name}')
            time.sleep(.05)
        try:
            self.set(node, 'AXFocused', self.true)
        finally:
            self.release(node)
        self.wait_text(TITLE, f'Selected: {name}')

    def drag_stage(self, name, dx, dy):
        # A completed move publishes connector geometry. Wait for native input
        # eligibility across that publication before beginning a new gesture.
        self.focus_stage(name)
        node = self.wait_find(TITLE, name, 'AXStaticText')
        try:
            position, size = Point(), Point()
            for label, kind, result in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, label)
                try:
                    if not raw or not self.value(raw, kind, C.byref(result)):
                        raise RuntimeError('Missing canvas stage geometry')
                finally:
                    if raw:
                        self.release(raw)
            if size.x <= 0 or size.y <= 0:
                raise RuntimeError('Empty canvas stage bounds')
            start = Point(position.x + size.x / 2, position.y + size.y / 2)
        finally:
            self.release(node)
        point = start
        self.send(5, start)
        self.send(1, start)
        try:
            for step in range(1, 11):
                point = Point(start.x + dx * step / 10, start.y + dy * step / 10)
                self.send(6, point)
                time.sleep(.02)
        finally:
            self.send(2, point)

    def exercise(self):
        self.draft(TITLE, CONVERSATION, 'Keep the conversation alive λ')
        self.set(self.app, 'AXFrontmost', self.true)
        window = self.window(TITLE)
        try:
            self.perform(window, 'AXRaise')
        finally:
            self.release(window)
        self.press(TITLE, 'Explore workspace')
        self.click_counter(0)
        self.wait_text(TITLE, '1 of 100 checkpoints')
        self.press(TITLE, 'Workspace')
        self.wait_text(TITLE, 'A closer look.')
        self.press(TITLE, 'Explore run diagram')
        self.wait_text(TITLE, 'Read sources · x 34 · y 42')
        self.focus_stage('Read sources')
        self.key(124, flags=1 << 17)
        self.wait_text(TITLE, 'Read sources · x 35 · y 42')
        self.drag_stage('Read sources', 20, 10)
        self.wait_text(TITLE, 'Read sources · x 55 · y 52')
        self.focus_stage('Read sources')
        start = self.stage_x('Read sources')
        self.key(124, flags=1 << 19)  # Alt+Right pans the native viewport.
        deadline = time.monotonic() + 10
        while abs(self.stage_x('Read sources') - start) < 19:
            if time.monotonic() > deadline:
                raise RuntimeError('Native keyboard pan did not move the stage on screen')
            time.sleep(.05)
        self.key(24)  # Native '+' zoom, no OCaml animation loop.
        self.wait_text(TITLE, 'Zoom 120%')
        self.press(TITLE, 'Reset view')
        self.wait_text(TITLE, 'Zoom 100%')
        images = os.environ.get('GPUIO_SCREENSHOT_DIR')
        if images:
            time.sleep(.2)
            screenshot(self, Path(images) / 'diagram-dark.png')
        self.focus_stage('Read sources')
        self.key(36)  # Native activation opens the selected stage.
        self.wait_text(TITLE, 'SIMULATED · NO FILES ARE MODIFIED')
        self.press(TITLE, 'Next stage')  # Replace current visit, don't add history.
        self.wait_text(TITLE, 'Prepare a patch from the source context, keeping the original files intact.')
        self.press(TITLE, 'Back')
        self.wait_text(TITLE, 'Read sources · x 55 · y 52')
        self.press(TITLE, 'Forward')
        self.wait_text(TITLE, 'Prepare a patch from the source context, keeping the original files intact.')
        self.press(TITLE, 'Review checkpoints')
        self.wait_text(TITLE, '1 of 100 checkpoints')
        self.press(TITLE, 'Diagram')
        self.wait_text(TITLE, 'Read sources · x 55 · y 52')
        self.press(TITLE, 'Close workspace inspector')
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, 'Read sources · x 55 · y 52')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        if images:
            time.sleep(.3)
            screenshot(self, Path(images) / 'diagram-light.png')
        crumb = self.wait_find(TITLE, 'Workspace', 'AXLink')
        try:
            self.perform(crumb, 'AXPress')
        finally:
            self.release(crumb)
        self.wait_text(TITLE, 'A closer look.')
        self.press(TITLE, 'Forward')
        self.wait_text(TITLE, 'Read sources · x 55 · y 52')
        assert self.draft(TITLE, CONVERSATION) == 'Keep the conversation alive λ'
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This test requires AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Chat diagram exceeded its 120-second budget')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(120)
            mac = Diagram(child.pid, child)
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
    print('GPUIO_CHAT_DIAGRAM_APPKIT_OK: native pointer/keyboard movement, pan/zoom, activation, '
          'history/replacement/breadcrumbs, retained review/diagram/draft, themes and window cleanup')


if __name__ == '__main__':
    main()
