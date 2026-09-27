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

    def document_action(self, label, action):
        self.release(self.wait_find(TITLE, label, 'AXGroup'))
        node = self.within(label, action, 'AXButton')
        if not node:
            raise RuntimeError(f'Missing {action} in document {label}')
        try:
            self.perform(node, 'AXPress')
        finally:
            self.release(node)
        time.sleep(.15)

    def clipboard(self):
        return subprocess.run(['/usr/bin/pbpaste'], capture_output=True,
                              text=True, encoding='utf-8', check=True,
                              env={**os.environ, 'LC_ALL': 'en_US.UTF-8'}).stdout

    def rect(self, node):
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
            self.document_action(label, 'Expand')
            self.document_action(label, 'Copy source')
            actual = self.clipboard()
            assert actual == expected, (label, actual)
            self.capture('presentation-artifact-' + ('code-' if label == 'greeting.ml' else 'diff-') + suffix + '.png')
            self.document_action(label, 'Collapse')
        # Both expanded artifacts exceed the viewport. The code toolbar stays in
        # native overscan above the transcript. AX activation must invoke that
        # exact control, never synthesize a click into the window header.
        self.document_action('greeting.ml', 'Expand')
        self.document_action('Proposed patch', 'Expand')
        self.press(TITLE, 'Latest')
        time.sleep(.3)
        root = self.wait_find(TITLE, '', 'AXList')
        copy = self.within('greeting.ml', 'Copy source', 'AXButton')
        assert copy, 'Expected retained code toolbar'
        try:
            _, top, _, height = self.rect(root)
            _, y, _, h = self.rect(copy)
            assert not top <= y + h / 2 <= top + height, (top, height, y, h)
            subprocess.run(['/usr/bin/pbcopy'], input='AX source sentinel', text=True,
                           encoding='utf-8', check=True,
                           env={**os.environ, 'LC_ALL': 'en_US.UTF-8'})
            self.perform(copy, 'AXPress')
            time.sleep(.15)
            assert self.clipboard() == sources['greeting.ml']
        finally:
            self.release(copy)
            self.release(root)
        self.absent('Close workspace inspector', 'AXButton')
        self.document_action('greeting.ml', 'Collapse')
        self.document_action('Proposed patch', 'Collapse')
        print('PRESENTATION_DOCUMENT_CONTROLS_OK: labelled documents, exact source, '
              'offscreen AX action without unrelated clicks', flush=True)

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
        self.document_action('Complete', 'Copy code')
        assert self.clipboard() == 'let greeting = "Hello, λ 👨‍👩‍👧‍👦"\nlet answer = 42'
        self.document_action('Complete', 'Copy table')
        table = self.clipboard()
        assert table.startswith('| Check | Result |') and '| Unicode | Preserved |' in table, repr(table)
        assert '| Streaming | Complete |' in table, repr(table)
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
