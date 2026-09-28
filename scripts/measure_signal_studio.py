#!/usr/bin/env python3
"""Measure combined Signal Studio streaming and exact native component lifetimes."""
import argparse
from collections import Counter, defaultdict
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import time
from measure_chart_stream import command, summary


def fields(line):
    return {k: float(v) for k, v in re.findall(r'(\w+)=([\d.]+)', line)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('scratch/signal-workload'))
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/signal_studio/main.exe'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {'platform': platform.platform(), 'architecture': platform.machine(),
              'revision': command('git', 'rev-parse', 'HEAD'),
              'dirty': bool(command('git', 'status', '--porcelain')),
              'measurement': 'development build; foreground native render callbacks, not physical presentation; whole-process CPU/RSS; opt-in component lifecycle traces'}
    if platform.system() == 'Darwin':
        report['hardware'] = command('sysctl', '-n', 'machdep.cpu.brand_string')
        report['memory_bytes'] = int(command('sysctl', '-n', 'hw.memsize'))
    report['ocaml'] = command('./scripts/gpuio', 'exec', 'ocamlc', '-version')
    report['rust'] = command('rustc', '--version')
    env = os.environ.copy()
    env['GPUIO_COUNTER_TRACE'] = '1'
    started = time.monotonic()
    usage = None
    log_path = args.output / 'application.log'
    with log_path.open('w') as log:
        child = subprocess.Popen([str(args.executable.resolve()), '--workload-check', '--reduced-motion'],
                                 stdout=log, stderr=subprocess.STDOUT, env=env)
        try:
            while usage is None:
                pid, status, current = os.wait4(child.pid, os.WNOHANG)
                if pid:
                    child.returncode = os.waitstatus_to_exitcode(status)
                    usage = current
                    break
                if time.monotonic() - started > 180:
                    raise TimeoutError('Signal Studio workload exceeded 180 seconds')
                time.sleep(.1)
            if child.returncode:
                raise RuntimeError(f'Signal Studio workload failed: {child.returncode}; see {log_path}')
        finally:
            if child.returncode is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
    report.update(wall_seconds=time.monotonic()-started, cpu_user_seconds=usage.ru_utime,
                  cpu_system_seconds=usage.ru_stime,
                  peak_rss_bytes=usage.ru_maxrss*(1 if platform.system() == 'Darwin' else 1024))
    samples, closed, commands, acks = [], [], [], []
    lifetimes = defaultdict(Counter)
    live_components, live_values = set(), set()
    retired_values = set()
    peak_live = 0
    for line in log_path.read_text().splitlines():
        if match := re.search(r'COUNTER_LIFETIME id=(\d+) phase=(\w+)', line):
            identity, phase = int(match[1]), match[2]
            lifetimes[identity][phase] += 1
            if phase == 'mount':
                live_components.add(identity)
                live_values.add(identity)
                peak_live = max(peak_live, len(live_values))
            elif phase == 'component_drop':
                live_components.remove(identity)
            elif phase == 'value_drop':
                live_values.remove(identity)
        if 'COUNTER_COMMAND ' in line:
            commands.append(fields(line))
        if match := re.search(r'extension command completed (\d+)', line):
            acks.append(int(match[1]))
        if 'workload sample ' in line:
            # The host can emit Window_closed before its current native frame's
            # callbacks finish dropping. They must be gone by the next rendered
            # sample, with no accumulation across window lifetimes.
            assert not (retired_values & live_values), (retired_values, live_values)
            retired_values.clear()
            samples.append(fields(line))
        if 'workload closed ' in line:
            row = fields(line)
            row.update(live_components=len(live_components), live_callback_values=len(live_values))
            assert not live_components and len(live_values) <= 1, row
            retired_values.update(live_values)
            closed.append(row)
        if 'workload passed ' in line:
            assert not live_components and not live_values
            report['final'] = fields(line)
    assert len(samples) == 96 and len(closed) == 12, (len(samples), len(closed))
    assert len(lifetimes) == 24, lifetimes
    expected = Counter(mount=1, unmount=1, component_drop=1, value_drop=1)
    assert all(counts == expected for counts in lifetimes.values()), lifetimes
    assert not live_components and not live_values
    assert peak_live <= 2, peak_live
    assert [c['value'] for c in commands] == list(range(20, 32)), commands
    assert acks == list(range(1, 13)), acks
    assert all(row['windows'] == 0 for row in closed), closed
    assert len({(row['scopes'], row['tasks'], row['source_charge']) for row in closed}) == 1, closed
    assert report['final']['final_source_charge'] == 0
    report.update(samples=len(samples), close_cycles=closed, native_lifetimes=len(lifetimes),
                  native_commands=len(commands), peak_live_callback_values=peak_live,
                  workloads={key: summary([row[key] for row in samples]) for key in
                             ('publish_ms', 'update_frame_ms', 'source_charge', 'submitted_bytes', 'submitted_messages')})
    (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    print('SIGNAL_STUDIO_WORKLOAD_OK: 384 desired updates, 96 render samples, 12 commands, '
          '12 close/reopen cycles, 24 native component/callback lifetimes, final source release', flush=True)


if __name__ == '__main__':
    main()
