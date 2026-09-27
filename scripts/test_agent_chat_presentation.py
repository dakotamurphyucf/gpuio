#!/usr/bin/env python3
"""Real query/loading/error/filter and transcript presentation in the chat app."""
from pathlib import Path
import ctypes as C
import os
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_feedback import Feedback
from test_tree_outline import Point
from test_agent_chat_review import TITLE, CONVERSATION


class Presentation(Feedback):
    def indeterminate(self, label):
        node = self.wait_find(TITLE, label, 'AXProgressIndicator')
        value = self.attr(node, 'AXValue')
        try:
            assert not value, 'Loading must not invent numeric progress: ' + label
        finally:
            if value:
                self.release(value)
            self.release(node)

    def query_loading(self):
        expected = {'Preparing result columns', 'Preparing result rows', 'Loading result query'}
        root = self.wait_find(TITLE, 'Artifact workspace', 'AXGroup')
        started = time.monotonic()
        try:
            deadline = started + 8
            while time.monotonic() < deadline:
                found = set()
                def visit(node):
                    label = self.text(node, 'AXTitle')
                    if label in expected and self.text(node, 'AXRole') == 'AXProgressIndicator':
                        value = self.attr(node, 'AXValue')
                        try:
                            assert not value, 'Invented loading progress: ' + label
                        finally:
                            if value:
                                self.release(value)
                        found.add(label)
                    children = self.children(node)
                    try:
                        for child in children:
                            if found == expected:
                                break
                            visit(child)
                    finally:
                        for child in children:
                            self.release(child)
                visit(root)
                if found == expected:
                    print('QUERY_LOADING_SEMANTICS_OK elapsed=' + str(time.monotonic() - started), flush=True)
                    return
                time.sleep(.02)
            raise RuntimeError('Query loading indicators missing: ' + repr(expected - found))
        finally:
            self.release(root)

    def document_action(self, expected, action):
        # Native document labels are painted text, not separate AX nodes. Identify
        # each real document by its exact copied source inside the transcript.
        root = self.wait_find(TITLE, '', 'AXList')
        buttons = []
        def bounds(node):
            position, size = Point(), Point()
            for name, kind, target in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, name)
                try:
                    if not raw or not self.value(raw, kind, C.byref(target)):
                        raise RuntimeError('Missing document geometry')
                finally:
                    if raw:
                        self.release(raw)
            return position.x, position.y, size.x, size.y
        left, top, width, height = bounds(root)
        def visit(node):
            if (self.text(node, 'AXRole') == 'AXButton'
                    and self.text(node, 'AXTitle') == 'Copy source'):
                buttons.append(self.retain(node))
            children = self.children(node)
            try:
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    self.release(child)
        try:
            visit(root)
            for copy in buttons:
                x, y, w, h = bounds(copy)
                # Managed overscan rows can retain offscreen descendants. Their
                # AXPress fallback is a click; never click outside the viewport.
                if not (left <= x + w / 2 <= left + width
                        and top <= y + h / 2 <= top + height):
                    continue
                self.perform(copy, 'AXPress')
                time.sleep(.1)
                actual = subprocess.run(['/usr/bin/pbpaste'], capture_output=True,
                                        text=True, encoding='utf-8', check=True,
                                        env={**os.environ, 'LC_ALL': 'en_US.UTF-8'}).stdout
                if actual != expected:
                    continue
                if action == 'Copy source':
                    return
                parent = self.attr(copy, 'AXParent')
                if not parent:
                    raise RuntimeError('Document button has no parent')
                children = self.children(parent)
                try:
                    for child in children:
                        if (self.text(child, 'AXRole') == 'AXButton'
                                and self.text(child, 'AXTitle') == action):
                            self.perform(child, 'AXPress')
                            time.sleep(.15)
                            return
                finally:
                    for child in children:
                        self.release(child)
                    self.release(parent)
                raise RuntimeError('Document control missing: ' + action)
            self.dump(TITLE)
            raise RuntimeError('Expected document source not available')
        finally:
            for button in buttons:
                self.release(button)
            self.release(root)

    def artifacts(self, suffix):
        self.release(self.wait_find(TITLE, 'Expand', 'AXButton'))
        sources = {
            'greeting.ml': ('open Core\n\nlet greeting name =\n'
                            '  sprintf "Hello, %s — λ 👨‍👩‍👧‍👦" name\n;;\n\nlet answer = 42\n'),
            'Proposed patch': ('--- a/greeting.ml\n+++ b/greeting.ml\n'
                               '@@ -1,2 +1,3 @@\n-let answer = 0\n+let answer = 42\n'
                               '+let status = "ready"\n let language = "OCaml"\n'),
        }
        for label, expected in sources.items():
            self.document_action(expected, 'Expand')
            self.document_action(expected, 'Copy source')
            actual = subprocess.run(['/usr/bin/pbpaste'], capture_output=True,
                                    text=True, encoding='utf-8', check=True,
                                        env={**os.environ, 'LC_ALL': 'en_US.UTF-8'}).stdout
            assert actual == expected, (label, actual)
            self.capture('presentation-artifact-' + ('code-' if label == 'greeting.ml' else 'diff-') + suffix + '.png')
            self.document_action(expected, 'Collapse')
        print('PRESENTATION_DOCUMENT_CONTROLS_OK', flush=True)

    def exercise(self, reduced):
        suffix = 'reduced' if reduced else 'full'
        self.release(self.wait_find(TITLE, 'Explore workspace', 'AXButton'))
        self.set(self.app, 'AXFrontmost', self.true)
        self.artifacts(suffix)
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.press(TITLE, 'Explore results')
        self.wait_text(TITLE, '24 results loaded')
        self.press(TITLE, 'Score ≥ 80')
        self.wait_text(TITLE, '10 results loaded')
        self.focus('Remove score filter')
        self.key(36)
        self.wait_text(TITLE, '48 results loaded')
        self.absent('Remove score filter', 'AXButton')
        self.press(TITLE, 'Empty results')
        self.wait_text(TITLE, 'No findings match')
        self.capture('presentation-empty-' + suffix + '.png')
        self.press(TITLE, 'Remove score filter')
        self.wait_text(TITLE, '48 results loaded')
        self.press(TITLE, 'Slow query')
        self.query_loading()
        self.scroll_settings(1000)
        self.capture('presentation-loading-' + suffix + '.png')
        self.press(TITLE, 'Close workspace inspector')
        self.absent('Preparing result rows', 'AXProgressIndicator')
        self.absent('Loading result query', 'AXProgressIndicator')
        time.sleep(5.2)  # Page data is window-owned and can finish while hidden.
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, '24 results loaded')
        self.absent('Preparing result columns', 'AXProgressIndicator')
        self.press(TITLE, 'Slow query')
        self.wait_text(TITLE, 'Loading results…')
        self.press(TITLE, 'Empty results')
        self.absent('Loading result query', 'AXProgressIndicator')
        time.sleep(5.2)
        self.wait_text(TITLE, '0 results loaded')
        self.press(TITLE, 'Fail next query')
        self.wait_text(TITLE, 'Results unavailable')
        self.wait_text(TITLE, 'Sample results unavailable. Retry to continue.')
        self.press(TITLE, 'Retry results')
        self.wait_text(TITLE, '24 results loaded')
        self.absent('Results unavailable')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        self.press(TITLE, 'Score ≥ 80')
        self.wait_text(TITLE, '10 results loaded')
        self.scroll_settings(1000)
        self.capture('presentation-filter-light-' + suffix + '.png')
        self.press(TITLE, 'Close workspace inspector')
        self.press(TITLE, 'Demo controls')
        self.press(TITLE, 'Slow stream')
        self.draft(TITLE, CONVERSATION, 'Show me a useful response λ')
        self.press(TITLE, 'Send')
        self.wait_text(TITLE, '· Responding…')
        self.indeterminate('Generating response')
        self.draft(TITLE, CONVERSATION, 'Keep this presentation draft')
        self.capture('presentation-stream-light-' + suffix + '.png')
        self.press(TITLE, 'Cancel')
        self.wait_text(TITLE, '· Cancelled')
        self.absent('Generating response', 'AXProgressIndicator')
        self.press(TITLE, 'Simulate error')
        self.draft(TITLE, CONVERSATION, 'Demonstrate a recoverable failure')
        self.press(TITLE, 'Send')
        self.wait_text(TITLE, 'Response interrupted')
        self.wait_text(TITLE, 'Simulated connection interrupted')
        self.capture('presentation-error-light-' + suffix + '.png')
        self.press(TITLE, 'Retry response')
        self.wait_text(TITLE, '· Complete')
        self.absent('Response interrupted')
        self.absent('Generating response', 'AXProgressIndicator')
        self.draft(TITLE, CONVERSATION, 'Keep this presentation draft')
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def run(reduced):
    def timeout(_signal, _frame):
        raise TimeoutError('Presentation walkthrough exceeded 150 seconds')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe'),
                                  '--reduced-motion' if reduced else '--full-motion'],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(150)
            mac = Presentation(child.pid, child)
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
    print(f'GPUIO_CHAT_PRESENTATION_APPKIT_OK reduced={reduced}: native loading semantics, '
          'completion/hiding/cancellation, removable full-query filters, failure/retry, '
          'streaming/cancel/error recovery, themes and cleanup', flush=True)


if __name__ == '__main__':
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    run(False)
    run(True)
