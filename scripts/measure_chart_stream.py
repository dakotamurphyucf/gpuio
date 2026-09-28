#!/usr/bin/env python3
"""Measure the public OCaml chart streaming workload; close/reap its owned child."""
import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import time


def command(*args):
    return subprocess.check_output(args, text=True).strip()


def summary(values):
    ordered = sorted(values)
    return {'median': statistics.median(ordered),
            'p95': ordered[math.ceil(len(ordered) * .95) - 1], 'max': ordered[-1]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('.cache/chart-stream'))
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/chart_stream/main.exe'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'platform': platform.platform(), 'architecture': platform.machine(),
              'revision': command('git', 'rev-parse', 'HEAD'),
              'dirty': bool(command('git', 'status', '--porcelain')),
              'executable': str(args.executable),
              'measurement': 'development build; foreground render callbacks, not physical presentation; whole-process CPU/RSS'}
    if platform.system() == 'Darwin':
        report['hardware'] = command('sysctl', '-n', 'machdep.cpu.brand_string')
        report['memory_bytes'] = int(command('sysctl', '-n', 'hw.memsize'))
    report['ocaml'] = command('./scripts/gpuio', 'exec', 'ocamlc', '-version')
    report['rust'] = command('rustc', '--version')
    started = time.monotonic()
    usage = None
    log_path = args.output / 'application.log'
    with log_path.open('w') as log:
        child = subprocess.Popen([str(args.executable.resolve())], stdout=log, stderr=subprocess.STDOUT)
        try:
            while usage is None:
                pid, status, current = os.wait4(child.pid, os.WNOHANG)
                if pid:
                    child.returncode = os.waitstatus_to_exitcode(status)
                    usage = current
                    break
                if time.monotonic() - started > 200:
                    raise TimeoutError('Chart workload exceeded 200 seconds')
                time.sleep(.1)
            if child.returncode:
                raise RuntimeError(f'Chart workload failed: {child.returncode}; see {log_path}')
        finally:
            if child.returncode is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
    report['wall_seconds'] = time.monotonic() - started
    report['cpu_user_seconds'] = usage.ru_utime
    report['cpu_system_seconds'] = usage.ru_stime
    report['peak_rss_bytes'] = usage.ru_maxrss * (1 if platform.system() == 'Darwin' else 1024)
    output = log_path.read_text()
    samples = []
    for line in output.splitlines():
        if 'CHART_STREAM_SAMPLE ' in line:
            samples.append({k: float(v) for k, v in re.findall(r'(\w+)=([\d.]+)', line)})
        if 'CHART_STREAM_OK ' in line:
            report['final'] = {k: int(v) for k, v in re.findall(r'(\w+)=(\d+)', line)}
    if len(samples) != 80 or 'final' not in report:
        raise RuntimeError(f'Incomplete chart workload: {len(samples)} samples; see {log_path}')
    report['workloads'] = []
    for count, exact, burst in [(10000, 0, 1), (100000, 0, 1), (100000, 1, 1), (100000, 0, 8)]:
        group = [s for s in samples if (s['points'], s['exact'], s['burst']) == (count, exact, burst)]
        report['workloads'].append({
            'points': count, 'exact': bool(exact), 'burst': burst, 'samples': len(group),
            **{key: summary([s[key] for s in group]) for key in
               ['build_ms', 'update_frame_ms', 'representatives', 'vertices', 'quads',
                'plan_bytes', 'source_charge', 'submitted_bytes', 'submitted_messages']}})
    (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    print('CHART_STREAM_MEASUREMENT_OK', flush=True)


if __name__ == '__main__':
    main()
