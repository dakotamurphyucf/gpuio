#!/usr/bin/env python3
"""Actual route sharing and independent sidebar expansion/collapse in the chat."""
import ctypes as C
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

from test_agent_chat_feedback import Feedback
from test_agent_chat_review import TITLE, CONVERSATION
from test_tree_outline import Point


class Sidebar(Feedback):
    def pointer_link(self, label):
        node = self.wait_find(TITLE, label, 'AXLink')
        try:
            position, size = Point(), Point()
            for name, kind, target in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, name)
                try:
                    if not raw or not self.value(raw, kind, C.byref(target)):
                        raise RuntimeError('Missing sidebar link geometry')
                finally:
                    if raw:
                        self.release(raw)
            point = Point(position.x + size.x / 2, position.y + size.y / 2)
        finally:
            self.release(node)
        self.send(5, point)
        self.send(1, point)
        try:
            pass
        finally:
            self.send(2, point)

    def link(self, label, *, keyboard=False):
        node = self.wait_find(TITLE, label, 'AXLink')
        try:
            if keyboard:
                self.set(node, 'AXFocused', self.true)
                self.key(36)
            else:
                self.perform(node, 'AXPress')
        finally:
            self.release(node)

    def current(self, label):
        node = self.wait_find(TITLE, label, 'AXLink')
        try:
            assert self.text(node, 'AXHelp') == 'Current workspace page', label
        finally:
            self.release(node)

    def exercise(self):
        self.draft(TITLE, CONVERSATION, 'Sidebar keeps my draft λ')
        self.set(self.app, 'AXFrontmost', self.true)
        self.pointer_link('Workspace overview')
        self.wait_text(TITLE, 'A closer look.')
        self.absent('Checkpoints', 'AXLink')
        self.press(TITLE, 'Expand Run diagram')
        self.press(TITLE, 'Expand Source collection')
        self.link('Checkpoints', keyboard=True)
        self.wait_text(TITLE, '0 of 100 checkpoints')
        self.current('Checkpoints')
        self.press(TITLE, 'Collapse Run diagram')
        self.absent('Checkpoints', 'AXLink')
        self.release(self.wait_find(TITLE, 'Run findings', 'AXLink'))
        self.wait_text(TITLE, '0 of 100 checkpoints')  # Expansion never navigates.
        self.link('Run findings')
        self.wait_text(TITLE, 'Findings, in focus.')
        self.current('Run findings')
        self.press(TITLE, 'Back')
        self.wait_text(TITLE, '0 of 100 checkpoints')
        self.press(TITLE, 'Expand Run diagram')
        self.current('Checkpoints')  # History, not an independent selected-page copy.
        self.capture('sidebar-expanded-dark.png')
        self.press(TITLE, 'Collapse destinations')
        self.absent('Checkpoints', 'AXLink')
        self.absent('Run findings', 'AXLink')
        self.link('Source collection', keyboard=True)
        self.wait_text(TITLE, 'Context, close at hand.')
        self.current('Source collection')
        self.capture('sidebar-icons-dark.png')
        self.press(TITLE, 'Hide mode')
        self.absent('Source collection', 'AXLink')
        self.wait_text(TITLE, 'Context, close at hand.')
        self.capture('sidebar-hidden-dark.png')
        self.press(TITLE, 'Show destinations')
        self.release(self.wait_find(TITLE, 'Checkpoints', 'AXLink'))
        self.release(self.wait_find(TITLE, 'Run findings', 'AXLink'))
        self.current('Source collection')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        self.capture('sidebar-expanded-light.png')
        self.link('Guided tour')
        self.wait_text(TITLE, 'Tour stop: Sources')
        self.current('Guided tour')
        self.press(TITLE, 'Close workspace inspector')
        self.link('Run feedback')
        self.wait_text(TITLE, 'Not rated yet')
        self.current('Run feedback')
        assert self.draft(TITLE, CONVERSATION) == 'Sidebar keeps my draft λ'
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def run():
    def timeout(_signal, _frame):
        raise TimeoutError('Sidebar walkthrough exceeded 150 seconds')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(150)
            mac = Sidebar(child.pid, child)
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
    print('GPUIO_CHAT_SIDEBAR_APPKIT_OK: shared routes/history, independent expansion, '
          'keyboard links, icon/offcanvas inertness/restoration, themes, draft and cleanup',
          flush=True)


if __name__ == '__main__':
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    run()
