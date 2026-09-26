#!/usr/bin/env python3
"""Local AppKit source-explorer acceptance; simulated data, owned app cleanup."""
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


class Sources(Review):
    def check_row_budget(self):
        tree = self.wait_find(TITLE, 'Sample workspace sources', 'AXOutline', search_files=True)
        def count_rows(node):
            count = int(self.text(node, 'AXRole') == 'AXRow')
            children = self.children(node)
            try:
                return count + sum(count_rows(child) for child in children)
            finally:
                for child in children:
                    self.release(child)
        try:
            count = count_rows(tree)
        finally:
            self.release(tree)
        if not 0 < count <= 24:
            raise RuntimeError(f'100k source fixture exceeds its 24-row AX budget: {count}')
        print(f'CHAT_SOURCE_NATIVE_ROWS={count} dataset=100000 budget=24', flush=True)

    def row_center(self, name):
        node = self.wait_find(TITLE, name, 'AXRow', search_files=True)
        try:
            position, size = Point(), Point()
            for label, kind, result in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, label)
                try:
                    if not raw or not self.value(raw, kind, C.byref(result)):
                        raise RuntimeError('Missing source row geometry')
                finally:
                    if raw:
                        self.release(raw)
            if size.x <= 0 or size.y <= 0:
                raise RuntimeError('Source row is not visible')
            return Point(position.x + size.x / 2, position.y + size.y / 2)
        finally:
            self.release(node)

    def drag(self, name, destination):
        start, end = self.row_center(name), self.row_center(destination)
        point = start
        self.send(5, start)
        self.send(1, start)
        try:
            for step in range(1, 21):
                point = Point(start.x + (end.x - start.x) * step / 20,
                              start.y + (end.y - start.y) * step / 20)
                self.send(6, point)
                time.sleep(.02)
        finally:
            self.send(2, point)

    def row_action(self, name, action=None, expanded=None):
        node = self.wait_find(TITLE, name, 'AXRow', search_files=True)
        try:
            if expanded is not None:
                self.set(node, 'AXExpanded', self.true if expanded else
                         C.c_void_p.in_dll(self.cf, 'kCFBooleanFalse').value)
            elif action:
                self.perform(node, action)
            else:
                self.set(node, 'AXFocused', self.true)
        finally:
            self.release(node)

    def menu(self, label):
        node = self.wait_find(TITLE, label, search_files=True)
        try:
            self.perform(node, 'AXPress')
        finally:
            self.release(node)

    def row_menu(self, name):
        row = self.wait_find(TITLE, name, 'AXRow', search_files=True)
        def visit(node):
            if self.text(node, 'AXRole') == 'AXButton' and self.text(node, 'AXTitle') == 'Actions':
                self.perform(node, 'AXPress')
                return True
            children = self.children(node)
            try:
                return any(visit(child) for child in children)
            finally:
                for child in children:
                    self.release(child)
        try:
            if not visit(row):
                raise RuntimeError(f'Missing source actions for {name}')
        finally:
            self.release(row)

    def exercise(self):
        self.draft(TITLE, CONVERSATION, 'Sources stay beside this draft λ')
        self.set(self.app, 'AXFrontmost', self.true)
        window = self.window(TITLE)
        try:
            self.perform(window, 'AXRaise')
        finally:
            self.release(window)
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.press(TITLE, 'Explore sources')
        self.wait_text(TITLE, '6 loaded')
        self.row_action('App.ml', 'AXPress')
        self.wait_text(TITLE, 'Selected: App.ml')
        self.row_action('App.ml')
        self.key(36)
        self.wait_text(TITLE, 'Source note: The simulated application entry point.')
        self.key(17)  # 't': native typeahead selects Theme.ml.
        self.wait_text(TITLE, 'Selected: Theme.ml')
        self.key(125, flags=1 << 17)  # Shift+Down extends selection through README.md.
        self.wait_text(TITLE, '2 selected')
        self.row_action('Research notes', expanded=True)
        self.menu('Retry')
        self.wait_text(TITLE, '8 loaded')
        self.row_action('App.ml', 'AXPress')
        self.drag('App.ml', 'Archive')
        self.wait_text(TITLE, 'Move this sample source?')
        self.press(TITLE, 'Cancel move')
        self.wait_text(TITLE, 'Move cancelled.')
        self.row_menu('App.ml')
        self.menu('Archive source')
        self.press(TITLE, 'Confirm move')
        self.wait_text(TITLE, 'Sample source moved.')
        self.press(TITLE, 'Reveal main source')
        self.row_action('App.ml', 'AXPress')
        self.wait_text(TITLE, 'Location: Archive')
        images = os.environ.get('GPUIO_SCREENSHOT_DIR')
        if images:
            time.sleep(.2)
            screenshot(self, Path(images) / 'sources-dark.png')
        self.press(TITLE, 'Diagram')
        self.press(TITLE, 'Back')
        self.wait_text(TITLE, 'Selected: App.ml')
        self.wait_text(TITLE, 'Location: Archive')
        self.press(TITLE, 'Close workspace inspector')
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, 'Location: Archive')
        self.press(TITLE, 'Load 100,000 sources')
        self.wait_text(TITLE, '100,000 loaded')
        self.press(TITLE, 'Reveal last source')
        self.row_action('Source 099997', 'AXPress')
        self.wait_text(TITLE, 'Selected: Source 099997')
        self.check_row_budget()
        self.press(TITLE, 'Empty workspace')
        self.wait_text(TITLE, '0 loaded')
        self.wait_text(TITLE, 'No sources yet')
        self.wait_text(TITLE, 'No sources in this sample workspace.')
        self.press(TITLE, 'Sample sources')
        self.wait_text(TITLE, '6 loaded')
        self.row_action('Research notes', expanded=True)
        loading = self.wait_find(TITLE, 'Loading…', search_files=True)
        self.release(loading)
        self.row_action('Research notes', expanded=False)
        time.sleep(.45)  # Past the deterministic 300ms producer's completion.
        self.wait_text(TITLE, '6 loaded')
        self.row_action('Research notes', expanded=True)
        self.wait_text(TITLE, '8 loaded')  # New request succeeds; cancelled work did not append.
        self.row_action('App.ml', 'AXPress')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        if images:
            time.sleep(.2)
            screenshot(self, Path(images) / 'sources-light.png')
        assert self.draft(TITLE, CONVERSATION) == 'Sources stay beside this draft λ'
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This test requires AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Chat sources exceeded its 150-second budget')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(150)
            mac = Sources(child.pid, child)
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
    print('GPUIO_CHAT_SOURCES_APPKIT_OK: lazy loading/failure/retry/collapse cancellation, native selection/range/typeahead, '
          'native drag/context move cancel/approve, reveal, page/inspector retention, 100k sources, '
          'empty/reset, themes and cleanup')


if __name__ == '__main__':
    main()
