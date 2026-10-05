#!/usr/bin/env python3
"""Verify lifecycle coverage, baseline limits, handshake sampling and child cleanup."""
import copy
import hashlib
import re
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
from measure_resource_lifecycle import Checkpoints, validate, entity_records, metal_record, metal_records


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
    def test_metal_records_boundaries_and_invalid_coverage(self):
        rows = [f'GPUIO_METAL_AUDIT checkpoint ({n} 123 4 90000000 1024)' for n in range(1, 34)]
        output = '\n'.join(rows)
        result = metal_records(output, warmups=3, measurements=30, smoke=False)
        self.assertTrue(result['qualification'])
        self.assertEqual(result['last_minus_warmup_bytes'], 0)
        for delta, passed in [(64 * 1024**2, True), (64 * 1024**2 + 1, False)]:
            changed = rows.copy()
            changed[-2] = f'GPUIO_METAL_AUDIT checkpoint (32 123 4 90000000 {1024 + delta})'
            self.assertEqual(metal_records('\n'.join(changed), warmups=3, measurements=30,
                                           smoke=False)['qualification'], passed)
        for text in ['\n'.join(rows[:-1]), output+'\n'+rows[-1], '\n'.join(reversed(rows)),
                     output.replace('(20 123 ', '(20 456 '), output.replace('(20 ', '(19 ')]:
            with self.assertRaises(ValueError):
                metal_records(text, warmups=3, measurements=30, smoke=False)
        for payload in ['(1 123 0 4 5)', '(1 0 1 4 5)', '(1 123 1 -1 5)',
                        f'(1 123 1 4 {2**64})', '(1 123 1 4)', '(1 123 1 4 5 6)']:
            with self.assertRaises(ValueError):
                metal_record('GPUIO_METAL_AUDIT checkpoint ' + payload)
        self.assertFalse(metal_records('\n'.join(rows[:4]), warmups=1, measurements=3,
                                       smoke=True)['qualification'])

    def test_metal_checkpoint_required_before_ack_and_rejects_wrong_order(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / 'app.log'
            samples = []
            child = SimpleNamespace(pid=123, stdin=io.BytesIO())
            checkpoint = Checkpoints(log, samples, cycles=4, native_entities=True, warmups=1, metal_memory=True)
            text = (f'GPUIO_LIFECYCLE checkpoint (1 {snapshot()})\n'
                    'GPUIO_ENTITY_AUDIT checkpoint (1 baseline)\n')
            log.write_text(text)
            checkpoint(child)
            self.assertEqual(child.stdin.getvalue(), b'')
            self.assertEqual(samples, [])
            text += 'GPUIO_METAL_AUDIT checkpoint (1 123 4 2048 1024)\n'
            log.write_text(text)
            with patch('measure_resource_lifecycle.subprocess.check_output', return_value='12345'):
                checkpoint(child)
            self.assertEqual(child.stdin.getvalue(), b'continue 1\n')
            # A later device change fails before the next acknowledgement.
            log.write_text(text + f'GPUIO_LIFECYCLE checkpoint (2 {snapshot()})\n'
                           'GPUIO_ENTITY_AUDIT checkpoint (2 checked)\n'
                           'GPUIO_METAL_AUDIT checkpoint (2 456 4 2048 1024)\n')
            with self.assertRaisesRegex(ValueError, 'order or device'):
                checkpoint(child)
            self.assertEqual(child.stdin.getvalue(), b'continue 1\n')
            # Metal cannot precede its native entity audit or arrive unrequested.
            log.write_text('GPUIO_METAL_AUDIT checkpoint (1 123 4 2048 1024)\n')
            for enabled in (False, True):
                reader = Checkpoints(log, [], cycles=4, native_entities=True, metal_memory=enabled)
                with self.assertRaises(ValueError):
                    reader(child)

    def test_paired_entity_schema_fingerprints(self):
        root = Path(__file__).resolve().parent.parent / 'examples/resource_audit'
        digest = hashlib.sha256((root/'schema.txt').read_bytes()).hexdigest()
        for path in ['ocaml/gpuio_resource_audit.ml','rust/src/lib.rs']:
            self.assertEqual(re.findall(r'[a-f0-9]{64}', (root/path).read_text()), [digest])

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

    def test_native_audit_requires_both_records_before_ack_in_either_order(self):
        for audit_first in (False, True):
            with tempfile.TemporaryDirectory() as tmp:
                log = Path(tmp) / 'app.log'
                samples = []
                checkpoint = Checkpoints(log, samples, cycles=4, native_entities=True, warmups=1)
                child = SimpleNamespace(pid=123, stdin=io.BytesIO())
                lifecycle = f'GPUIO_LIFECYCLE checkpoint (1 {snapshot()})\n'
                audit = 'GPUIO_ENTITY_AUDIT checkpoint (1 baseline)\n'
                first, second = (audit, lifecycle) if audit_first else (lifecycle, audit)
                log.write_text(first)
                checkpoint(child)
                self.assertEqual(samples, [])
                self.assertEqual(child.stdin.getvalue(), b'')
                log.write_text(first + second)
                with patch('measure_resource_lifecycle.subprocess.check_output', return_value='12345'):
                    checkpoint(child)
                self.assertEqual(child.stdin.getvalue(), b'continue 1\n')
                log.write_text(first + second + 'GPUIO_ENTITY_AUDIT failed 2\n')
                with self.assertRaisesRegex(ValueError, 'audit failed'):
                    checkpoint(child)
                self.assertEqual(child.stdin.getvalue(), b'continue 1\n')

    def test_entity_report_rejects_missing_duplicate_failure_and_wrong_baseline(self):
        output = '\n'.join(['GPUIO_ENTITY_AUDIT checkpoint (1 baseline)',
                            'GPUIO_ENTITY_AUDIT checkpoint (2 checked)',
                            'GPUIO_ENTITY_AUDIT checkpoint (3 checked)',
                            'GPUIO_ENTITY_AUDIT checkpoint (4 checked)',
                            'GPUIO_ENTITY_AUDIT complete 4'])
        self.assertEqual(entity_records(output, warmups=1, measurements=3)['measurements'], 3)
        for text in [output.rsplit('\n',1)[0], output + '\nGPUIO_ENTITY_AUDIT complete 4',
                     output.replace('(2 checked)','(2 baseline)'),
                     output.replace('checkpoint (3 checked)','failed 3'),
                     output.replace('(3 checked)','(2 checked)')]:
            with self.assertRaises(ValueError):
                entity_records(text, warmups=1, measurements=3)

    def test_final_native_ack_waits_for_terminal_record(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / 'app.log'
            samples = []
            checkpoint = Checkpoints(log, samples, cycles=2, native_entities=True, warmups=1)
            child = SimpleNamespace(pid=123, stdin=io.BytesIO())
            text = ''
            for cycle, phase in [(1, 'baseline'),(2, 'checked')]:
                text += (f'GPUIO_ENTITY_AUDIT checkpoint ({cycle} {phase})\n'
                         f'GPUIO_LIFECYCLE checkpoint ({cycle} {snapshot()})\n')
                log.write_text(text)
                with patch('measure_resource_lifecycle.subprocess.check_output', return_value='12345'):
                    checkpoint(child)
            self.assertEqual(child.stdin.getvalue(), b'continue 1\n')
            log.write_text(text + 'GPUIO_ENTITY_AUDIT complete 2\n')
            with patch('measure_resource_lifecycle.subprocess.check_output', return_value='12345'):
                checkpoint(child)
            self.assertEqual(child.stdin.getvalue(), b'continue 1\ncontinue 2\n')

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
