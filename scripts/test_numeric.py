#!/usr/bin/env python3
"""Exercise public Bonsai/Eio sliders through macOS AX and OS keyboard events."""
import ctypes as C
from pathlib import Path
import subprocess
import tempfile
import time

from test_agent_chat import Mac

TITLE = 'GPUIO numeric controls'


class Numeric(Mac):
    def number(self, node, attribute='AXValue'):
        value = self.attr(node, attribute)
        if not value:
            raise RuntimeError(f'Missing numeric {attribute}')
        try:
            get = self.cf.CFNumberGetValue
            get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
            result = C.c_double()
            if not get(value, 13, C.byref(result)):
                raise RuntimeError(f'Invalid numeric {attribute}')
            return result.value
        finally:
            self.release(value)

    def expect_value(self, label, expected, minimum=None, maximum=None):
        deadline = time.monotonic() + 5
        actual = None
        while time.monotonic() < deadline:
            node = self.wait_find(TITLE, label, 'AXSlider')
            try:
                actual = self.number(node)
                if actual == expected:
                    if minimum is not None:
                        assert self.number(node, 'AXMinValue') == minimum
                    if maximum is not None:
                        assert self.number(node, 'AXMaxValue') == maximum
                    return
            finally:
                self.release(node)
            time.sleep(0.02)
        raise RuntimeError(f'{label}: expected {expected}, got {actual}')

    def adjust(self, label, *, key=None, action=None, value=None):
        node = self.wait_find(TITLE, label, 'AXSlider')
        try:
            self.set(node, 'AXFocused', self.true)
            if key is not None:
                self.key(key)
            if action:
                self.perform(node, action)
            if value is not None:
                create = self.cf.CFNumberCreate
                create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, C.c_void_p]
                number = C.c_double(value)
                encoded = create(None, 13, C.byref(number))
                try:
                    self.set(node, 'AXValue', encoded)
                finally:
                    self.release(encoded)
        finally:
            self.release(node)


def exercise(mac):
    mac.expect_value('Minimum output', 2, -2, 7)
    mac.expect_value('Maximum output', 7, 2, 8)
    mac.adjust('Minimum output', key=124)  # Right
    mac.expect_value('Minimum output', 2.5)
    mac.key(48)  # Tab moves to the independent upper thumb.
    mac.key(123)  # Left
    mac.expect_value('Maximum output', 6.5, 2.5, 8)
    mac.adjust('Maximum output', value=5)
    mac.expect_value('Maximum output', 5)
    mac.press(TITLE, 'Read-only')
    mac.release(mac.wait_find(TITLE, 'Allow edits', 'AXButton'))
    mac.adjust('Minimum output', key=124)
    time.sleep(0.15)
    mac.expect_value('Minimum output', 2.5)
    mac.press(TITLE, 'Allow edits')
    mac.press(TITLE, 'Disable')
    mac.press(TITLE, 'Reset range')  # Explicit programmatic replace is allowed.
    mac.expect_value('Minimum output', 0)
    mac.expect_value('Maximum output', 6)
    mac.press(TITLE, 'Enable')
    mac.press(TITLE, 'Unmount')
    mounted = mac.wait_find(TITLE, 'Mount', 'AXButton')
    mac.release(mounted)
    removed = mac.find(TITLE, 'Minimum output', 'AXSlider')
    if removed:
        mac.release(removed)
        raise RuntimeError('Unmounted slider remains accessible')
    mac.press(TITLE, 'Mount')
    mac.expect_value('Minimum output', 2)
    mac.expect_value('Maximum output', 7)

    mac.expect_value('Single linear value', 3, -2, 8)
    mac.adjust('Single linear value', action='AXIncrement')
    mac.expect_value('Single linear value', 3.5)
    mac.wait_text(TITLE, '(Single 3.5)')  # OCaml receives the native observation.
    mac.adjust('Single linear value', key=115)  # Home
    mac.expect_value('Single linear value', -2)
    mac.adjust('Single linear value', key=119)  # End
    mac.expect_value('Single linear value', 8)
    mac.adjust('Logarithmic value', key=126)  # Up
    mac.expect_value('Logarithmic value', 33, 1, 1000)
    mac.adjust('Logarithmic value', key=119)
    mac.expect_value('Logarithmic value', 1000)
    mac.adjust('Logarithmic value', key=121)  # PageDown
    mac.expect_value('Logarithmic value', 990)
    mac.adjust('Logarithmic interval lower', action='AXIncrement')
    mac.expect_value('Logarithmic interval lower', 11, 1, 100)
    mac.adjust('Logarithmic interval upper', action='AXDecrement')
    mac.expect_value('Logarithmic interval upper', 99, 11, 1000)
    mac.adjust('Logarithmic interval lower', value=200)  # Stop at the other thumb.
    mac.expect_value('Logarithmic interval lower', 99, 1, 99)
    mac.press(TITLE, 'Close')


def main():
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/numeric/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Numeric(child.pid, child)
            exercise(mac)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Numeric app exited unsuccessfully')
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
            print(log.read(), end='')
    print('GPUIO_NUMERIC_APP_AX_OK: all slider modes, OS keyboard, AX values/actions, read-only/disabled, remount, close')


if __name__ == '__main__':
    main()
