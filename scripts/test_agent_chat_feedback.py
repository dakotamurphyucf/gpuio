#!/usr/bin/env python3
"""Local feedback, retained notes and contextual navigation in the real chat app."""
import ctypes as C
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_dates_colors import DatesColors
from test_agent_chat_review import TITLE, CONVERSATION
from test_tree_outline import Point


class Feedback(DatesColors):
    def absent(self, label, role=None):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            node = self.find(TITLE, label, role)
            if not node:
                return
            self.release(node)
            time.sleep(.03)
        raise RuntimeError(f'Hidden content remains accessible: {label}')

    def focus(self, label, role='AXButton'):
        node = self.wait_find(TITLE, label, role)
        try:
            self.set(node, 'AXFocused', self.true)
            # AX setters enqueue native actions. Consecutive focus changes must
            # observe the first transition before requesting the second, or
            # GPUI can coalesce them and never leave an Escape-suppressed anchor.
            deadline = time.monotonic() + 5
            while not self.boolean(node, 'AXFocused'):
                if time.monotonic() >= deadline:
                    raise RuntimeError('Native focus was not accepted: ' + label)
                time.sleep(.01)
        finally:
            self.release(node)

    def hover(self, label):
        node = self.wait_find(TITLE, label, 'AXButton')
        try:
            position, size = Point(), Point()
            for name, kind, target in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, name)
                try:
                    if not raw or not self.value(raw, kind, C.byref(target)):
                        raise RuntimeError('Missing contributor trigger geometry')
                finally:
                    if raw:
                        self.release(raw)
            self.send(5, Point(position.x + size.x / 2, position.y + size.y / 2))
        finally:
            self.release(node)

    def exercise(self):
        self.draft(TITLE, CONVERSATION, 'Feedback preserves the conversation λ')
        self.set(self.app, 'AXFrontmost', self.true)
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Review feedback')
        self.wait_text(TITLE, 'Not rated yet')
        self.slider('Run usefulness', key=124)
        self.key(124)
        self.key(124)
        self.wait_text(TITLE, 'Run usefulness: 3 of 5')
        self.key(119)  # End saturates at five.
        self.wait_text(TITLE, 'Run usefulness: 5 of 5')
        self.press(TITLE, 'Clear rating')
        self.wait_text(TITLE, 'Not rated yet')
        self.slider('Run usefulness', value=4)
        self.wait_text(TITLE, 'Run usefulness: 4 of 5')
        self.press(TITLE, 'Private review notes')
        self.field(TITLE, 'Private review note', 'AXTextArea', 'Verify Unicode λ and cancellation.')
        self.press(TITLE, 'Private review notes')
        self.absent('Private review note', 'AXTextArea')
        self.button_state('Private review notes', 'AXFocused', True)
        self.press(TITLE, 'Private review notes')
        assert self.field(TITLE, 'Private review note', 'AXTextArea') == 'Verify Unicode λ and cancellation.'
        self.capture('feedback-note-dark.png')
        self.press(TITLE, 'Private review notes')
        self.scroll_settings(-340)
        self.press(TITLE, 'Check the sources')
        self.button_state('Check the sources', 'AXExpanded', True)
        self.focus('Check the sources')
        self.button_state('Check the sources', 'AXFocused', True)
        self.key(125)  # Native accordion header navigation, then activate.
        self.button_state('Inspect the changes', 'AXFocused', True)
        self.key(36)
        self.button_state('Inspect the changes', 'AXExpanded', True)
        self.button_state('Check the sources', 'AXExpanded', False)
        self.absent('Check that the supplied context supports the response and its claims.')
        self.press(TITLE, 'Keep sections open')
        self.press(TITLE, 'Check the sources')
        self.button_state('Check the sources', 'AXExpanded', True)
        self.button_state('Inspect the changes', 'AXExpanded', True)
        self.press(TITLE, 'One section')
        self.button_state('Check the sources', 'AXExpanded', True)
        self.button_state('Inspect the changes', 'AXExpanded', False)
        self.scroll_settings(-220)
        self.capture('feedback-guidance-dark.png')
        self.scroll_settings(1000)
        self.focus('About this contributor')
        self.release(self.wait_find(TITLE, 'GPUIO local assistant', 'AXImage'))
        self.release(self.wait_find(TITLE, 'Open contributor sources', 'AXLink'))
        self.capture('feedback-contributor-dark.png')
        self.key(53)
        self.absent('Open contributor sources', 'AXLink')
        self.button_state('About this contributor', 'AXFocused', True)
        self.focus('Run usefulness', 'AXSlider')
        self.hover('Back')  # Escape also requires a fresh pointer leave/entry.
        self.hover('About this contributor')
        self.release(self.wait_find(TITLE, 'Open contributor sources', 'AXLink'))
        rating = self.wait_find(TITLE, 'Run usefulness', 'AXSlider')
        try:
            assert self.boolean(rating, 'AXFocused'), 'Hover stole keyboard focus'
        finally:
            self.release(rating)
        link = self.wait_find(TITLE, 'Open contributor sources', 'AXLink')
        try:
            self.perform(link, 'AXPress')
        finally:
            self.release(link)
        self.wait_text(TITLE, 'Context, close at hand.')
        self.absent('Open contributor sources', 'AXLink')
        self.hover('Back')  # Avoid a fresh hover opening the remounted preview.
        self.focus('Back')  # AXPress alone does not transfer keyboard focus.
        self.button_state('Back', 'AXFocused', True)
        self.press(TITLE, 'Back')
        self.wait_text(TITLE, 'Run usefulness: 4 of 5')
        self.hover('Back')
        self.focus('Back')
        self.button_state('Back', 'AXFocused', True)
        self.absent('Open contributor sources', 'AXLink')
        self.press(TITLE, 'Private review notes')
        assert self.field(TITLE, 'Private review note', 'AXTextArea') == 'Verify Unicode λ and cancellation.'
        self.press(TITLE, 'Close workspace inspector')
        self.absent('Run usefulness', 'AXSlider')
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, 'Run usefulness: 4 of 5')
        assert self.field(TITLE, 'Private review note', 'AXTextArea') == 'Verify Unicode λ and cancellation.'
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        self.hover('Back')
        self.focus('Private review note', 'AXTextArea')
        self.absent('Open contributor sources', 'AXLink')
        self.capture('feedback-note-light.png')
        self.press(TITLE, 'Private review notes')
        self.focus('About this contributor')
        self.release(self.wait_find(TITLE, 'Open contributor sources', 'AXLink'))
        self.capture('feedback-contributor-light.png')
        self.key(53)
        assert self.draft(TITLE, CONVERSATION) == 'Feedback preserves the conversation λ'
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Feedback walkthrough exceeded 180 seconds')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(180)
            mac = Feedback(child.pid, child)
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
    print('GPUIO_CHAT_FEEDBACK_APPKIT_OK: rating keyboard/AX/clear, retained native notes/'
          'focus/remount, single/multiple accordion keyboard/semantics, contributor '
          'keyboard/pointer preview/Escape/source navigation, themes, composer and cleanup')


if __name__ == '__main__':
    main()
