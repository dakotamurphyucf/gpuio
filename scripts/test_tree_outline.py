#!/usr/bin/env python3
"""Drag and approve a sample tree move, then move back through its context menu.

macOS only. AX lookup and pointer hit-testing restrict input to the spawned child.
Uses in-memory sample data; the example never moves real files. Build tree/main.exe
first. Every exit path releases held buttons and reaps the child.
"""
import ctypes as C
from pathlib import Path
import subprocess
import sys
import tempfile
import time

from test_agent_chat import Mac

TITLE = 'GPUIO — Outline Lab'


class Point(C.Structure):
    _fields_ = [('x', C.c_double), ('y', C.c_double)]


class Outline(Mac):
    def __init__(self, pid, child):
        super().__init__(pid, child)
        def bind(lib, name, result, *args):
            fn = getattr(lib, name)
            fn.restype, fn.argtypes = result, args
            return fn
        ptr = C.c_void_p
        self.value = bind(self.ax, 'AXValueGetValue', C.c_bool, ptr, C.c_int, ptr)
        self.system = bind(self.ax, 'AXUIElementCreateSystemWide', ptr)
        self.hit = bind(self.ax, 'AXUIElementCopyElementAtPosition', C.c_int, ptr, C.c_float, C.c_float, C.POINTER(ptr))
        self.owner = bind(self.ax, 'AXUIElementGetPid', C.c_int, ptr, C.POINTER(C.c_int))
        self.mouse = bind(self.cg, 'CGEventCreateMouseEvent', ptr, ptr, C.c_int, Point, C.c_int)
        self.post = bind(self.cg, 'CGEventPost', None, C.c_int, ptr)
        self.integer = bind(self.cg, 'CGEventSetIntegerValueField', None, ptr, C.c_int, C.c_longlong)

    def row_center(self, label):
        row = self.wait_find(TITLE, label, 'AXRow', search_files=True)
        try:
            position, size = Point(), Point()
            for name, kind, result in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                value = self.attr(row, name)
                try:
                    if not value or not self.value(value, kind, C.byref(result)):
                        raise RuntimeError(f'Missing row geometry: {label}')
                finally:
                    if value:
                        self.release(value)
            if size.x <= 0 or size.y <= 0:
                raise RuntimeError('Empty row geometry')
            return Point(position.x + size.x / 2, position.y + size.y / 2)
        finally:
            self.release(row)

    def send(self, kind, point, button=0):
        if kind not in (2, 4):  # Always release our held mouse button on failure.
            root, hit, pid = self.system(), C.c_void_p(), C.c_int()
            try:
                if (self.hit(root, point.x, point.y, C.byref(hit)) or not hit.value
                        or self.owner(hit, C.byref(pid)) or pid.value != self.pid):
                    raise RuntimeError('Child window is obscured or moved; input stopped')
            finally:
                if hit.value:
                    self.release(hit)
                self.release(root)
        event = self.mouse(None, kind, point, button)
        if not event:
            raise RuntimeError('Cannot create mouse event')
        try:
            self.integer(event, 1, 1)
            self.post(0, event)
        finally:
            self.release(event)

    def exercise(self):
        self.row_center('Project plan')  # Wait for the child's AX server.
        self.set(self.app, 'AXFrontmost', self.true)
        window = self.window(TITLE)
        try:
            self.perform(window, 'AXRaise')
        finally:
            self.release(window)
        time.sleep(.2)
        source, destination = self.row_center('Project plan'), self.row_center('Archive')
        self.send(5, source)
        time.sleep(.1)
        self.send(1, source)
        point = source
        try:
            for step in range(1, 25):
                ratio = step / 24
                point = Point(source.x + (destination.x - source.x) * ratio,
                              source.y + (destination.y - source.y) * ratio)
                self.send(6, point)
                time.sleep(.02)
        finally:
            self.send(2, point)
        self.press(TITLE, 'Confirm move')
        self.wait_text(TITLE, '1 approved moves')
        point = self.row_center('Project plan')
        self.send(5, point)
        self.send(3, point, 1)
        try:
            time.sleep(.05)
        finally:
            self.send(4, point, 1)
        item = self.wait_find(TITLE, 'Move to Inbox', search_files=True)
        try:
            self.perform(item, 'AXPress')
        finally:
            self.release(item)
        self.press(TITLE, 'Confirm move')


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This test exercises the AppKit backend')
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/tree/main.exe'), '--outline', '--gesture-self-test'],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Outline(child.pid, child)
            mac.exercise()
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Outline example failed')
        finally:
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
            output = log.read()
            print(output, end='')
        if 'TREE_OUTLINE_GESTURE_PASS' not in output:
            raise RuntimeError('Missing native drag/menu approval acceptance marker')


if __name__ == '__main__':
    main()
