#!/usr/bin/env python3
"""Portable preservation control-flow tests; native payload roundtrip is separate."""
import signal
import unittest

from mac_clipboard import preserved_clipboard


class Fake:
    def __init__(self, unavailable=False):
        self.unavailable, self.restored, self.closed = unavailable, False, False
        self.data = [[('public.utf8-plain-text', b'original'), ('custom.type', b'\0\xff')]]
    def snapshot(self):
        if self.unavailable:
            raise ValueError('promised data unavailable')
        return self.data
    def restore(self, saved):
        self.restored = True
        self.data = saved
    def close(self):
        self.closed = True


class Preservation(unittest.TestCase):
    def test_success_failure_and_interrupt_restore_all_representations(self):
        for failure in (None, ValueError('test failed'), KeyboardInterrupt()):
            board = Fake()
            original = board.data
            handler = signal.getsignal(signal.SIGTERM)
            try:
                with preserved_clipboard(lambda: board) as active:
                    active.data = [[('public.utf8-plain-text', b'test')]]
                    if failure:
                        raise failure
            except BaseException as error:
                self.assertIs(error, failure)
            self.assertEqual(board.data, original)
            self.assertTrue(board.closed and board.restored)
            self.assertEqual(signal.getsignal(signal.SIGTERM), handler)

    def test_unavailable_snapshot_does_not_modify_original(self):
        board = Fake(unavailable=True)
        with self.assertRaises(ValueError), preserved_clipboard(lambda: board):
            self.fail('unavailable snapshot must not start test')
        self.assertFalse(board.restored)
        self.assertTrue(board.closed)


if __name__ == '__main__':
    unittest.main()
