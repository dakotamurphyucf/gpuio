#!/usr/bin/env python3
"""Native split resizing, container branches and retained chat drafts on macOS."""
import ctypes as C
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_presentation import Presentation
from test_agent_chat_results import Results
from test_agent_chat_review import TITLE, CONVERSATION
from test_tree_outline import Point


class Responsive(Presentation):
    def heading(self, compact):
        wanted = 'ARTIFACTS' if compact else 'ARTIFACT WORKSPACE'
        hidden = 'ARTIFACT WORKSPACE' if compact else 'ARTIFACTS'
        # The sidebar also has an ARTIFACTS caption. Scope branch assertions to
        # the inspector rather than accidentally inspecting the sidebar.
        end = time.monotonic() + 8
        while time.monotonic() < end:
            root = self.wait_find(TITLE, 'Artifact workspace', 'AXGroup')
            found = []
            def visit(node):
                if self.text(node, 'AXRole') == 'AXStaticText':
                    values = [self.text(node, attr) for attr in ('AXTitle', 'AXValue', 'AXDescription')]
                    found.extend(label for label in (wanted, hidden) if label in values)
                children = self.children(node)
                try:
                    for child in children:
                        visit(child)
                finally:
                    for child in children:
                        self.release(child)
            try:
                visit(root)
            finally:
                self.release(root)
            if found.count(wanted) == 1 and hidden not in found:
                return
            time.sleep(.03)
        raise RuntimeError(f'Inspector branch visibility: expected {wanted}, got {found}')

    def composer(self, title=TITLE):
        # AppKit accessibility handles can change when the surrounding native
        # layout changes. Native editor identity/marked text is separately
        # asserted by native_split; reacquire AX handles for public observations.
        node = self.wait_find(title, 'Message · ' + CONVERSATION, 'AXTextArea')
        try:
            return self.rect(node), self.text(node, 'AXValue')
        finally:
            self.release(node)

    def settled_width(self, expected, title=TITLE):
        end = time.monotonic() + 8
        while time.monotonic() < end:
            bounds, text = self.composer(title)
            if abs(bounds[2] - expected) < 1:
                return text
            time.sleep(.03)
        raise RuntimeError(f'Composer did not settle at {expected}: {bounds}')

    def resize_window(self, title, width, height):
        window = self.window(title)
        create = self.ax.AXValueCreate
        create.restype, create.argtypes = C.c_void_p, [C.c_int, C.c_void_p]
        try:
            for name, kind, point in [('AXPosition', 1, Point(30, 50)),
                                      ('AXSize', 2, Point(width, height))]:
                value = create(kind, C.byref(point))
                try:
                    self.set(window, name, value)
                finally:
                    self.release(value)
            end = time.monotonic() + 8
            while time.monotonic() < end:
                bounds = self.rect(window)
                if abs(bounds[2] - width) < 1 and abs(bounds[3] - height) < 1:
                    break
                time.sleep(.03)
            else:
                raise RuntimeError(f'Window did not resize: {bounds}')
        finally:
            self.release(window)
        time.sleep(.15)

    def one_toolbar(self):
        expected = {'Close tab', 'Settings', 'Explore workspace', 'New window', 'Commands'}
        counts = dict.fromkeys(expected, 0)
        def visit(node):
            if self.text(node, 'AXRole') == 'AXButton':
                label = self.text(node, 'AXTitle') or self.text(node, 'AXDescription')
                if label in counts:
                    counts[label] += 1
            children = self.children(node)
            try:
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    self.release(child)
        window = self.window(TITLE)
        try:
            visit(window)
        finally:
            self.release(window)
        assert all(count == 1 for count in counts.values()), counts

    def exercise(self, reduced):
        suffix = 'reduce' if reduced else 'full'
        draft = 'Retain this draft λ 👨‍👩‍👧‍👦'
        self.release(self.wait_find(TITLE, 'Explore workspace', 'AXButton'))
        self.set(self.app, 'AXFrontmost', self.true)
        self.resize_window(TITLE, 1180, 820)
        self.wait_text(TITLE, 'STUDIO')
        self.draft(TITLE, CONVERSATION, draft)
        closed_width = self.composer()[0][2]
        self.one_toolbar()
        self.press(TITLE, 'Explore workspace')
        self.heading(compact=True)
        divider = self.wait_find(TITLE, 'Artifact inspector width', 'AXSplitter')
        try:
            x, y, w, h = self.rect(divider)
            Results.drag(self, Point(x + w / 2, y + h / 2),
                         Point(x + w / 2 - 128, y + h / 2))
        finally:
            self.release(divider)
        self.heading(compact=False)
        self.one_toolbar()
        resized_width, observed_draft = self.composer()[0][2], self.composer()[1]
        assert observed_draft == draft
        assert closed_width - resized_width > 250
        self.capture('responsive-inspector-dark-' + suffix + '.png')
        # Exercise keyboard resizing on the native divider as well as pointer.
        divider = self.wait_find(TITLE, 'Artifact inspector width', 'AXSplitter')
        try:
            self.set(divider, 'AXFocused', self.true)
            self.key(123)  # Left.
            self.settled_width(resized_width - 16)
            self.key(124)  # Right.
            self.settled_width(resized_width)
        finally:
            self.release(divider)
        for _ in range(2):
            self.press(TITLE, 'Close workspace inspector')
            self.absent('Close workspace inspector', 'AXButton')
            self.absent('Artifact inspector width', 'AXSplitter')
            assert self.settled_width(closed_width) == draft
            self.press(TITLE, 'Explore workspace')
            self.release(self.wait_find(TITLE, 'Artifact inspector width', 'AXSplitter'))
            assert self.settled_width(resized_width) == draft
        self.resize_window(TITLE, 1000, 820)
        self.absent('STUDIO', 'AXStaticText')
        self.one_toolbar()
        assert self.composer()[1] == draft
        self.press(TITLE, 'Light theme')
        self.release(self.wait_find(TITLE, 'Dark theme', 'AXButton'))
        self.capture('responsive-narrow-light-' + suffix + '.png')
        first_width = self.composer()[0][2]
        self.press(TITLE, 'New window')
        second = 'GPUIO · Agent workspace 2'
        self.release(self.wait_find(second, 'Explore workspace', 'AXButton'))
        self.resize_window(second, 1360, 820)
        assert self.composer(second)[1] == ''
        self.press(second, 'Explore workspace')
        self.release(self.wait_find(second, 'Artifact inspector width', 'AXSplitter'))
        assert self.settled_width(first_width) == draft
        self.close(second)
        end = time.monotonic() + 8
        while self.has_window(second) and time.monotonic() < end:
            time.sleep(.03)
        assert not self.has_window(second)
        window = self.window(TITLE)
        try:
            self.perform(window, 'AXRaise')
        finally:
            self.release(window)
        self.resize_window(TITLE, 1360, 820)
        self.wait_text(TITLE, 'STUDIO')
        self.one_toolbar()
        self.capture('responsive-wide-light-' + suffix + '.png')
        assert self.composer()[1] == draft
        self.draft(TITLE, CONVERSATION, '')
        self.close(TITLE)
        print(f'CHAT_RESPONSIVE_OK reduced={reduced} closed_width={closed_width:.1f} '
              f'resized_width={resized_width:.1f} retained_draft=True independent_windows=True', flush=True)


def run(reduced):
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe'),
                                  '--reduced-motion' if reduced else '--full-motion'],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(120)
            mac = Responsive(child.pid, child)
            mac.exercise(reduced)
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


if __name__ == '__main__':
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Responsive chat test exceeded 120 seconds')
    signal.signal(signal.SIGALRM, timeout)
    run(False)
    run(True)
