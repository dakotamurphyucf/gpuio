#!/usr/bin/env python3
"""Exercise public numeric editors with macOS AX objects and OS keyboard events."""
from pathlib import Path
import subprocess
import tempfile
import time

from test_numeric import Numeric

TITLE = 'GPUIO numeric editors'


class Numbers(Numeric):
    def expect(self, label, draft, numeric=None, help_contains=None):
        deadline = time.monotonic() + 5
        actual = None
        while time.monotonic() < deadline:
            field = self.wait_find(TITLE, label, 'AXTextField')
            try:
                actual = self.text(field, 'AXValue')
                help_text = self.text(field, 'AXHelp')
            finally:
                self.release(field)
            if actual == draft and (help_contains is None or help_contains in (help_text or '')):
                incrementor = self.wait_find(TITLE, label, 'AXIncrementor')
                try:
                    if numeric is None:
                        assert self.text(incrementor, 'AXValue') == draft
                    elif self.number(incrementor) != numeric:
                        time.sleep(0.02)
                        continue
                    assert self.number(incrementor, 'AXMinValue') == -2
                    assert self.number(incrementor, 'AXMaxValue') == 8
                    return
                finally:
                    self.release(incrementor)
            time.sleep(0.02)
        raise RuntimeError(f'{label}: expected {draft!r}, got {actual!r}')

    def focus(self, label):
        field = self.wait_find(TITLE, label, 'AXTextField')
        try:
            self.set(field, 'AXFocused', self.true)
        finally:
            self.release(field)

    def step(self, label, action):
        node = self.wait_find(TITLE, label, 'AXIncrementor')
        try:
            self.perform(node, action)
        finally:
            self.release(node)


def exercise(mac):
    mac.expect('Temperature', '1.5', 1.5)
    mac.field(TITLE, 'Temperature', 'AXTextField', '-')
    mac.expect('Temperature', '-', help_contains='Complete the number')
    mac.key(36)  # Return rejects incomplete text without destroying it.
    mac.expect('Temperature', '-', help_contains='Complete the number')
    mac.key(53)  # Escape restores the last committed value.
    mac.expect('Temperature', '1.5', 1.5)
    mac.field(TITLE, 'Temperature', 'AXTextField', '99')
    mac.expect('Temperature', '99', help_contains='limited to -2 through 8')
    mac.key(36)
    mac.expect('Temperature', '8', 8)
    mac.key(125)  # Down
    mac.expect('Temperature', '7.5', 7.5)
    mac.wait_text(TITLE, 'Committed: (Number 7.5)')
    mac.step('Temperature', 'AXDecrement')
    mac.expect('Temperature', '7', 7)
    mac.press(TITLE, 'Read-only')
    mac.release(mac.wait_find(TITLE, 'Allow edits', 'AXButton'))
    mac.focus('Temperature')
    mac.key(126)
    time.sleep(0.1)
    mac.expect('Temperature', '7', 7)
    mac.press(TITLE, 'Allow edits')
    mac.press(TITLE, 'Disable')
    mac.release(mac.wait_find(TITLE, 'Enable', 'AXButton'))
    mac.press(TITLE, 'Reset value')
    mac.expect('Temperature', '1.5', 1.5)
    mac.press(TITLE, 'Enable')
    mac.press(TITLE, 'Unmount')
    mac.release(mac.wait_find(TITLE, 'Mount', 'AXButton'))
    removed = mac.find(TITLE, 'Temperature', 'AXTextField')
    if removed:
        mac.release(removed)
        raise RuntimeError('Unmounted numeric input remains accessible')
    mac.press(TITLE, 'Mount')
    mac.expect('Temperature', '1.5', 1.5)

    mac.expect('Stacked temperature', '2', 2)
    mac.press(TITLE, 'Stacked temperature increase')
    mac.expect('Stacked temperature', '2.5', 2.5)
    mac.press(TITLE, 'Stacked temperature decrease')
    mac.expect('Stacked temperature', '2', 2)
    mac.focus('Stacked temperature')
    mac.key(126)
    mac.expect('Stacked temperature', '2.5', 2.5)
    mac.focus('Keyboard temperature')
    mac.key(125)
    mac.expect('Keyboard temperature', '1.5', 1.5)
    mac.step('Keyboard temperature', 'AXIncrement')
    mac.expect('Keyboard temperature', '2', 2)
    mac.wait_text(TITLE, 'Committed: (Number 2)')
    mac.press(TITLE, 'Close')


def main():
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/numeric/number.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Numbers(child.pid, child)
            exercise(mac)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Numeric editor example exited unsuccessfully')
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
    print('GPUIO_NUMBER_APP_AX_OK: public draft/commit observations, OS keyboard, AX values/actions, three layouts, policy, remount and close')


if __name__ == '__main__':
    main()
