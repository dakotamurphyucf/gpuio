#!/usr/bin/env python3
"""Stage spring interruption and active-response motion in the actual chat app."""
import ctypes as C
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_presentation import Presentation
from test_agent_chat_diagram import Diagram
from test_agent_chat_review import TITLE, CONVERSATION
from test_tree_outline import Point


class Motion(Presentation):
    def y(self, node):
        raw = self.attr(node, 'AXPosition')
        try:
            point = Point()
            if not raw or not self.value(raw, 1, C.byref(point)):
                raise RuntimeError('Missing native geometry')
            return point.y
        finally:
            if raw:
                self.release(raw)

    def samples(self, node, seconds):
        values = []
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            values.append(self.y(node))
            time.sleep(.015)
        return values

    def exercise(self, reduced):
        suffix = 'reduced' if reduced else 'full'
        self.release(self.wait_find(TITLE, 'Explore workspace', 'AXButton'))
        self.set(self.app, 'AXFrontmost', self.true)
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.wait_text(TITLE, 'A closer look.')
        self.capture('motion-overview-' + suffix + '.png')
        self.press(TITLE, 'Explore run diagram')
        self.wait_text(TITLE, 'Read sources · x 34 · y 42')
        Diagram.focus_stage(self, 'Read sources')
        self.key(36)
        self.wait_text(TITLE, 'SIMULATED · NO FILES ARE MODIFIED')
        next_stage = self.wait_find(TITLE, 'Next stage', 'AXButton')
        try:
            baseline = self.y(next_stage)
            self.absent('Local run context')
            self.press(TITLE, 'Show stage context')
            opening = self.samples(next_stage, 1.7)
            assert abs(opening[-1] - baseline - 112) < 1, (baseline, opening)
            if not reduced:
                assert any(1 < y - baseline < 111 for y in opening), opening
            self.wait_text(TITLE, 'Execution: simulated on this device')
            self.capture('motion-context-' + suffix + '.png')
            self.press(TITLE, 'Hide stage context')
            self.absent('Local run context')
            closing = self.samples(next_stage, 1.7)
            assert abs(closing[-1] - baseline) < 1, (baseline, closing)
            # Reverse an opening spring before it settles. Native retargeting must
            # finish at the accepted state, without accumulating layout offsets.
            self.press(TITLE, 'Show stage context')
            early = self.samples(next_stage, .08)
            self.press(TITLE, 'Hide stage context')
            reversed_values = self.samples(next_stage, 1.7)
            assert abs(reversed_values[-1] - baseline) < 1, reversed_values
            self.absent('Local run context')
            print(f'CHAT_SPRING_GEOMETRY_OK reduced={reduced} '
                  f'opening_delta={opening[-1] - baseline:.2f} '
                  f'intermediate={sum(1 < y - baseline < 111 for y in opening)} '
                  f'interrupted_delta={early[-1] - baseline:.2f}', flush=True)
        finally:
            self.release(next_stage)
        self.focus('Show stage context')
        self.key(36)
        self.wait_text(TITLE, 'Local run context')
        self.press(TITLE, 'Next stage')
        self.wait_text(TITLE, 'Up next: Review output')
        self.press(TITLE, 'Close workspace inspector')
        self.absent('Local run context')
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, 'Up next: Review output')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        self.capture('motion-context-light-' + suffix + '.png')
        self.press(TITLE, 'Close workspace inspector')
        self.press(TITLE, 'Demo controls')
        self.press(TITLE, 'Slow stream')
        self.draft(TITLE, CONVERSATION, 'Show native response activity')
        self.press(TITLE, 'Send')
        self.wait_text(TITLE, '· Responding…')
        self.indeterminate('Generating response')
        self.draft(TITLE, CONVERSATION, 'Typing while motion runs λ')
        self.capture('motion-response-light-' + suffix + '.png')
        self.press(TITLE, 'Cancel')
        self.wait_text(TITLE, '· Cancelled')
        self.absent('Generating response', 'AXProgressIndicator')
        assert self.draft(TITLE, CONVERSATION) == 'Typing while motion runs λ'
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def run(reduced):
    def timeout(_signal, _frame):
        raise TimeoutError('Chat motion walkthrough exceeded 120 seconds')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe'),
                                  '--reduced-motion' if reduced else '--full-motion'],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(120)
            mac = Motion(child.pid, child)
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
    print(f'GPUIO_CHAT_MOTION_APPKIT_OK reduced={reduced}: spring geometry/interruption, '
          'context visibility, navigation/remount, response activity/input/cancel, themes and cleanup', flush=True)


if __name__ == '__main__':
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    run(False)
    run(True)
