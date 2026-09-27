#!/usr/bin/env python3
"""Artifact carousel and real avatar decoding/fallback in the macOS chat demo."""
from pathlib import Path
import ctypes as C
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_feedback import Feedback
from test_agent_chat_review import TITLE, CONVERSATION
from test_tree_outline import Point


class Tour(Feedback):
    def swipe(self):
        node = self.wait_find(TITLE, 'Workspace tour', 'AXGroup')
        try:
            position, size = Point(), Point()
            for name, kind, target in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, name)
                try:
                    if not raw or not self.value(raw, kind, C.byref(target)):
                        raise RuntimeError('Missing tour geometry')
                finally:
                    if raw:
                        self.release(raw)
            start = Point(position.x + size.x * .8, position.y + size.y * .2)
            end = Point(position.x + size.x * .2, start.y)
        finally:
            self.release(node)
        self.send(5, start)
        self.send(1, start)
        try:
            for step in range(1, 9):
                self.send(6, Point(start.x + (end.x - start.x) * step / 8, start.y))
                time.sleep(.025)
        finally:
            self.send(2, end)

    def stop(self, name):
        self.wait_text(TITLE, 'Tour stop: ' + name)

    def steady(self, name, seconds=4.7):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            node = self.find(TITLE, 'Tour stop: ' + name, 'AXStaticText')
            if not node:
                raise RuntimeError(f'Tour advanced while paused: expected {name}')
            self.release(node)
            time.sleep(.08)

    def gallery(self, label):
        node = self.within('Workspace tour', label, 'AXButton')
        if not node:
            raise RuntimeError('Missing tour control: ' + label)
        try:
            self.perform(node, 'AXPress')
        finally:
            self.release(node)

    def exercise(self, reduced):
        self.draft(TITLE, CONVERSATION, 'Tour preserves my draft λ')
        self.set(self.app, 'AXFrontmost', self.true)
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.press(TITLE, 'Take workspace tour')
        self.stop('Sources')
        self.focus('Workspace tour', 'AXGroup')
        self.key(124)
        self.stop('Results')
        current = self.within('Workspace tour', '2', 'AXButton')
        try:
            assert current and self.text(current, 'AXHelp') == 'Current item'
        finally:
            if current:
                self.release(current)
        self.key(119)
        self.stop('Feedback')
        self.key(115)
        self.stop('Sources')
        time.sleep(.25)  # Settle the native Home transition before a pointer gesture.
        self.swipe()
        self.stop('Results')
        self.gallery('Previous')
        self.stop('Sources')
        self.gallery('Next')
        self.stop('Results')
        self.press(TITLE, 'Open Results')
        self.wait_text(TITLE, 'Findings, in focus.')
        self.focus('Back')
        self.press(TITLE, 'Back')
        self.stop('Results')
        self.press(TITLE, 'Start guided tour')
        self.focus('Workspace tour', 'AXGroup')
        self.steady('Results')  # Native focus pauses the opt-in timer.
        self.focus('Back')
        self.hover('Open Results')
        self.steady('Results')  # Pointer hover pauses without relying on focus.
        self.hover('Back')
        if reduced:
            self.steady('Results')
        else:
            self.stop('Diagram')  # Outside focus/hover permits one native proposal.
        selected = 'Results' if reduced else 'Diagram'
        self.press(TITLE, 'Close workspace inspector')
        self.absent('Workspace tour', 'AXGroup')
        time.sleep(4.7)
        self.press(TITLE, 'Explore workspace')
        self.focus('Back')
        self.stop(selected)  # Hidden time is not caught up on remount.
        self.press(TITLE, 'Pause guided tour')
        self.stop(selected)
        self.gallery('First')
        self.stop('Sources')
        self.capture('tour-reduced.png' if reduced else 'tour-dark.png')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        self.capture('tour-reduced-light.png' if reduced else 'tour-light.png')
        self.press(TITLE, 'Open Sources')
        self.wait_text(TITLE, 'Context, close at hand.')
        self.focus('Back')
        self.press(TITLE, 'Back')
        self.stop('Sources')
        self.gallery('3')
        self.stop('Diagram')
        self.press(TITLE, 'Open Diagram')
        self.wait_text(TITLE, 'Select a stage to inspect the run')
        self.focus('Back')
        self.press(TITLE, 'Back')
        self.stop('Diagram')
        self.gallery('Last')
        self.stop('Feedback')
        self.press(TITLE, 'Open Feedback')
        self.wait_text(TITLE, 'Not rated yet')
        self.focus('About this contributor')
        self.wait_text(TITLE, 'Contributor initials')
        self.press(TITLE, 'Local portrait')
        self.wait_text(TITLE, 'Local portrait ready')
        if not reduced:
            self.capture('contributor-portrait-light.png')
        self.press(TITLE, 'Unavailable portrait')
        self.wait_text(TITLE, 'Portrait unavailable · showing initials')
        self.release(self.wait_find(TITLE, 'GPUIO local assistant', 'AXImage'))
        if not reduced:
            self.capture('contributor-fallback-light.png')
        self.press(TITLE, 'Local portrait')
        self.wait_text(TITLE, 'Local portrait ready')
        self.key(53)
        self.press(TITLE, 'Close workspace inspector')
        self.press(TITLE, 'Explore workspace')
        self.focus('About this contributor')
        self.wait_text(TITLE, 'Local portrait ready')
        self.press(TITLE, 'Unavailable portrait')
        self.wait_text(TITLE, 'Portrait unavailable · showing initials')
        self.key(53)
        if not reduced:
            self.press(TITLE, 'Dark theme')
            self.wait_text(TITLE, 'Light theme')
            self.focus('Back')  # Escape requires a fresh focus entry to reopen.
            self.focus('About this contributor')
            self.wait_text(TITLE, 'Portrait unavailable · showing initials')
            self.capture('contributor-fallback-dark.png')
            self.press(TITLE, 'Local portrait')
            self.wait_text(TITLE, 'Local portrait ready')
            self.capture('contributor-portrait-dark.png')
            self.key(53)
        assert self.draft(TITLE, CONVERSATION) == 'Tour preserves my draft λ'
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def run(reduced):
    def timeout(_signal, _frame):
        raise TimeoutError('Tour walkthrough exceeded 180 seconds')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    args = [str(repo / '_build/default/examples/agent_chat/main.exe')]
    args.append('--reduced-motion' if reduced else '--full-motion')
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen(args, cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(180)
            mac = Tour(child.pid, child)
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
    print(f'GPUIO_CHAT_TOUR_APPKIT_OK reduced={reduced}: native carousel keyboard/controls/'
          'selection, real destinations, opt-in timer/focus/hover/hidden policy, themes, '
          'real portrait decode/fallback/remount, composer and window cleanup', flush=True)


if __name__ == '__main__':
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    run(False)
    run(True)
