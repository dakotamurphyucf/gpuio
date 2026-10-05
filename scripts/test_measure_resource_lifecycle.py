#!/usr/bin/env python3
"""Verify lifecycle coverage, baseline limits, handshake sampling and child cleanup."""
import copy
import io
import os
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from measure_chart_stream import collect
from measure_list_history import ZERO_RESOURCES
from measure_resource_lifecycle import Checkpoints, validate


def snapshot(live=False):
    resources = ' '.join(f'({key} {int(live and key in ("windows", "assets", "documents", "canvases"))})' for key in ZERO_RESOURCES)
    return f'({resources} (native_command_queue ((commands 0)(bytes 0)(peak_bytes 100))) (scopes ((scopes 1)(tasks 1)(cleanups 0))))'


def fixture(smoke=False):
    warmups, measurements = (1, 3) if smoke else (3, 30)
    lines = [f'GPUIO_LIFECYCLE config ({warmups} {measurements} false)']
    samples = []
    for cycle in range(1, warmups + measurements + 1):
        lines += [f'GPUIO_LIFECYCLE exercised ({cycle} {snapshot(True)})',
                  f'GPUIO_LIFECYCLE checkpoint ({cycle} {snapshot()})']
        samples.append(dict(cycle=cycle, rss_bytes=100 * 1024**2, elapsed_seconds=cycle * 2.0))
    lines.append(f'GPUIO_LIFECYCLE complete {measurements}')
    return '\n'.join(lines), samples


class LifecycleReport(unittest.TestCase):
    def test_full_coverage_and_predeclared_growth(self):
        output, samples = fixture()
        result = validate(output, samples, smoke=False, background=False)
        self.assertTrue(result['qualification'])
        self.assertEqual(result['baseline_cycles'], list(range(24, 34)))
        samples[-2]['rss_bytes'] += 64 * 1024**2
        self.assertTrue(validate(output, samples, smoke=False, background=False)['qualification'])
        samples[-2]['rss_bytes'] += 1
        self.assertFalse(validate(output, samples, smoke=False, background=False)['qualification'])
        # A late dip does not hide earlier growth during the last ten cycles.
        self.assertEqual(samples[-1]['rss_bytes'], samples[-10]['rss_bytes'])
        short, samples = fixture(True)
        self.assertFalse(validate(short, samples, smoke=True, background=False)['qualification'])

    def test_missing_phases_leaks_and_bad_memory_never_pass(self):
        output, samples = fixture()
        for text in [output.rsplit('\n', 1)[0], output + '\nGPUIO_LIFECYCLE complete 30',
                     output.replace('checkpoint (2 ', 'checkpoint (1 '),
                     output.replace('(assets 0)', '(assets 1)', 1),
                     output.replace('(assets 1)', '(assets 0)', 1),
                     output.replace('(tasks 1)', '(tasks 2)'),
                     output.replace('(commands 0)', '(commands 1)')]:
            with self.subTest(text=text[:60]), self.assertRaises(ValueError):
                validate(text, samples, smoke=False, background=False)
        bad = copy.deepcopy(samples)
        bad[4]['elapsed_seconds'] = bad[3]['elapsed_seconds']
        negative = copy.deepcopy(samples)
        negative[-1]['rss_bytes'] = -1
        for rows in (samples[:-1], samples[::-1], negative, bad):
            with self.assertRaises(ValueError):
                validate(output, rows, smoke=False, background=False)

    def test_partial_records_wait_for_newline_and_sample_before_ack(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / 'app.log'
            samples = []
            checkpoint = Checkpoints(log, samples, cycles=4)
            child = SimpleNamespace(pid=123, stdin=io.BytesIO())
            text = f'GPUIO_LIFECYCLE checkpoint (1 {snapshot()})'
            log.write_text(text[:20])
            checkpoint(child)
            self.assertEqual(child.stdin.getvalue(), b'')
            log.write_text(text)
            checkpoint(child)
            self.assertEqual(samples, [])
            log.write_text(text + '\n')
            def sampled(*args, **kwargs):
                self.assertEqual(child.stdin.getvalue(), b'')
                return '12345\n'
            with patch('measure_resource_lifecycle.subprocess.check_output', side_effect=sampled):
                checkpoint(child)
            self.assertEqual(samples[0]['rss_bytes'], 12345 * 1024)
            self.assertEqual(child.stdin.getvalue(), b'continue 1\n')
            checkpoint(child)
            self.assertEqual(len(samples), 1)
            log.write_text(text + '\n' + text + '\n')
            with self.assertRaises(ValueError):
                checkpoint(child)

    def test_collector_handshake_and_callback_failure_reap_exact_child(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            child = root / 'child'
            child.write_text(f'#!{sys.executable}\nimport sys\nprint("ready", flush=True)\nassert input() == "go"\n')
            child.chmod(0o700)
            for fail in (False, True):
                report = {}
                log = root / f'{fail}.log'
                sent = False
                def tick(process):
                    nonlocal sent
                    if sent or 'ready' not in log.read_text():
                        return
                    if fail:
                        raise ValueError('sampling failed')
                    process.stdin.write(b'go\n')
                    process.stdin.flush()
                    sent = True
                if fail:
                    with self.assertRaisesRegex(ValueError, 'sampling failed'):
                        collect(child, log, report, 5, on_poll=tick)
                else:
                    collect(child, log, report, 5, on_poll=tick)
                    self.assertEqual(report['returncode'], 0)
                with self.assertRaises(ChildProcessError):
                    os.waitpid(report['child_pid'], os.WNOHANG)
                with self.assertRaises(ProcessLookupError):
                    os.kill(report['child_pid'], 0)
                self.assertGreater(report['peak_rss_bytes'], 0)


if __name__ == '__main__':
    unittest.main()
