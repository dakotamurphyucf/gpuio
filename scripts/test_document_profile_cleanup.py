#!/usr/bin/env python3
"""Explicit runtime snapshots must include document retirement before acceptance."""
import contextlib
import io
import unittest
from unittest.mock import patch

import test_gallery as gallery


class Snapshot:
    def __init__(self, documents, source_bytes):
        self.documents, self.source_bytes = documents, source_bytes
        self.refreshes = 0
        self.now = 0.

    def press(self, title, label):
        assert label == 'Refresh resource counts'
        self.refreshes += 1

    def find(self, title, label, role, **kwargs):
        return label

    def text(self, node, attribute):
        index = self.refreshes - 1
        if node == 'Images:':
            return 'Images: 0 · Charts: 0 · Canvases: 0'
        values = self.documents if node == 'Documents:' else self.source_bytes
        value = values[min(index, len(values)-1)]
        return None if value is None else f'{node} {value}'

    def release(self, node):
        pass

    def sleep(self, seconds):
        self.now += seconds


class CleanupTests(unittest.TestCase):
    def wait(self, snapshot, *, documents=True):
        with patch.object(gallery.time, 'monotonic', lambda: snapshot.now), \
                patch.object(gallery.time, 'sleep', snapshot.sleep), \
                contextlib.redirect_stdout(io.StringIO()):
            gallery.wait_for_resource_cleanup(snapshot, documents=documents)

    def test_refreshes_document_counts_after_other_resources_are_zero(self):
        snapshot = Snapshot([2, 1, 0], [0])
        self.wait(snapshot)
        self.assertEqual(snapshot.refreshes, 3)

    def test_waits_for_source_bytes_after_document_count_is_zero(self):
        snapshot = Snapshot([0], [128, 0])
        self.wait(snapshot)
        self.assertEqual(snapshot.refreshes, 2)

    def test_persistent_registrations_bytes_or_missing_values_fail(self):
        for documents, source_bytes in [([2], [0]), ([0], [128]), ([None], [0]), ([0], [None])]:
            with self.subTest(documents=documents, source_bytes=source_bytes):
                snapshot = Snapshot(documents, source_bytes)
                with self.assertRaisesRegex(RuntimeError, 'Native registrations did not retire'):
                    self.wait(snapshot)
                self.assertGreaterEqual(snapshot.now, 35)
                self.assertLess(snapshot.now, 35.1)

    def test_existing_non_document_callers_keep_their_resource_scope(self):
        snapshot = Snapshot([2], [128])
        self.wait(snapshot, documents=False)
        self.assertEqual(snapshot.refreshes, 1)


if __name__ == '__main__':
    unittest.main()
