#!/usr/bin/env python3
"""Qualify settled focused/unfocused idle using native counters and native visibility."""
import argparse
import ctypes as C
import hashlib
import json
import math
from pathlib import Path
import platform
import signal
import subprocess
import time

from measure_chart_stream import collect, command
from measure_list_history import natural, read_interval, sexp
from measure_resource_lifecycle import retired

TITLE = 'GPUIO · Idle qualification'
PREFIX = 'GPUIO_IDLE_PERF '


def validate(output, evidence, *, smoke):
    rows = [line[len(PREFIX):].split(' ', 1) for line in output.splitlines() if line.startswith(PREFIX)]
    def take(name):
        if not rows or rows[0][0] != name:
            raise ValueError(f'Missing idle record {name}')
        return sexp(rows.pop(0)[1])
    seconds = 2 if smoke else 60
    if take('config') != [str(smoke).lower(), str(seconds)]:
        raise ValueError('Idle configuration mismatch')
    result = {}
    for phase, expected in [('focused', 'true'), ('unfocused', 'false')]:
        if take('ready') != phase:
            raise ValueError('Reordered idle readiness')
        name, before_capture = take('begin')
        if name != phase or take('state-before') != [phase, [expected]]:
            raise ValueError('Wrong initial activation state')
        finished = take('finish')
        observed_phase, native = take('observations')
        if observed_phase != phase or not (seconds - 1 <= len(native) < 128):
            raise ValueError('Incomplete native visibility sampling')
        if any(active != expected or visible != ['true'] for _, active, visible in native):
            raise ValueError('Native window became inactive or occluded, or visibility is unavailable')
        native_times = [natural(t)/1e9 for t, _, _ in native]
        if (native_times[0] > 1.5 or native_times[-1] < seconds - 1.5
                or any(not 0 < b-a <= 1.5 for a,b in zip(native_times,native_times[1:]))):
            raise ValueError('Native observations did not cover the idle interval')
        interval = read_interval(lambda name: finished if name == 'finish' else take(name), phase,
                                 wall_clock=False, before_capture=natural(before_capture))
        interval['native_observations'] = native
        if (interval['elapsed_ns'] < seconds*1_000_000_000 or interval['dropped_inputs']
                or any(h['count'] for h in interval['histograms'].values())):
            raise ValueError('Idle interval was too short or contained native work')
        if take('state-after') != [phase, [expected], '0']:
            raise ValueError('Native activation changed during idle')
        observations = evidence.get(phase, [])
        if len(observations) < seconds - 1 or not observations:
            raise ValueError('Insufficient independent activation observations')
        if any(o['frontmost'] != (expected == 'true') for o in observations):
            raise ValueError('Wrong foreground state')
        times = [o['elapsed_seconds'] for o in observations]
        if any(not math.isfinite(t) or t < 0 for t in times) or any(b <= a for a, b in zip(times, times[1:])):
            raise ValueError('Nonmonotonic activation observations')
        if times[0] > 1.5 or times[-1] < seconds - 1.5 or any(b-a > 1.5 for a,b in zip(times,times[1:])):
            raise ValueError('Activation observations did not cover the measured idle interval')
        result[phase] = interval
    retired(take('cleanup'))
    if take('complete') != '2' or rows or not evidence.get('editor_focused_then_blurred'):
        raise ValueError('Incomplete idle/blur evidence')
    return result


class Driver:
    def __init__(self, log, evidence):
        self.log, self.evidence = log, evidence
        self.offset, self.pending = 0, b''
        self.mac = self.finder = None
        self.phase = None
        self.started = 0

    def boolean(self, node, name):
        value = self.mac.attr(node, name)
        if not value:
            raise RuntimeError(f'Missing {name}')
        get = self.mac.cf.CFBooleanGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p]
        try:
            return bool(get(value))
        finally:
            self.mac.release(value)

    def wait(self, predicate):
        deadline = time.monotonic() + 8
        while not predicate():
            if time.monotonic() > deadline:
                raise RuntimeError('Native focus did not settle')
            time.sleep(.05)

    def ready(self, child, phase):
        from test_agent_chat import Mac
        if self.mac is None:
            self.mac = Mac(child.pid, child)
        if phase == 'focused':
            self.mac.set(self.mac.app, 'AXFrontmost', self.mac.true)
            editor = self.mac.wait_find(TITLE, 'Idle editor', 'AXTextField')
            button = None
            try:
                self.mac.set(editor, 'AXFocused', self.mac.true)
                self.wait(lambda: self.boolean(editor, 'AXFocused'))
                time.sleep(.7)
                button = self.mac.wait_find(TITLE, 'Settle without a caret', 'AXButton')
                self.mac.set(button, 'AXFocused', self.mac.true)
                self.wait(lambda: self.boolean(button, 'AXFocused') and not self.boolean(editor, 'AXFocused'))
                self.evidence['editor_focused_then_blurred'] = True
            finally:
                self.mac.release(editor)
                if button:
                    self.mac.release(button)
        elif phase == 'unfocused':
            import os
            pids = subprocess.check_output(['/usr/bin/pgrep', '-u', str(os.getuid()), '-x', 'Finder'], text=True, timeout=3).split()
            if len(pids) != 1:
                raise RuntimeError('Expected one existing Finder for foreground handoff')
            self.finder = Mac(int(pids[0]))
            self.finder.set(self.finder.app, 'AXFrontmost', self.finder.true)
        else:
            raise ValueError('Unexpected idle phase')
        expected = phase == 'focused'
        self.wait(lambda: self.boolean(self.mac.app, 'AXFrontmost') == expected)
        child.stdin.write(f'ready {phase}\n'.encode())
        child.stdin.flush()

    def __call__(self, child):
        with self.log.open('rb') as stream:
            stream.seek(self.offset)
            data = stream.read(65536)
            self.offset += len(data)
        lines = (self.pending + data).split(b'\n')
        self.pending = lines.pop()
        if len(self.pending) > 65536:
            raise ValueError('Oversized idle record')
        for line in lines:
            if not line.startswith(PREFIX.encode()):
                continue
            name, payload = line[len(PREFIX):].decode().split(' ', 1)
            if name == 'ready':
                self.ready(child, sexp(payload))
            elif name == 'begin':
                self.phase, _ = sexp(payload)
                self.started = time.monotonic()
                self.evidence[self.phase] = []
            elif name == 'finish':
                self.phase = None
        now = time.monotonic()
        if self.phase and now - self.started >= len(self.evidence[self.phase]):
            self.evidence[self.phase].append(dict(elapsed_seconds=now-self.started,
                frontmost=self.boolean(self.mac.app, 'AXFrontmost')))

    def close(self):
        for mac in (self.mac, self.finder):
            if mac:
                mac.release(mac.app)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--executable', type=Path, default=Path('_build/default/examples/performance_idle/main.exe'))
    parser.add_argument('--smoke', action='store_true')
    parser.add_argument('--build-profile', choices=('dev', 'release'), required=True)
    parser.add_argument('--timeout', type=float, default=180)
    args = parser.parse_args()
    if platform.system() != 'Darwin' or not math.isfinite(args.timeout) or not 0 < args.timeout <= 600:
        parser.error('Requires macOS and a finite timeout in (0,600]')
    if not args.smoke and args.build_profile != 'release':
        parser.error('Full idle qualification requires an optimized build')
    args.output.mkdir(parents=True, exist_ok=False)
    report = dict(complete=False, smoke=args.smoke, build_profile=args.build_profile, evidence={}, platform=platform.platform(),
                  measurement='Native idle counters, activation transitions and sampled AppKit occlusion; not physical FPS')
    log = args.output / 'application.log'
    driver = Driver(log, report['evidence'])
    def interrupt(sig, _):
        raise SystemExit(128+sig)
    handlers = {sig: signal.signal(sig, interrupt) for sig in (signal.SIGINT, signal.SIGTERM)}
    try:
        report.update(revision=command('git','rev-parse','HEAD'), dirty=bool(command('git','status','--porcelain')),
                      executable_sha256=hashlib.sha256(args.executable.read_bytes()).hexdigest(),
                      hardware=command('sysctl','-n','machdep.cpu.brand_string'),
                      display=command('system_profiler','SPDisplaysDataType','-json'),
                      power=command('pmset','-g','batt'), thermal=command('pmset','-g','therm'))
        try:
            collect(args.executable, log, report, args.timeout, arguments=['--smoke'] if args.smoke else [], on_poll=driver)
        finally:
            driver.close()
        report['workload'] = validate(log.read_text(), report['evidence'], smoke=args.smoke)
        report['complete'] = True
    except BaseException as error:
        report['failure'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (args.output/'report.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
        for sig, handler in handlers.items():
            signal.signal(sig,handler)
    print(f'IDLE_MEASUREMENT_OK report={args.output / "report.json"}', flush=True)


if __name__ == '__main__':
    main()
