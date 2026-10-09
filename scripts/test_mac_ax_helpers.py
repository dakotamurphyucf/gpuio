#!/usr/bin/env python3
"""Portable traversal/deadline checks; no macOS frameworks or UI are loaded."""
import contextlib
import io
import unittest
from unittest.mock import patch

from test_agent_chat import Mac
from test_agent_chat_sources import Sources
from test_agent_chat_dates_colors import DatesColors
from test_agent_chat_combined import Combined


class Tree(Mac):
    def __init__(self, count, cost):
        self.count = count
        self.cost = cost
        self.now = 0
        self.child = None
        self.released = []
        self.visited = []

    def window(self, title):
        return -1

    def retain(self, node):
        return node

    def release(self, node):
        self.released.append(node)

    def node_values(self, node):
        self.now += self.cost
        self.visited.append(node)
        if node == -1:
            return ['AXWindow', '', '', ''], list(range(self.count))
        return ['AXStaticText', f'item {node}', '', ''], []

    def sleep(self, duration):
        self.now += duration


class TraversalTests(unittest.TestCase):
    def test_specialized_walkthroughs_preserve_the_wait_deadline(self):
        # Exercise inherited wait_find through each actual override. This catches
        # incompatible signatures as well as replacing its shared deadline.
        for walkthrough in (Sources, DatesColors, Combined):
            with self.subTest(walkthrough=walkthrough.__name__):
                tree_type = type('WalkthroughTree', (Tree, walkthrough), {})
                tree = tree_type(50, .2)
                with patch('test_agent_chat.time.monotonic', lambda: tree.now):
                    self.assertEqual(tree.wait_find('app', 'item 1', search_files=True), 1)
                self.assertGreater(tree.now, 8)
                self.assertLess(tree.now, 35)

    def test_wait_reaches_nodes_beyond_the_default_lookup_budget(self):
        tree = Tree(50, .1)
        with patch('test_agent_chat.time.monotonic', lambda: tree.now):
            self.assertIsNone(tree.find('app', 'item 49'))
            tree.now = 0
            tree.visited.clear()
            self.assertEqual(tree.wait_find('app', 'item 49'), 49)
        self.assertEqual(tree.visited.count(-1), 1)
        self.assertGreater(tree.now, 3)
        self.assertLess(tree.now, 35)

    def test_missing_node_retains_overall_wait_and_diagnostic_bounds(self):
        tree = Tree(100, 1)
        output = io.StringIO()
        with patch('test_agent_chat.time.monotonic', lambda: tree.now), \
                patch('test_agent_chat.time.sleep', tree.sleep), \
                contextlib.redirect_stdout(output):
            with self.assertRaisesRegex(RuntimeError, 'Timed out finding'):
                tree.wait_find('app', 'absent')
        self.assertLess(tree.now, 39)
        self.assertIn('AX_DUMP_TRUNCATED', output.getvalue())

    def test_dump_bounds_output_and_releases_unvisited_children(self):
        tree = Tree(1000, .001)
        output = io.StringIO()
        with patch('test_agent_chat.time.monotonic', lambda: tree.now), \
                contextlib.redirect_stdout(output):
            tree.dump('app')
        self.assertEqual(len(tree.visited), 200)
        self.assertEqual(len(output.getvalue().splitlines()), 201)
        self.assertEqual(sorted(tree.released), list(range(-1, 1000)))

    def test_unbounded_native_call_is_not_claimed_preemptible(self):
        # A blocking AX call can overrun the traversal deadline. It must not
        # admit further node queries after returning; the helper cannot cancel it.
        tree = Tree(100, 5)
        with patch('test_agent_chat.time.monotonic', lambda: tree.now), \
                contextlib.redirect_stdout(io.StringIO()):
            tree.dump('app')
        self.assertEqual(tree.visited, [-1])
        self.assertEqual(sorted(tree.released), list(range(-1, 100)))


if __name__ == '__main__':
    unittest.main()
