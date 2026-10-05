#!/usr/bin/env python3
"""Sample one process after acknowledged window/resource retirement, without GC.

The child waits for an exact numbered acknowledgement after each checkpoint.
RSS is sampled before allowing the next window to allocate; OS physical footprint,
GPU allocations and native entity counts are not inferred from registry counters.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import signal
import subprocess
import time

from measure_chart_stream import collect, command
from measure_list_history import ZERO_RESOURCES, fields, natural, sexp


def retired(snapshot):
    state = fields(snapshot)
    if any(natural(state[k]) for k in ZERO_RESOURCES):
        raise ValueError('Owned resources remained at lifecycle checkpoint')
    queue = fields(state['native_command_queue'])
    if natural(queue['commands']) or natural(queue['bytes']):
        raise ValueError('Native command queue not drained')
    scopes = fields(state['scopes'])
    if {k: natural(v) for k, v in scopes.items()} != {'scopes': 1, 'tasks': 1, 'cleanups': 0}:
        raise ValueError('Window scope, task or cleanup survived retirement')
    return state


def validate(output, samples, *, smoke, background):
    records = []
    for line in output.splitlines():
        if line.startswith('GPUIO_LIFECYCLE '):
            name, payload = line[len('GPUIO_LIFECYCLE '):].split(' ', 1)
            records.append((name, sexp(payload)))
    warmups, measurements = (1, 3) if smoke else (3, 30)
    if not records or records.pop(0) != ('config', [str(warmups), str(measurements), str(background).lower()]):
        raise ValueError('Lifecycle configuration mismatch')
    if len(samples) != warmups + measurements:
        raise ValueError('Missing lifecycle RSS samples')
    cycles = []
    for cycle in range(1, warmups + measurements + 1):
        if len(records) < 2:
            raise ValueError('Truncated lifecycle records')
        exercised, checkpoint = records[:2]
        del records[:2]
        if exercised[0] != 'exercised' or checkpoint[0] != 'checkpoint':
            raise ValueError('Missing or reordered lifecycle phases')
        n, before = exercised[1]
        m, after = checkpoint[1]
        if natural(n) != cycle or natural(m) != cycle:
            raise ValueError('Missing or duplicate cycle')
        before = fields(before)
        if any(natural(before[k]) != 1 for k in ('windows', 'assets', 'documents', 'canvases')):
            raise ValueError('Required cycle resources were not registered')
        retired(after)
        sample = samples[cycle - 1]
        if sample['cycle'] != cycle or not isinstance(sample['rss_bytes'], int) or sample['rss_bytes'] <= 0:
            raise ValueError('Invalid or reordered memory sample')
        if not math.isfinite(sample['elapsed_seconds']) or sample['elapsed_seconds'] < 0:
            raise ValueError('Invalid sampling time')
        if cycle > 1 and sample['elapsed_seconds'] <= samples[cycle - 2]['elapsed_seconds']:
            raise ValueError('Nonmonotonic sample times')
        cycles.append(dict(cycle=cycle, warmup=cycle <= warmups,
                           exercised=before, retired=fields(after), **{k: v for k, v in sample.items() if k != 'cycle'}))
    if records != [('complete', str(measurements))]:
        raise ValueError('Missing or duplicate lifecycle completion')
    final = samples[-min(10, measurements):]
    values = [s['rss_bytes'] for s in final]
    growth = max(0, max(values) - values[0])
    return dict(cycles=cycles, baseline_cycles=[s['cycle'] for s in final],
                first_rss_bytes=values[0], last_rss_bytes=values[-1],
                peak_rss_bytes=max(values), range_bytes=max(values)-min(values),
                baseline_growth_bytes=growth,
                qualification=(not smoke and growth <= 64 * 1024**2))


class Checkpoints:
    def __init__(self, log, samples, *, cycles):
        self.log, self.samples, self.cycles = log, samples, cycles
        self.offset, self.pending, self.started = 0, b'', time.monotonic()

    def __call__(self, child):
        with self.log.open('rb') as stream:
            stream.seek(self.offset)
            data = stream.read(1024 * 1024)
            self.offset += len(data)
        lines = (self.pending + data).split(b'\n')
        self.pending = lines.pop()
        if len(self.pending) > 65536:
            raise ValueError('Oversized lifecycle record')
        for line in lines:
            prefix = b'GPUIO_LIFECYCLE checkpoint '
            if not line.startswith(prefix):
                continue
            number, snapshot = sexp(line[len(prefix):].decode())
            cycle = natural(number)
            if cycle != len(self.samples) + 1 or cycle > self.cycles:
                raise ValueError('Unexpected lifecycle checkpoint')
            retired(snapshot)
            # ps reports resident KiB on both supported platforms. The owned
            # child is blocked on stdin here, not allocating its next window.
            raw = subprocess.check_output(['ps', '-o', 'rss=', '-p', str(child.pid)],
                                          text=True, timeout=2).strip()
            rss = natural(raw) * 1024
            if rss == 0:
                raise ValueError('Missing resident-set measurement')
            self.samples.append(dict(cycle=cycle, rss_bytes=rss,
                                     elapsed_seconds=time.monotonic() - self.started))
            child.stdin.write(f'continue {cycle}\n'.encode())
            child.stdin.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--executable', type=Path, default=Path('_build/default/examples/performance_lifecycle/main.exe'))
    parser.add_argument('--build-profile', choices=('dev', 'release'), required=True)
    parser.add_argument('--smoke', action='store_true')
    parser.add_argument('--background', action='store_true')
    parser.add_argument('--check-budgets', action='store_true')
    parser.add_argument('--timeout', type=float, default=600)
    args = parser.parse_args()
    if not math.isfinite(args.timeout) or not 0 < args.timeout <= 3600:
        parser.error('Timeout must be finite and in (0,3600] seconds')
    if args.check_budgets and (args.smoke or args.build_profile != 'release'):
        parser.error('Budget checks require full optimized cycles')
    args.output.mkdir(parents=True, exist_ok=False)
    report = dict(complete=False, samples=[], platform=platform.platform(), architecture=platform.machine(),
                  build_profile=args.build_profile, smoke=args.smoke, background=args.background,
                  measurement='Settled process RSS and acknowledged application registrations; not GPU memory or native entity counts')
    log = args.output / 'application.log'
    def interrupted(signum, _frame):
        raise SystemExit(128 + signum)
    handlers = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    try:
        with args.executable.open('rb') as binary:
            report['executable_sha256'] = hashlib.file_digest(binary, 'sha256').hexdigest()
        report.update(revision=command('git', 'rev-parse', 'HEAD'), dirty=bool(command('git', 'status', '--porcelain')))
        if platform.system() == 'Darwin':
            report.update(hardware=command('sysctl', '-n', 'machdep.cpu.brand_string'),
                          memory_bytes=int(command('sysctl', '-n', 'hw.memsize')),
                          display=command('system_profiler', 'SPDisplaysDataType', '-json'),
                          power=command('pmset', '-g', 'batt'), thermal=command('pmset', '-g', 'therm'))
        arguments = [flag for enabled, flag in ((args.smoke, '--smoke'), (args.background, '--background')) if enabled]
        collect(args.executable, log, report, args.timeout, arguments=arguments,
                on_poll=Checkpoints(log, report['samples'], cycles=4 if args.smoke else 33))
        report['workload'] = validate(log.read_text(), report['samples'], smoke=args.smoke, background=args.background)
        if args.check_budgets and not report['workload']['qualification']:
            raise ValueError('Last ten-cycle RSS baseline growth exceeds 64 MiB')
        report['complete'] = True
    except BaseException as error:
        report['failure'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
    print(f'RESOURCE_LIFECYCLE_MEASUREMENT_OK report={args.output / "report.json"}')


if __name__ == '__main__':
    main()
