#!/usr/bin/env python3
"""Offline telemetry and real non-GUI process cleanup tests; no performance claim."""
import copy
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

import measure_chart_stream as measure


def fixture():
    lines = ['CHART_STREAM_START version=2']
    elapsed = 0

    def phase(sequence, stage):
        nonlocal elapsed
        elapsed += 1
        lines.append(f'CHART_STREAM_PHASE sequence={sequence} stage={stage} elapsed_ms={elapsed} active=2')

    for stage in ('initial_publication', 'initial_ready', 'initial_frame'):
        phase(0, stage)
    sequence = 0
    for count, exact, burst, iterations in measure.WORKLOADS:
        for iteration in range(1, iterations + 1):
            sequence += 1
            for stage in ('build', 'publication', 'ready', 'frame', 'complete'):
                phase(sequence, stage)
            row = dict.fromkeys(measure.METRICS, 1)
            row.update(points=count, exact=exact, burst=burst, iteration=iteration,
                       representatives=count, update_frame_ms=3, active_after=2)
            lines.append('CHART_STREAM_SAMPLE ' + ' '.join(f'{k}={v}' for k, v in row.items()))
    phase(80, 'cleanup')
    lines.append('CHART_STREAM_OK samples=80 updates=150 peak_source_charge=123 '
                 'peak_pending=1 peak_commands=0 final_source_charge=0 native_queue_peak_bytes=256')
    return '\n'.join(lines)


class Telemetry(unittest.TestCase):
    def test_stage_summaries_and_exact_workload(self):
        parsed = measure.parse_output(fixture())
        workloads = measure.validate(parsed)
        self.assertEqual([row['samples'] for row in workloads], [30, 30, 10, 10])
        self.assertEqual(workloads[2]['published_ready_ms'], {'median': 1, 'p95': 1, 'max': 1})
        self.assertEqual(parsed['last_phase']['stage'], 'cleanup')

    def test_equal_total_count_cannot_hide_missing_group_or_duplicate(self):
        parsed = measure.parse_output(fixture())
        parsed['samples'][-1] = parsed['samples'][0]
        with self.assertRaisesRegex(ValueError, 'duplicate or reordered'):
            measure.validate(parsed)

    def test_bad_values_and_partial_records_are_preserved_as_failure(self):
        for old, new in [('build_ms=1', 'build_ms=nan'),
                         ('build_ms=1', 'build_ms=-1'),
                         ('build_ms=1', 'build_ms=inf'),
                         ('update_frame_ms=3', 'update_frame_ms=4'),
                         ('iteration=1 ', 'iteration=1.5 '),
                         ('active_after=2', 'active_after=9'),
                         ('version=2', 'version=1')]:
            with self.subTest(new=new):
                parsed = measure.parse_output(fixture().replace(old, new, 1))
                self.assertTrue(parsed['parse_errors'])
                with self.assertRaises(ValueError):
                    measure.validate(parsed)
        parsed = measure.parse_output('CHART_STREAM_START version=2\n'
                                      'CHART_STREAM_PHASE sequence=1 stage=frame elapsed_ms=10 active=1\n'
                                      'CHART_STREAM_SAMPLE points=100000')
        self.assertEqual(parsed['last_phase']['stage'], 'frame')
        self.assertEqual(parsed['samples'], [])
        self.assertTrue(parsed['parse_errors'])

    def test_missing_stale_and_duplicate_completion_never_pass(self):
        for output in (fixture().rsplit('\n', 1)[0],
                       fixture().replace('final_source_charge=0', 'final_source_charge=1'),
                       fixture() + '\n' + fixture().splitlines()[-1],
                       fixture().replace('build_ms=1', 'build_ms=1 build_ms=2', 1)):
            with self.assertRaises(ValueError):
                measure.validate(measure.parse_output(output))

    def test_phase_sequence_monotonicity_and_exact_retention(self):
        baseline = measure.parse_output(fixture())
        missing = copy.deepcopy(baseline)
        missing['phases'].pop(4)
        backwards = copy.deepcopy(baseline)
        backwards['phases'][5]['elapsed_ms'] = 0
        sampled_exact = copy.deepcopy(baseline)
        sampled_exact['samples'][60]['representatives'] = 100
        for parsed in (missing, backwards, sampled_exact):
            with self.assertRaises(ValueError):
                measure.validate(parsed)


class ChildLifecycle(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='gpuio-chart-runner-test-')
        self.root = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()

    def child(self, body):
        path = self.root / 'child'
        path.write_text(f'#!{sys.executable}\n' + body)
        path.chmod(0o700)
        return path

    def assert_reaped(self, report):
        with self.assertRaises(ChildProcessError):
            os.waitpid(report['child_pid'], os.WNOHANG)
        with self.assertRaises(ProcessLookupError):
            os.kill(report['child_pid'], 0)
        self.assertIn('cpu_user_seconds', report)
        self.assertIn('peak_rss_bytes', report)

    def test_normal_and_failed_exit_keep_exact_child_usage(self):
        for code in (0, 7):
            report = {}
            executable = self.child(f'print("partial evidence", flush=True)\nraise SystemExit({code})\n')
            if code:
                with self.assertRaisesRegex(RuntimeError, 'exited with 7'):
                    measure.collect(executable, self.root / f'{code}.log', report, 2)
            else:
                measure.collect(executable, self.root / f'{code}.log', report, 2)
            self.assertEqual(report['returncode'], code)
            self.assert_reaped(report)

    def test_timeout_kills_and_reaps_term_ignoring_child(self):
        report = {}
        executable = self.child('import signal, time\nsignal.signal(signal.SIGTERM, signal.SIG_IGN)\n'
                                'print("CHART_STREAM_PHASE sequence=1 stage=frame elapsed_ms=1 active=1", flush=True)\n'
                                'time.sleep(30)\n')
        with self.assertRaises(TimeoutError):
            measure.collect(executable, self.root / 'timeout.log', report, .5)
        self.assertEqual(report['returncode'], -signal.SIGKILL)
        self.assertLess(report['wall_seconds'], 6)
        self.assert_reaped(report)

    def test_main_retains_incomplete_report_without_workload_success(self):
        executable = self.child('print("CHART_STREAM_START version=2", flush=True)\n')
        output = self.root / 'report'
        with patch.object(measure, 'command', return_value='1'), patch.object(sys, 'argv',
                ['measure', '--output', str(output), '--executable', str(executable)]):
            with self.assertRaises(ValueError):
                measure.main()
        report = json.loads((output / 'report.json').read_text())
        self.assertFalse(report['complete'])
        self.assertEqual(report['samples'], [])
        self.assertIn('error', report)
        self.assert_reaped(report)

    def test_sigterm_writes_partial_report_and_reaps_owned_child(self):
        marker = self.root / 'started'
        executable = self.child(f'import time\nfrom pathlib import Path\nPath({str(marker)!r}).touch()\ntime.sleep(30)\n')
        output = self.root / 'interrupted'
        script = ('import sys\nfrom unittest.mock import patch\nimport measure_chart_stream as m\n'
                  f'sys.argv=["measure", "--output", {str(output)!r}, "--executable", {str(executable)!r}]\n'
                  'with patch.object(m, "command", return_value="1"):\n    m.main()\n')
        wrapper = subprocess.Popen([sys.executable, '-c', script], cwd=Path(__file__).parent,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 5
            while not marker.exists() and wrapper.poll() is None and time.monotonic() < deadline:
                time.sleep(.02)
            self.assertTrue(marker.exists())
            wrapper.send_signal(signal.SIGTERM)
            _, stderr = wrapper.communicate(timeout=6)
            self.assertEqual(wrapper.returncode, 143, stderr)
            report = json.loads((output / 'report.json').read_text())
            self.assertFalse(report['complete'])
            self.assertIn('SystemExit: 143', report['error'])
            self.assertEqual(report['returncode'], -signal.SIGTERM)
            with self.assertRaises(ProcessLookupError):
                os.kill(report['child_pid'], 0)
        finally:
            if wrapper.poll() is None:
                wrapper.terminate()
                try:
                    wrapper.wait(timeout=6)
                except subprocess.TimeoutExpired:
                    wrapper.kill()
                    wrapper.wait()
            if wrapper.stderr:
                wrapper.stderr.close()


if __name__ == '__main__':
    unittest.main()
