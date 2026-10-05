#!/usr/bin/env python3
"""Measure chart publication/readiness/render callbacks; retain failures and reap the child.

Requested focus and observed active state are not proof of visibility or physical
presentation. This diagnostic does not enforce release performance thresholds.
"""
import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import statistics
import subprocess
import time

WORKLOADS = [(10000, 0, 1, 30), (100000, 0, 1, 30),
             (100000, 1, 1, 10), (100000, 0, 8, 10)]
METRICS = ['build_ms', 'published_ms', 'published_ready_ms', 'ready_frame_ms',
           'update_frame_ms', 'representatives', 'vertices', 'quads', 'plan_bytes',
           'source_charge', 'submitted_bytes', 'submitted_messages']
TIMINGS = {name for name in METRICS if name.endswith('_ms')}
SAMPLE_FIELDS = set(METRICS) | {'points', 'exact', 'burst', 'iteration', 'active_after'}
STAGES = {'initial_publication', 'initial_ready', 'initial_frame', 'build',
          'publication', 'ready', 'frame', 'complete', 'cleanup'}


def command(*args):
    return subprocess.check_output(args, text=True).strip()


def summary(values):
    ordered = sorted(values)
    return {'median': statistics.median(ordered),
            'p95': ordered[math.ceil(len(ordered) * .95) - 1], 'max': ordered[-1]}


def fields(text):
    result = {}
    for token in text.split():
        key, separator, value = token.partition('=')
        if not separator or not value or key in result:
            raise ValueError(f'Invalid or duplicate measurement field: {token}')
        result[key] = value
    return result


def numbers(values, *, real=()):
    parsed = {}
    for key, value in values.items():
        number = float(value) if key in real else int(value)
        if not math.isfinite(number) or number < 0:
            raise ValueError(f'Invalid nonnegative metric: {key}={value}')
        parsed[key] = number
    return parsed


def parse_output(output):
    """Preserve every complete sample and the last valid phase even on failure."""
    result = {'samples': [], 'phases': [], 'parse_errors': []}
    for line_number, line in enumerate(output.splitlines(), 1):
        match = re.search(r'CHART_STREAM_(START|PHASE|SAMPLE|OK) (.*)', line)
        if not match:
            continue
        kind, text = match.groups()
        try:
            row = fields(text)
            if kind == 'START':
                if row != {'version': '2'} or 'version' in result:
                    raise ValueError('Missing/duplicate/unsupported measurement version')
                result['version'] = 2
            elif kind == 'PHASE':
                if set(row) != {'sequence', 'stage', 'elapsed_ms', 'active'}:
                    raise ValueError('Unexpected phase fields')
                stage = row.pop('stage')
                row = numbers(row, real={'elapsed_ms'})
                if stage not in STAGES or row['active'] not in (0, 1, 2):
                    raise ValueError('Unknown phase or active state')
                row['stage'] = stage
                result['phases'].append(row)
            elif kind == 'SAMPLE':
                if set(row) != SAMPLE_FIELDS:
                    raise ValueError('Missing or unknown sample fields')
                row = numbers(row, real=TIMINGS)
                if row['active_after'] not in (0, 1, 2):
                    raise ValueError('Unknown active state')
                total = sum(row[key] for key in
                            ['published_ms', 'published_ready_ms', 'ready_frame_ms'])
                if abs(total - row['update_frame_ms']) > .003:
                    raise ValueError('Stage durations do not sum to update-to-render duration')
                result['samples'].append(row)
            else:
                if 'final' in result:
                    raise ValueError('Duplicate completion marker')
                result['final'] = numbers(row)
        except (ValueError, OverflowError) as error:
            result['parse_errors'].append({'line': line_number, 'error': str(error)})
    result['last_phase'] = result['phases'][-1] if result['phases'] else None
    return result


def validate(result):
    if result['parse_errors'] or result.get('version') != 2:
        raise ValueError('Malformed or unversioned workload; see parse_errors')
    expected = [(count, exact, burst, iteration)
                for count, exact, burst, iterations in WORKLOADS
                for iteration in range(1, iterations + 1)]
    actual = [tuple(sample[key] for key in ('points', 'exact', 'burst', 'iteration'))
              for sample in result['samples']]
    if actual != expected:
        raise ValueError(f'Incomplete, duplicate or reordered workload: {len(actual)} samples')
    expected_phases = [(0, stage) for stage in
                       ('initial_publication', 'initial_ready', 'initial_frame')]
    expected_phases += [(sequence, stage) for sequence in range(1, 81)
                        for stage in ('build', 'publication', 'ready', 'frame', 'complete')]
    expected_phases.append((80, 'cleanup'))
    phases = result['phases']
    if ([(row['sequence'], row['stage']) for row in phases] != expected_phases
            or any(right['elapsed_ms'] < left['elapsed_ms']
                   for left, right in zip(phases, phases[1:]))):
        raise ValueError('Missing, reordered or nonmonotonic phase evidence')
    final = result.get('final', {})
    required = {'samples', 'updates', 'peak_source_charge', 'peak_pending',
                'peak_commands', 'final_source_charge', 'native_queue_peak_bytes'}
    if (set(final) != required or final['samples'] != 80 or final['updates'] != 150
            or final['final_source_charge'] != 0
            or final['peak_source_charge'] > 128 * 1024 * 1024):
        raise ValueError('Missing/invalid completion or cleanup evidence')
    for sample in result['samples']:
        if sample['exact'] and sample['representatives'] != sample['points']:
            raise ValueError('Exact mode did not retain every source value')
    workloads = []
    for count, exact, burst, _ in WORKLOADS:
        group = [sample for sample in result['samples']
                 if (sample['points'], sample['exact'], sample['burst']) == (count, exact, burst)]
        workloads.append({'points': count, 'exact': bool(exact), 'burst': burst,
                          'samples': len(group),
                          **{key: summary([sample[key] for sample in group]) for key in METRICS}})
    return workloads


def collect(executable, log_path, report, timeout, *, arguments=()):
    """Own one session, keep exact-child wait4 usage, and clean up on all exits."""
    started = time.monotonic()
    child = None
    usage = None

    def reap(block=False):
        nonlocal usage
        pid, status, current = os.wait4(child.pid, 0 if block else os.WNOHANG)
        if pid:
            child.returncode = os.waitstatus_to_exitcode(status)
            usage = current
        return bool(pid)

    with log_path.open('x') as log:
        try:
            child = subprocess.Popen([str(Path(executable).resolve()), *arguments], stdout=log,
                                     stderr=subprocess.STDOUT, start_new_session=True)
            report['child_pid'] = child.pid
            while not reap():
                remaining = timeout - (time.monotonic() - started)
                if remaining <= 0:
                    raise TimeoutError(f'Chart workload exceeded {timeout:g} seconds')
                time.sleep(min(.1, remaining))
            if child.returncode:
                raise RuntimeError(f'Chart workload exited with {child.returncode}')
        finally:
            if child is not None:
                # Ignore additional interrupts only during bounded cleanup, then
                # restore handlers. Never signal by process name or another PID.
                handlers = {sig: signal.signal(sig, signal.SIG_IGN)
                            for sig in (signal.SIGINT, signal.SIGTERM)}
                try:
                    try:
                        os.killpg(child.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    deadline = time.monotonic() + 3
                    while child.returncode is None and not reap() and time.monotonic() < deadline:
                        time.sleep(.02)
                    try:
                        os.killpg(child.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    if child.returncode is None:
                        reap(block=True)
                    report['returncode'] = child.returncode
                finally:
                    for sig, handler in handlers.items():
                        signal.signal(sig, handler)
            report['wall_seconds'] = time.monotonic() - started
            if usage is not None:
                report.update(cpu_user_seconds=usage.ru_utime,
                              cpu_system_seconds=usage.ru_stime,
                              peak_rss_bytes=usage.ru_maxrss * (1 if platform.system() == 'Darwin' else 1024))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('scratch/chart-stream'))
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/chart_stream/main.exe'))
    parser.add_argument('--timeout', type=float, default=200.)
    args = parser.parse_args()
    if not math.isfinite(args.timeout) or not 0 < args.timeout <= 3600:
        parser.error('--timeout must be finite and in (0, 3600] seconds')
    args.output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'platform': platform.platform(),
              'architecture': platform.machine(), 'executable': str(args.executable),
              'measurement': 'development build; accepted publication, render-dependent Ready, requested render callback; not physical presentation or GPU time',
              'active_state': '0 unknown, 1 inactive, 2 active; focus observations do not prove visibility or exclude occlusion'}
    log_path = args.output / 'application.log'

    def interrupted(signum, _frame):
        raise SystemExit(128 + signum)

    handlers = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    try:
        report.update(revision=command('git', 'rev-parse', 'HEAD'),
                      dirty=bool(command('git', 'status', '--porcelain')),
                      ocaml=command('./scripts/gpuio', 'exec', 'ocamlc', '-version'),
                      rust=command('rustc', '--version'))
        if platform.system() == 'Darwin':
            report['hardware'] = command('sysctl', '-n', 'machdep.cpu.brand_string')
            report['memory_bytes'] = int(command('sysctl', '-n', 'hw.memsize'))
        collect(args.executable, log_path, report, args.timeout)
        report.update(parse_output(log_path.read_text(errors='replace')))
        report['workloads'] = validate(report)
        report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        if log_path.exists():
            report.update(parse_output(log_path.read_text(errors='replace')))
        (args.output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
    print(f'CHART_STREAM_MEASUREMENT_OK report={args.output / "report.json"}', flush=True)


if __name__ == '__main__':
    main()
