"""Reject incomplete streaming, typing, pacing and native content evidence."""
import copy
import hashlib
import unittest

import measure_streaming_typing as m
from measure_list_history import ZERO_RESOURCES


def fixture():
    rows = []
    def emit(name, value):
        rows.append(m.PREFIX + name + ' ' + value)
    emit('config', '(true 4 80 40)')
    emit('begin', '(streams 17)')
    emit('ready', '40')
    for second in range(1, 5):
        progress = f'((updates {second*20})(bytes {second*20*128}))'
        emit('progress', f'({second} {second*1000000000+1000000} ({" ".join([progress]*4)}) {second*10})')
    for stream in range(4):
        times = ' '.join(str((index + 1)*50000000 + 1000000) for index in range(80))
        emit('ui-updates', f'({stream} ({times}))')
    emit('streams-complete', '(4001000000 12)')
    emit('typed', '(40 40)')
    emit('finish', '(streams (Finished (elapsed_ns 4100000000)(capture_ns 18)(dropped_inputs 0)(counts (1000 1000 0 1000 1000))))')
    for metric in range(5):
        count, values = (0, '') if metric == 2 else (1, '(1000000 1000)')
        emit('buckets', f'(streams (Buckets (metric {metric})(offset 0)(total {count})(values ({values}))))')
    for stream in range(4):
        emit('verify-row', f'({stream} 80)')
    resources = ' '.join(f'({key} 0)' for key in ZERO_RESOURCES)
    emit('cleanup', '(' + resources + ' (native_command_queue ((commands 0)(bytes 0))) (scopes ((scopes 1)(tasks 1)(cleanups 0))))')
    emit('complete', str(80*4*128))
    evidence = dict(input_source_restored=True, text='asdf'*10,
                    keys=[dict(index=i, elapsed_ns=50000000+i*100000000+1000000) for i in range(40)],
                    rows=[dict(stream=i, sha256=hashlib.sha256(m.final_row(i, 80).encode()).hexdigest()) for i in range(4)])
    return '\n'.join(rows), evidence


class StreamingQualificationTest(unittest.TestCase):
    def test_canonical_unicode_size_and_identity(self):
        for stream in range(4):
            for sequence in (1, 16, 17, 2400):
                text = m.fragment(stream, sequence)
                self.assertEqual(len(text.encode()), 128)
                self.assertIn('λ 世界', text)
        self.assertNotEqual(m.fragment(0, 1), m.fragment(1, 1))
        self.assertNotEqual(m.fragment(0, 1), m.fragment(0, 2))
        self.assertIn('fragment 002400', m.final_row(3, 2400))

    def test_complete_trace_keeps_all_timings_and_histograms(self):
        output, evidence = fixture()
        result = m.validate(output, evidence, smoke=True)
        self.assertEqual(len(result['progress']), 4)
        self.assertEqual([len(s['elapsed_ns']) for s in result['streams']], [80]*4)
        self.assertEqual(result['total_source_bytes'], 40960)
        self.assertEqual(m.budget_failures(result, 1000000), [])

    def test_rejects_missing_regressed_and_misidentified_content(self):
        output, evidence = fixture()
        edits = [
            output.replace('GPUIO_STREAM_PERF progress (2 ', 'MISSING (2 ', 1),
            output.replace('(updates 40)(bytes 5120)', '(updates 1)(bytes 128)', 1),
            output.replace('(updates 40)(bytes 5120)', '(updates 40)(bytes 1)', 1),
            output.replace('GPUIO_STREAM_PERF typed (40 40)', 'GPUIO_STREAM_PERF typed (39 40)'),
            output.replace('GPUIO_STREAM_PERF verify-row (2 80)', 'GPUIO_STREAM_PERF verify-row (1 80)'),
            output.replace('(4001000000 12)', '(4001000000 33)'),
            output.replace('(windows 0)', '(windows 1)'),
            output + '\n' + m.PREFIX + 'complete 40960',
        ]
        for invalid in edits:
            with self.subTest(invalid=invalid[-70:]), self.assertRaises(ValueError):
                m.validate(invalid, evidence, smoke=True)
        for change in ('key', 'text', 'row', 'restore', 'missing-time'):
            bad = copy.deepcopy(evidence)
            if change == 'key':
                bad['keys'][20]['index'] = 19
            elif change == 'text':
                bad['text'] = 'asdf'*9
            elif change == 'row':
                bad['rows'][2] = bad['rows'][1]
            elif change == 'restore':
                bad['input_source_restored'] = False
            else:
                bad['keys'].pop()
            with self.subTest(change=change), self.assertRaises(ValueError):
                m.validate(output, bad, smoke=True)

    def test_pacing_input_and_resource_budgets_are_independent(self):
        output, evidence = fixture()
        good = m.validate(output, evidence, smoke=True)
        changes = [
            lambda w: w['streams'][0]['lateness_ns'].__setitem__(0, 100000001),
            lambda w: w['key_lateness_ns'].__setitem__(0, 50000001),
            lambda w: w['interval'].__setitem__('dropped_inputs', 1),
            lambda w: w['interval']['histograms']['input_to_frame'].__setitem__('count', 999),
            lambda w: w['interval']['histograms']['draw'].__setitem__('p99', 33400001),
        ]
        for change in changes:
            bad = copy.deepcopy(good)
            change(bad)
            self.assertEqual(len(m.budget_failures(bad, 1000000)), 1)
        self.assertEqual(len(m.budget_failures(good, 1024**3+1)), 1)


if __name__ == '__main__':
    unittest.main()
