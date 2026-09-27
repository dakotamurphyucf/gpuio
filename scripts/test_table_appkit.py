#!/usr/bin/env python3
"""Public table context flows through macOS keyboard, pointer and AX actions.

Only the spawned application's PID receives keys. Pointer input checks the owning
window before posting; every exit path releases held buttons and reaps the child.
Clipboard/marked-text acceptance lives in the retained-host native suite.
"""
import ctypes as C
from pathlib import Path
import subprocess
import signal
import sys
import tempfile
import time

from test_tree_outline import Outline, Point

TITLE = 'GPUIO — Table Lab'


class Table(Outline):
    def cell_center(self, value):
        cell = self.wait_find(TITLE, value, 'AXCell', search_files=True)
        try:
            position, size = Point(), Point()
            for name, kind, result in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(cell, name)
                try:
                    if not raw or not self.value(raw, kind, C.byref(result)):
                        raise RuntimeError(f'Missing cell geometry: {value}')
                finally:
                    if raw:
                        self.release(raw)
            if size.x <= 0 or size.y <= 0:
                raise RuntimeError('Empty cell geometry')
            return Point(position.x + size.x / 2, position.y + size.y / 2)
        finally:
            self.release(cell)

    def click(self, point, right=False):
        self.send(5, point)
        self.send(3 if right else 1, point, 1 if right else 0)
        try:
            time.sleep(.05)
        finally:
            self.send(4 if right else 2, point, 1 if right else 0)

    def wait_closed_dialog(self):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            node = self.find(TITLE, 'Event actions')
            if not node:
                return
            self.release(node)
            time.sleep(.05)
        raise RuntimeError('Event dialog did not close')

    def exercise(self):
        self.cell_center('000000')
        self.set(self.app, 'AXFrontmost', self.true)
        window = self.window(TITLE)
        try:
            self.perform(window, 'AXRaise')
        finally:
            self.release(window)
        time.sleep(.2)
        self.click(self.cell_center('000000'))
        self.wait_text(TITLE, 'Selected event 0 · number')
        self.key(125)  # Down through AppKit, no application callback invocation.
        self.wait_text(TITLE, 'Selected event 1 · number')
        self.key(36)
        self.wait_text(TITLE, 'Event 000001 · Read file')
        self.key(53)
        self.wait_closed_dialog()
        self.key(109, 1 << 17)  # Shift-F10 after native focus restoration.
        self.wait_text(TITLE, 'Event 000001 · Read file')
        self.press(TITLE, 'Reveal result')
        self.wait_closed_dialog()
        self.wait_text(TITLE, 'Selected event 1 · message')
        self.key(123)
        self.wait_text(TITLE, 'Selected event 1 · tool')
        # Context target is independent of the existing selected row.
        self.click(self.cell_center('000002'), right=True)
        self.wait_text(TITLE, 'Event 000002 · Search')
        self.press(TITLE, 'Close event details')
        self.wait_closed_dialog()
        self.wait_text(TITLE, 'Selected event 1 · tool')
        window = self.window(TITLE)
        try:
            close = self.attr(window, 'AXCloseButton')
            if not close:
                raise RuntimeError('Missing native window close control')
            try:
                self.perform(close, 'AXPress')
            finally:
                self.release(close)
        finally:
            self.release(window)


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This test exercises the AppKit backend')
    repo = Path(__file__).resolve().parent.parent
    def timeout(_signal, _frame):
        raise TimeoutError('Public AppKit table test exceeded its 120-second budget')
    signal.signal(signal.SIGALRM, timeout)
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/table/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(120)
            mac = Table(child.pid, child)
            mac.exercise()
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Table example failed')
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
    print('GPUIO_TABLE_PUBLIC_APPKIT_OK: pointer selection, OS arrows/Return/Shift-F10, '
          'context target, dialog dismissal/restoration, reveal and native window close')


if __name__ == '__main__':
    main()
