#!/usr/bin/env python3
"""Reject incomplete table traversal, fake paging and violated cache budgets."""
import unittest

from measure_table_history import validate
from test_measure_list_history import fixture as list_fixture


def fixture():
    lines = [line for line in list_fixture().splitlines() if 'growth-' not in line]
    return ('\n'.join(lines)
            .replace('GPUIO_PERF ', 'GPUIO_TABLE_PERF ')
            .replace('config (96 true false 2)', 'config (1030 64 128 4 true false 2)')
            .replace('history (96 96 32)', 'history (1030 1030 1030 64 28 1792 512 20 16 512)')
            .replace('complete 96', 'complete 1030')
            .replace('GPUIO_TABLE_PERF cleanup ', 'GPUIO_TABLE_PERF idle-activation ((false)(false)0)\nGPUIO_TABLE_PERF cleanup ')
            .replace('GPUIO_TABLE_PERF finish (history',
                     'GPUIO_TABLE_PERF interactions (0 0 515 63)\nGPUIO_TABLE_PERF finish (history'))


class TableReportTests(unittest.TestCase):
    def validate(self, text):
        return validate(text, smoke=True, background=False)

    def test_complete_paged_coverage(self):
        result = self.validate(fixture())
        self.assertEqual(result['coverage']['page_evictions'], 16)
        self.assertEqual(result['history']['histograms']['draw']['count'], 2)

    def test_truncated_and_extra_records(self):
        lines = fixture().splitlines()
        for size in range(len(lines)):
            with self.subTest(size=size), self.assertRaises(ValueError):
                self.validate('\n'.join(lines[:size]))
        with self.assertRaises(ValueError):
            self.validate(fixture() + '\nGPUIO_TABLE_PERF complete 1030')

    def test_missing_coverage_or_exceeded_budgets(self):
        for old, new in [('1030 1030 1030', '1030 1029 1030'),
                         ('interactions (0 0 515 63)', 'interactions (1 0 515 63)'),
                         ('1030 1030 1030', '1030 1030 1029'),
                         ('64 28 1792', '63 28 1792'), ('64 28 1792', '64 33 2112'),
                         ('28 1792', '28 1791'), ('512 20 16 512', '513 20 16 512'),
                         ('512 20 16 512', '512 8 4 512'),
                         ('(windows 0)', '(windows 1)'), ('(commands 0)', '(commands 1)'),
                         ('(elapsed_ns 2000000000)', '(elapsed_ns 1)'),
                         ('(total 2)', '(total 3)')]:
            with self.subTest(new=new), self.assertRaises(ValueError):
                self.validate(fixture().replace(old, new))


if __name__ == '__main__':
    unittest.main()
