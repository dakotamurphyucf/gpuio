#!/usr/bin/env python3
"""Portable checks for incomplete, reordered and dishonest measurement reports."""
import unittest

from measure_list_history import ZERO_RESOURCES, check_budgets, percentile, sexp, validate


def fixture():
    lines = ['config (96 true false 2)']
    for phase in ('history', 'idle'):
        lines.append(f'begin ({phase} 17)')
        if phase == 'history':
            lines += ['growth-start ((visible_last 6))',
                      'growth-complete ((visible_first 0)(visible_last 1)(anchor ((0 0))))']
        counts = '2 0 0 0 0' if phase == 'history' else '0 0 0 0 0'
        lines.append(f'finish ({phase}(Finished(elapsed_ns 2000000000)(capture_ns 20)'
                     f'(dropped_inputs 0)(counts({counts}))))')
        for metric in range(5):
            populated = phase == 'history' and metric == 0
            total, values = (2, '(100 1)(200 1)') if populated else (0, '')
            lines.append(f'buckets ({phase}(Buckets(metric {metric})(offset 0)'
                         f'(total {total})(values({values}))))')
        if phase == 'history':
            lines.append('history (96 96 32)')
    cleanup = ''.join(f'({name} 0)' for name in ZERO_RESOURCES)
    lines += [f'cleanup ({cleanup}(native_command_queue((commands 0)(bytes 0)(peak_bytes 100))))',
              'complete 96']
    return '\n'.join('GPUIO_PERF ' + line for line in lines)


class MeasurementTests(unittest.TestCase):
    def validate(self, text):
        return validate(text, smoke=True, background=False)

    def test_counts_raw_buckets_quantiles_and_idle(self):
        result = self.validate(fixture())
        self.assertEqual(result['history']['histograms']['draw'],
                         dict(count=2, buckets=[[100, 1], [200, 1]], p95=200, p99=200))
        self.assertIsNone(result['idle']['histograms']['draw']['p95'])
        self.assertEqual(percentile([[1, 95], [10, 4], [100, 1]], 95), 1)
        self.assertEqual(percentile([[1, 95], [10, 4], [100, 1]], 99), 10)

    def test_every_truncated_prefix_is_rejected(self):
        lines = fixture().splitlines()
        for length in range(len(lines)):
            with self.subTest(length=length), self.assertRaises(ValueError):
                self.validate('\n'.join(lines[:length]))

    def test_rejects_bad_counts_pages_and_resource_leaks(self):
        for old, new in [('(counts(2 0', '(counts(3 0'), ('(offset 0)', '(offset 1)'),
                         ('(100 1)(200 1)', '(200 1)(100 1)'), ('(100 1)', '(100 0)'),
                         ('(total 2)', '(total 3)'), ('(windows 0)', '(windows 1)'),
                         ('(commands 0)', '(commands 1)'), ('history (96 96 32)', 'history (96 95 32)'),
                         ('(elapsed_ns 2000000000)', '(elapsed_ns 1)'),
                         ('(visible_last 1)', '(visible_last 6)'),
                         ('(96 true false 2)', '(96 true true 2)')]:
            with self.subTest(new=new), self.assertRaises(ValueError):
                self.validate(fixture().replace(old, new))

    def test_duplicate_completion_is_rejected(self):
        with self.assertRaises(ValueError):
            self.validate(fixture() + '\nGPUIO_PERF complete 96')

    def test_sample_floor_and_thresholds_are_separate_from_functional_pass(self):
        result = self.validate(fixture())
        self.assertEqual(check_budgets(result, 1024), ['Fewer than 1000 draw samples'])
        draw = result['history']['histograms']['draw']
        draw.update(count=1000, p95=16_700_000, p99=33_400_000)
        self.assertEqual(check_budgets(result, 1024**3), [])
        draw['p99'] += 1
        self.assertEqual(check_budgets(result, 1024**3 + 1),
                         ['Draw p99 exceeds 33.4 ms', 'Peak RSS exceeds 1 GiB'])

    def test_bounded_restricted_sexp_parser(self):
        for text in ['()', '((x 0)(y(1 2)))']:
            self.assertIsInstance(sexp(text), list)
        for text in ['(', ')', '(x))', 'x y', '"quoted"', '('*17 + ')'*17, 'x'*65537]:
            with self.subTest(text=text[:40]), self.assertRaises(ValueError):
                sexp(text)


if __name__ == '__main__':
    unittest.main()
