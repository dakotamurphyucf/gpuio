#!/usr/bin/env python3
"""Exercise both public OTP modes through macOS AX and OS keyboard events."""
from pathlib import Path
import ctypes as C
import os
import signal
import subprocess
import tempfile
import time

from test_agent_chat import Mac

TITLE = 'GPUIO verification inputs'
DIGITS = 'Six-digit verification code'
LETTERS = 'Case-sensitive recovery code'
COMMAND = 1 << 20
SHIFT = 1 << 17


class Codes(Mac):
    def focused(self, label, role='AXTextField'):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            field = self.wait_find(TITLE, label, role)
            value = self.attr(field, 'AXFocused')
            try:
                get = self.cf.CFBooleanGetValue
                get.restype, get.argtypes = C.c_bool, [C.c_void_p]
                if value and get(value):
                    return
            finally:
                if value:
                    self.release(value)
                self.release(field)
            time.sleep(.02)
        raise RuntimeError(f'Tab navigation did not focus {label}')

    def tab_between_codes(self):
        self.focus(DIGITS)
        self.focused(DIGITS)
        self.key(48)
        self.focused(LETTERS)
        self.key(48, SHIFT)
        self.focused(DIGITS)

    def expect(self, label, expected, *, secure=False):
        deadline = time.monotonic() + 5
        actual = None
        while time.monotonic() < deadline:
            field = self.wait_find(TITLE, label, 'AXTextField')
            try:
                actual = self.text(field, 'AXValue')
                if secure:
                    assert self.text(field, 'AXSubrole') == 'AXSecureTextField'
                    if actual is None or all(ch == '•' for ch in actual):
                        return
                elif actual == expected:
                    return
            finally:
                self.release(field)
            time.sleep(0.02)
        raise RuntimeError(f'{label}: expected {expected!r}, got {actual!r}')

    def focus(self, label):
        field = self.wait_find(TITLE, label, 'AXTextField')
        try:
            self.set(field, 'AXFocused', self.true)
        finally:
            self.release(field)

    def button(self, label, next_label=None):
        self.press(TITLE, label)
        if next_label:
            self.release(self.wait_find(TITLE, next_label, 'AXButton'))


def exercise(mac):
    mac.expect(DIGITS, '12')
    mac.expect(LETTERS, 'Ab')
    mac.tab_between_codes()
    mac.field(TITLE, DIGITS, 'AXTextField', '１２３４５６')
    mac.expect(DIGITS, '123456')
    mac.wait_text(TITLE, 'Code filled. No authentication request was sent.')
    mac.key(123)  # Left from final caret.
    mac.key(51)   # Backspace removes the preceding cell.
    mac.expect(DIGITS, '12346')
    mac.wait_text(TITLE, '5 / 6 characters')
    mac.key(6, COMMAND)  # Undo.
    mac.expect(DIGITS, '123456')
    mac.key(6, COMMAND | SHIFT)  # Redo.
    mac.expect(DIGITS, '12346')
    mac.field(TITLE, DIGITS, 'AXTextField', '12x')
    mac.expect(DIGITS, '12346')  # Atomic invalid input rejection.
    mac.wait_text(TITLE, 'Unexpected_character')
    mac.button('Fill sample')
    mac.expect(DIGITS, '123456')
    mac.button('Read-only', 'Allow edits')
    mac.tab_between_codes()  # Read-only still participates in keyboard navigation.
    mac.focus(DIGITS)
    mac.key(0, COMMAND)  # Select all remains available.
    mac.key(51)
    mac.expect(DIGITS, '123456')
    mac.button('Allow edits', 'Read-only')
    mac.button('Mask', 'Reveal')
    mac.expect(DIGITS, None, secure=True)
    mac.button('Reveal', 'Mask')
    mac.expect(DIGITS, '123456')
    mac.button('Disable', 'Enable')
    mac.focus(LETTERS)
    mac.key(48, SHIFT)
    mac.focused('Close', 'AXButton')  # Disabled digits are skipped.
    mac.button('Clear')  # Explicit replacement is permitted when disabled.
    mac.expect(DIGITS, '')
    mac.button('Enable', 'Disable')
    mac.button('Unmount', 'Remount')
    stale = mac.find(TITLE, DIGITS, 'AXTextField')
    if stale:
        mac.release(stale)
        raise RuntimeError('Unmounted OTP remains accessible')
    mac.button('Remount', 'Unmount')
    mac.expect(DIGITS, '12')
    mac.tab_between_codes()  # Remounted handles join the native focus order.
    mac.field(TITLE, LETTERS, 'AXTextField', 'ａｂ１２ＣＤ３４')
    mac.expect(LETTERS, 'ab12CD34')
    mac.key(0, COMMAND)
    mac.key(51)
    mac.expect(LETTERS, '')
    mac.key(6, COMMAND)
    mac.expect(LETTERS, 'ab12CD34')
    mac.button('Clear recovery')
    mac.expect(LETTERS, '')
    mac.button('Fill recovery sample')
    mac.expect(LETTERS, 'Ab12Cd34')
    mac.wait_text(TITLE, '8 / 8 characters')
    mac.button('Close')


def main():
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/numeric/otp.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT,
                                 text=True, start_new_session=True)
        mac = None
        try:
            mac = Codes(child.pid, child)
            exercise(mac)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('OTP example exited unsuccessfully')
        finally:
            if mac:
                mac.release(mac.app)
            try:
                os.killpg(child.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
            log.seek(0)
            print(log.read(), end='')
    print('GPUIO_OTP_APP_AX_OK: both alphabets, normalization, OS selection/delete/history, '
          'Tab/Shift-Tab order, completion/rejection observations, protected value, policy, remount and close')


if __name__ == '__main__':
    main()
