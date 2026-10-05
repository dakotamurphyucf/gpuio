#!/usr/bin/env python3
"""Four independent native-history streams with paced native composer key events."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import signal
import time

from measure_chart_stream import collect, command
from measure_list_history import fields, natural, read_interval, sexp
from measure_resource_lifecycle import retired

PREFIX = 'GPUIO_STREAM_PERF '
TITLE = 'GPUIO · Streaming and typing qualification'
LABEL = 'Qualification composer'


def fragment(stream, sequence):
    prefix = f'Stream {stream} / fragment {sequence:06} · λ 世界 '.encode()
    return (prefix + b'x' * (127 - len(prefix)) + b'\n').decode()


def final_row(stream, updates):
    block = (updates - 1) // 16
    return f'Stream {stream} · block {block}\n' + ''.join(
        fragment(stream, sequence) for sequence in range(block * 16 + 1, updates + 1))


def validate(output, evidence, *, smoke):
    rows = []
    for line in output.splitlines():
        if line.startswith(PREFIX):
            name, payload = line[len(PREFIX):].split(' ', 1)
            rows.append((name, sexp(payload)))
    def take(name):
        if not rows or rows[0][0] != name:
            raise ValueError(f'Expected streaming record {name}')
        return rows.pop(0)[1]
    seconds = 4 if smoke else 120
    updates, keys = seconds * 20, seconds * 10
    if evidence.get('input_source_restored') is not True:
        raise ValueError('Missing input-source restoration evidence')
    if take('config') != [str(smoke).lower(), str(seconds), str(updates), str(keys)]:
        raise ValueError('Streaming workload configuration mismatch')
    name, captured = take('begin')
    if name != 'streams' or natural(take('ready')) != keys:
        raise ValueError('Missing native timing or readiness handshake')
    progress = []
    for second in range(1, seconds + 1):
        n, elapsed, streams, typed = take('progress')
        streams = [{k: natural(v) for k, v in fields(s).items()} for s in streams]
        typed, elapsed = natural(typed), natural(elapsed)
        if (natural(n) != second or len(streams) != 4 or typed > keys
                or elapsed < second * 1_000_000_000):
            raise ValueError('Missing or premature progress observation')
        for stream in streams:
            if (set(stream) != {'updates', 'bytes'} or stream['bytes'] != stream['updates'] * 128
                    or not (second - 1) * 20 <= stream['updates'] <= updates):
                raise ValueError('A stream stalled or reported inconsistent source bytes')
        if progress:
            previous = progress[-1]
            if (typed < previous['typed'] or elapsed <= previous['elapsed_ns']
                    or any(a['updates'] < b['updates'] for a, b in zip(streams, previous['streams']))):
                raise ValueError('Stream or composer observations regressed')
        if second >= 2 and typed < (second - 1) * 10:
            raise ValueError('Typing was not concurrent with streaming')
        progress.append(dict(second=second, elapsed_ns=elapsed, streams=streams, typed=typed))
    timings = []
    for stream in range(4):
        identity, times = take('ui-updates')
        times = list(map(natural, times))
        if natural(identity) != stream or len(times) != updates:
            raise ValueError('Missing stream updates')
        if any(b <= a for a, b in zip(times, times[1:])):
            raise ValueError('Stream update times are not strictly monotonic')
        lateness = [actual - (index + 1) * 50_000_000 for index, actual in enumerate(times)]
        if any(t < 0 for t in lateness):
            raise ValueError('Stream updated before its declared deadline')
        timings.append(dict(stream=stream, elapsed_ns=times, lateness_ns=lateness))
    elapsed, active = map(natural, take('streams-complete'))
    if not 0 < active <= 32 or elapsed < seconds * 1_000_000_000:
        raise ValueError('Invalid measured history bounds or duration')
    length, revision = map(natural, take('typed'))
    expected = ''.join('asdf'[i % 4] for i in range(keys))
    if length != keys or revision < keys or evidence.get('text') != expected:
        raise ValueError('Native composer differs from exact expected typing')
    dispatch = evidence.get('keys', [])
    if len(dispatch) != keys or [x['index'] for x in dispatch] != list(range(keys)):
        raise ValueError('Missing, reordered or duplicate dispatched keys')
    dispatch_times = [x['elapsed_ns'] for x in dispatch]
    if any(type(x) is not int or x < 0 for x in dispatch_times) or any(
            b <= a for a, b in zip(dispatch_times, dispatch_times[1:])):
        raise ValueError('Invalid key dispatch times')
    key_lateness = [actual - (50_000_000 + i * 100_000_000)
                    for i, actual in enumerate(dispatch_times)]
    if any(x < 0 for x in key_lateness):
        raise ValueError('Keys were dispatched before the scheduled deadline')
    interval = read_interval(take, 'streams', wall_clock=False, before_capture=natural(captured))
    native_rows = evidence.get('rows', [])
    if len(native_rows) != 4:
        raise ValueError('Missing final native source verification')
    for stream in range(4):
        if take('verify-row') != [str(stream), str(updates)]:
            raise ValueError('Reordered final-row verification')
        expected_hash = hashlib.sha256(final_row(stream, updates).encode()).hexdigest()
        if native_rows[stream] != dict(stream=stream, sha256=expected_hash):
            raise ValueError('Final native history block differs from canonical content')
    cleanup = take('cleanup')
    retired(cleanup)
    if natural(take('complete')) != updates * 4 * 128 or rows:
        raise ValueError('Incomplete or extra streaming evidence')
    return dict(progress=progress, streams=timings, elapsed_ns=elapsed, peak_active_rows=active,
                key_lateness_ns=key_lateness, interval=interval, cleanup=fields(cleanup),
                total_source_bytes=updates * 4 * 128)


def budget_failures(workload, peak_rss):
    failures = []
    for name, p95, p99 in [('draw', 16_700_000, 33_400_000), ('input_to_frame', 50_000_000, 100_000_000)]:
        histogram = workload['interval']['histograms'][name]
        if histogram['count'] < 1000:
            failures.append(f'Fewer than 1000 {name} samples')
        if histogram['p95'] is None or histogram['p95'] > p95:
            failures.append(f'{name} p95 exceeds {p95} ns')
        if histogram['p99'] is None or histogram['p99'] > p99:
            failures.append(f'{name} p99 exceeds {p99} ns')
    if peak_rss > 1024**3:
        failures.append('Peak RSS exceeds 1 GiB')
    if workload['interval']['dropped_inputs']:
        failures.append('Native input timestamps were dropped')
    if max(t for stream in workload['streams'] for t in stream['lateness_ns']) > 100_000_000:
        failures.append('A UI update missed its nominal deadline by more than two periods (100 ms)')
    if max(workload['key_lateness_ns']) > 50_000_000:
        failures.append('A key missed its nominal deadline by more than half a period (50 ms)')
    return failures


class Driver:
    def __init__(self, log, evidence, *, foreground=False):
        self.log, self.evidence = log, evidence
        self.foreground = foreground
        self.offset, self.pending = 0, b''
        self.mac, self.node, self.sources, self.original = None, None, None, None
        self.started, self.count, self.sent = None, 0, 0

    def close(self):
        try:
            if self.sources is not None and self.original is not None:
                self.sources.checked(self.sources.select, self.original)
                if self.sources.selected() != self.original:
                    raise RuntimeError('Original input source did not restore')
                self.evidence['input_source_restored'] = True
        finally:
            if self.sources is not None:
                self.sources.close()
            if self.node:
                self.mac.release(self.node)
            if self.mac:
                self.mac.release(self.mac.app)

    def ready(self, child, count):
        from test_agent_chat import Mac
        from mac_input_source import Sources
        self.mac = Mac(child.pid, child)
        self.mac.set(self.mac.app, 'AXFrontmost', self.mac.true)
        self.node = self.mac.wait_find(TITLE, LABEL, 'AXTextArea')
        self.mac.set(self.node, 'AXFocused', self.mac.true)
        self.sources = Sources(self.mac)
        self.original = self.sources.selected()
        self.evidence['original_input_source'] = self.original
        # Never enable/disable a layout. Record recovery before selecting an
        # already-enabled ASCII layout, then restore on success or failure.
        (self.log.parent / 'input-source-recovery.json').write_text(
            json.dumps({'selected': self.original}) + '\n')
        target = next((name for name in ('com.apple.keylayout.ABC', 'com.apple.keylayout.US')
                       if self.sources.enabled().get(name)), None)
        if target is None:
            raise RuntimeError('An already-enabled ABC/US layout is needed for physical key-code checks')
        self.sources.checked(self.sources.select, target)
        if self.sources.selected() != target:
            raise RuntimeError('Test input source did not become selected')
        self.evidence['test_input_source'] = target
        if self.mac.text(self.node, 'AXValue') != '':
            raise RuntimeError('Composer is not initially empty')
        if self.foreground:
            from mac_input_source import foreground_keys
            window = self.mac.window(TITLE)
            if not window:
                raise RuntimeError('Typing window disappeared before foreground qualification')
            try:
                self.mac.set(self.mac.app, 'AXFrontmost', self.mac.true)
                self.mac.perform(window, 'AXRaise')
                self.mac.set(self.node, 'AXFocused', self.mac.true)
            finally:
                self.mac.release(window)
            foreground_keys(self.mac)
            self.evidence['key_route'] = 'OS event route with per-event foreground ownership check'
        self.count = count
        self.started = time.monotonic_ns()
        child.stdin.write(b'go\n')
        child.stdin.flush()

    def type_due(self):
        if self.started is None:
            return
        while self.sent < self.count:
            elapsed = time.monotonic_ns() - self.started
            if elapsed < 50_000_000 + self.sent * 100_000_000:
                break
            self.mac.key((0, 1, 2, 3)[self.sent % 4])
            self.evidence['keys'].append(dict(index=self.sent, elapsed_ns=elapsed))
            self.sent += 1

    def __call__(self, child):
        self.type_due()
        with self.log.open('rb') as stream:
            stream.seek(self.offset)
            data = stream.read(1024 * 1024)
            self.offset += len(data)
        lines = (self.pending + data).split(b'\n')
        self.pending = lines.pop()
        from presentation_report import check_partial_line
        check_partial_line(self.pending)
        for line in lines:
            if not line.startswith(PREFIX.encode()):
                continue
            kind, payload = line[len(PREFIX):].decode().split(' ', 1)
            if kind == 'ready':
                if self.started is not None:
                    raise ValueError('Repeated ready handshake')
                self.ready(child, natural(sexp(payload)))
            elif kind == 'streams-complete':
                self.type_due()
                if self.sent != self.count:
                    raise RuntimeError('Streams finished before all paced keys were sent')
                expected = ''.join('asdf'[i % 4] for i in range(self.count))
                deadline = time.monotonic() + 10
                while self.mac.text(self.node, 'AXValue') != expected:
                    if time.monotonic() > deadline:
                        raise RuntimeError('Native composer did not retain the exact dispatched keys')
                    time.sleep(.01)
                self.evidence['text'] = expected
                child.stdin.write(b'typed\n')
                child.stdin.flush()
            elif kind == 'verify-row':
                identity, updates = map(natural, sexp(payload))
                expected = final_row(identity, updates)
                node = self.mac.wait_find(TITLE, expected, 'AXStaticText')
                try:
                    values, children = self.mac.node_values(node)
                    for item in children:
                        self.mac.release(item)
                    if expected not in values:
                        raise RuntimeError('Final row source was not installed natively')
                finally:
                    self.mac.release(node)
                self.evidence['rows'].append(dict(stream=identity, sha256=hashlib.sha256(expected.encode()).hexdigest()))
                child.stdin.write(f'verified {identity}\n'.encode())
                child.stdin.flush()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--executable', type=Path, default=Path('_build/default/examples/performance_streaming/main.exe'))
    parser.add_argument('--build-profile', choices=('dev', 'release'), required=True)
    parser.add_argument('--smoke', action='store_true')
    parser.add_argument('--check-budgets', action='store_true')
    parser.add_argument('--timeout', type=float, default=300)
    parser.add_argument('--presentation', action='store_true',
                        help='Require paired native Metal reports; select the presentation-enabled executable')
    args = parser.parse_args()
    import presentation_report

    if platform.system() != 'Darwin':
        parser.error('Native typing qualification currently requires macOS')
    if not math.isfinite(args.timeout) or not 0 < args.timeout <= 3600:
        parser.error('Timeout must be finite and in (0,3600]')
    if args.check_budgets and (args.smoke or args.build_profile != 'release'):
        parser.error('Budget checks require a full optimized workload')
    args.output.mkdir(parents=True, exist_ok=False)
    log = args.output / 'application.log'
    report = dict(complete=False, smoke=args.smoke, build_profile=args.build_profile,
                  evidence=dict(keys=[], rows=[]), platform=platform.platform(), architecture=platform.machine(),
                  measurement='Native submitted frames and input dispatch; not physical presentation or hardware latency')
    driver = Driver(log, report['evidence'], foreground=args.presentation)
    def interrupted(signum, _frame):
        raise SystemExit(128 + signum)
    handlers = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    try:
        with args.executable.open('rb') as binary:
            report['executable_sha256'] = hashlib.file_digest(binary, 'sha256').hexdigest()
        report.update(revision=command('git', 'rev-parse', 'HEAD'), dirty=bool(command('git', 'status', '--porcelain')),
                      hardware=command('sysctl', '-n', 'machdep.cpu.brand_string'), memory_bytes=int(command('sysctl', '-n', 'hw.memsize')),
                      display=command('system_profiler', 'SPDisplaysDataType', '-json'), power=command('pmset', '-g', 'batt'), thermal=command('pmset', '-g', 'therm'))
        try:
            collect(args.executable, log, report, args.timeout, arguments=['--smoke'] if args.smoke else [],
                    on_poll=driver, poll_interval=.005)
        finally:
            driver.close()
        report['workload'] = validate(log.read_text(), report['evidence'], smoke=args.smoke)
        report['presentation'] = presentation_report.validate(
            log.read_text(), [('streaming', report['workload']['interval'])], enabled=args.presentation)
        if args.presentation:
            report['measurement'] = 'Native CPU and paired Metal host-clock presentation; excludes hardware/photon latency'
        if args.check_budgets:
            report['budget_failures'] = budget_failures(report['workload'], report['peak_rss_bytes'])
            if args.presentation:
                report['budget_failures'] += presentation_report.budget_failures(
                    report['presentation'], require_input=True)
            if report['budget_failures']:
                raise RuntimeError('; '.join(report['budget_failures']))
        report['complete'] = True
    except BaseException as error:
        report['failure'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
    print(f'STREAMING_TYPING_MEASUREMENT_OK report={args.output / "report.json"}', flush=True)


if __name__ == '__main__':
    main()
