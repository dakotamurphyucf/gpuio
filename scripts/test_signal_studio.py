#!/usr/bin/env python3
"""Public Signal Studio AppKit input, responsive layout and composed component."""
import argparse
import ctypes as C
import re
from pathlib import Path
import signal
import subprocess
import time
from test_tree_outline import Outline, Point
from test_canvas import screenshot

TITLE = 'GPUIO · Signal Studio'


class Studio(Outline):
    def __init__(self, child, log_path, *, pid=None):
        super().__init__(child.pid if pid is None else pid, child)
        self.log_path = log_path

    def activate(self):
        self.set(self.app, 'AXFrontmost', self.true)
        window = self.window(TITLE)
        try:
            assert window
            self.perform(window, 'AXRaise')
        finally:
            if window:
                self.release(window)
        time.sleep(.25)

    def capture(self, path):
        # A desktop/Space change can remove our otherwise live window from the
        # on-screen capture list. Raise only our child and let its frame resume.
        self.activate()
        screenshot(self, path, title=TITLE)

    def wait_log(self, text, count=1):
        end = time.monotonic() + 20
        while time.monotonic() < end:
            if self.child.poll() is not None:
                raise RuntimeError(f'Application exited: {self.child.returncode}')
            if self.log_path.read_text().count(text) >= count:
                return
            time.sleep(.05)
        raise RuntimeError(f'Missing application observation: {text}')

    def rect(self, node):
        position, size = Point(), Point()
        for name, kind, result in [('AXPosition', 1, position), ('AXSize', 2, size)]:
            value = self.attr(node, name)
            try:
                if not value or not self.value(value, kind, C.byref(result)):
                    raise RuntimeError(f'Missing {name}')
            finally:
                if value:
                    self.release(value)
        return position.x, position.y, size.x, size.y

    def center(self, label, role):
        node = self.wait_find(TITLE, label, role)
        try:
            x, y, w, h = self.rect(node)
            return Point(x+w/2, y+h/2)
        finally:
            self.release(node)

    def focus(self, label, role):
        node = self.wait_find(TITLE, label, role)
        try:
            self.set(node, 'AXFocused', self.true)
            # AX setters enqueue native actions. Sending Home/Enter immediately
            # can still target the previously focused canvas on a busy desktop.
            # Observe completion; do not repeat the focus action or the keys.
            boolean = self.cf.CFBooleanGetValue
            boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
            deadline = time.monotonic() + 5
            while True:
                value = self.attr(node, 'AXFocused')
                try:
                    if value and boolean(value):
                        break
                finally:
                    if value:
                        self.release(value)
                if self.child.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f'Native focus was not accepted: {label}')
                time.sleep(.01)
        finally:
            self.release(node)

    def resize(self, width):
        window = self.window(TITLE)
        create = self.ax.AXValueCreate
        create.restype, create.argtypes = C.c_void_p, [C.c_int, C.c_void_p]
        try:
            for name, kind, point in [('AXPosition', 1, Point(20, 40)), ('AXSize', 2, Point(width, 860))]:
                value = create(kind, C.byref(point))
                try:
                    self.set(window, name, value)
                finally:
                    self.release(value)
        finally:
            self.release(window)
        time.sleep(.3)

    def wheel(self, point, pixels, *, control=False):
        self.send(5, point)
        wheel = self.cg.CGEventCreateScrollWheelEvent
        wheel.restype, wheel.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
        location = self.cg.CGEventSetLocation
        location.restype, location.argtypes = None, [C.c_void_p, Point]
        flags = self.cg.CGEventSetFlags
        flags.restype, flags.argtypes = None, [C.c_void_p, C.c_ulonglong]
        event = wheel(None, 0, 1, pixels)
        if not event:
            raise RuntimeError('Cannot create wheel event')
        try:
            location(event, point)
            flags(event, (1 << 18) if control else 0)
            self.post(0, event)
        finally:
            self.release(event)

    def active_layout_only(self, *, compact):
        ready = self.wait_find(TITLE, 'Model evaluation canvas', 'AXList')
        self.release(ready)
        labels = {
            ('AXList', 'Model evaluation canvas'): [],
            ('AXGroup', 'Latency across 24 evaluations'): [],
            ('AXGroup', 'Signal history and inspector'): [],
            ('AXStaticText', 'Canvas activity'): [],
            ('AXStaticText', 'Chart activity'): [],
        }
        deadline = time.monotonic() + 8
        def visit(node, depth):
            assert depth < 48 and time.monotonic() < deadline, 'AX traversal exceeded its bound'
            values, children = self.node_values(node)
            try:
                for (role, label), found in labels.items():
                    if values[0] == role and label in values[1:]:
                        found.append(self.rect(node))
                for child in children:
                    visit(child, depth + 1)
            finally:
                for child in children:
                    self.release(child)
        window = self.window(TITLE)
        try:
            visit(window, 0)
        finally:
            self.release(window)
        assert all(len(found) == 1 for found in labels.values()), labels
        canvas = labels[('AXList', 'Model evaluation canvas')][0]
        assert abs(canvas[2] - (490 if compact else 700)) < 1, canvas
        print(f'SIGNAL_ACTIVE_LAYOUT_ONLY compact={compact} labels=5 canvas_width={canvas[2]}', flush=True)

    def exercise(self, output, *, bundled=False):
        # Canvas/chart readiness needs the first active container-layout frame.
        # Activate our child before waiting; an occluded macOS window may defer it.
        deadline = time.monotonic() + 15
        while not self.has_window(TITLE):
            assert self.child.poll() is None and time.monotonic() < deadline
            time.sleep(.05)
        self.set(self.app, 'AXFrontmost', self.true)
        self.resize(1160)
        self.wait_log('SIGNAL_STUDIO: ready')
        self.wait_log('layout wide')
        self.active_layout_only(compact=False)
        self.capture(output / 'wide.png')
        self.press(TITLE, 'Alerts')
        self.notification_authorization = None
        if bundled:
            # A real .app has a notification identity. Observe its existing OS
            # authorization without requesting permission or opting into alerts.
            self.wait_log('alert authorization ')
            match = re.search(r'alert authorization \(Ok (\w+)\)', self.log_path.read_text())
            assert match, 'Packaged app did not obtain notification authorization status'
            self.notification_authorization = match[1]
            messages = {
                'Authorized': 'Desktop alerts are available. Enable them to opt in.',
                'Provisional': 'Desktop alerts are available. Enable them to opt in.',
                'Not_required': 'Desktop alerts are available. Enable them to opt in.',
                'Not_determined': 'Enable alerts to request notification permission.',
                'Denied': 'Notifications are denied. Run results stay in this window.',
            }
            self.wait_text(TITLE, messages[self.notification_authorization])
            self.capture(output / 'alerts-bundled.png')
        else:
            self.wait_text(TITLE, 'Desktop alerts are unavailable; run results stay in this window.')
            self.capture(output / 'alerts-unavailable.png')
        self.press(TITLE, 'Close alerts')
        self.press(TITLE, 'Increment counter, current value 0')
        self.wait_log('SIGNAL_STUDIO: run 1')
        self.focus('Increment counter, current value 1', 'AXButton')
        self.key(49)  # Space: native extension keyboard activation.
        self.wait_log('SIGNAL_STUDIO: run 2')
        self.press(TITLE, 'Swift')
        self.wait_text(TITLE, 'Swift · latency 200 ms · quality 25%')
        self.focus('Swift', 'AXStaticText')
        ready_before = self.log_path.read_text().count('chart ready')
        self.key(124, flags=1 << 17)
        self.wait_text(TITLE, 'Swift · latency 202 ms · quality 25%')
        self.wait_log('chart ready', ready_before + 1)
        start = self.center('Swift', 'AXStaticText')
        print('DRAG_START', start.x, start.y, flush=True)
        self.capture(output / 'before-drag.png')
        self.send(5, start)
        self.send(1, start)
        point = start
        try:
            for step in range(1, 13):
                point = Point(start.x+step*2, start.y-step)
                self.send(6, point)
                time.sleep(.02)
        finally:
            self.send(2, point)
        try:
            self.wait_log('canvas moved', 2)
        except RuntimeError:
            self.capture(output / 'failed-drag.png')
            self.dump(TITLE)
            raise
        self.wait_text(TITLE, 'Swift · latency 243 ms · quality 28%')
        canvas = self.center('Model evaluation canvas', 'AXList')
        self.wheel(canvas, -35)
        self.wait_log('viewport changed')
        self.wheel(canvas, 35, control=True)
        self.wait_log('viewport changed', 2)
        self.press(TITLE, 'Reset view')
        self.focus('Latency across 24 evaluations', 'AXGroup')
        self.key(115)
        self.key(36)
        self.wait_log('chart selected')
        self.press(TITLE, 'Hide inspector')
        self.wait_text(TITLE, 'Show inspector')
        time.sleep(.35)
        self.press(TITLE, 'Show inspector')
        self.wait_text(TITLE, 'Swift · latency 243 ms · quality 28%')
        self.press(TITLE, 'Lock control')
        self.wait_text(TITLE, 'Unlock control')
        point = self.center('Increment counter, current value 2', 'AXButton')
        self.send(5, point)
        self.send(1, point)
        self.send(2, point)
        time.sleep(.2)
        assert 'SIGNAL_STUDIO: run 3' not in self.log_path.read_text()
        self.press(TITLE, 'Unlock control')
        self.press(TITLE, 'Hide control')
        self.wait_text(TITLE, 'Show control')
        node = self.find(TITLE, 'Increment counter, current value 2', 'AXButton')
        if node:
            self.release(node)
            raise RuntimeError('Hidden component remains exposed')
        self.press(TITLE, 'Show control')
        self.wait_text(TITLE, 'Increment counter, current value 2')
        self.press(TITLE, 'Stream runs')
        self.wait_log('SIGNAL_STUDIO: run 5')
        self.capture(output / 'streaming.png')
        self.wait_log('stream complete')
        self.wait_log('SIGNAL_STUDIO: run 14')
        self.wait_text(TITLE, 'Increment counter, current value 14')
        self.capture(output / 'updated.png')
        self.resize(650)
        self.wait_log('layout compact')
        self.active_layout_only(compact=True)
        self.wait_text(TITLE, 'Increment counter, current value 14')
        self.capture(output / 'compact.png')
        self.resize(1160)
        self.wait_log('layout wide', 2)
        self.active_layout_only(compact=False)
        self.wait_text(TITLE, 'Swift · latency 243 ms · quality 28%')
        self.press(TITLE, 'Reset workspace')
        self.wait_log('extension mounted', 2)
        self.wait_text(TITLE, 'Increment counter, current value 0')
        self.close(TITLE)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('.cache/signal-studio'))
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/signal_studio/main.exe'),
                        help='Signal Studio executable, including an installed-library consumer')
    args = parser.parse_args()
    def timeout(_signal, _frame):
        raise TimeoutError("Signal Studio walkthrough exceeded 120 seconds")
    signal.signal(signal.SIGALRM, timeout)
    args.output.mkdir(parents=True, exist_ok=True)
    path = args.output / 'application.log'
    with path.open('w') as log:
        child = subprocess.Popen([str(args.executable.resolve()), '--exit-on-close'], stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(120)
            mac = Studio(child, path)
            mac.exercise(args.output)
            assert child.wait(timeout=15) == 0
            print('SIGNAL_STUDIO_APPKIT_OK: extension AX/key/disabled/hidden events, canvas select/key/drag/pan/zoom, chart selection, inspector, stream, responsive state, remount, OS close', flush=True)
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


if __name__ == '__main__':
    main()
