#!/usr/bin/env python3
"""Collect loaded-history histograms and idle checks, retaining failures and raw logs.

Native draw/submission timings are not physical presentation or GPU execution time.
The optional budget check covers this workload only, not release qualification.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import re
import signal

from measure_chart_stream import collect, command

METRICS = ('draw', 'dirty_to_submission', 'animation_submission_interval',
           'input_to_frame', 'inputs_per_frame')
ZERO_RESOURCES = ('windows', 'queued_jobs', 'queued_commands', 'pending_requests',
                  'assets', 'asset_uploads', 'asset_source_bytes', 'documents',
                  'document_source_bytes', 'charts', 'chart_data_bytes',
                  'canvases', 'canvas_scene_bytes')


def sexp(text):
    """The probe's bounded numeric/symbol grammar, deliberately excluding strings."""
    if len(text) > 65536 or re.search(r'[^a-zA-Z0-9_().\s-]', text):
        raise ValueError('Invalid probe record grammar or length')
    stack, roots = [], []
    for token in re.findall(r'\(|\)|[^\s()]+', text):
        if token == '(':
            value = []
            (stack[-1] if stack else roots).append(value)
            stack.append(value)
            if len(stack) > 16:
                raise ValueError('Probe nesting limit exceeded')
        elif token == ')':
            if not stack:
                raise ValueError('Unexpected closing parenthesis')
            stack.pop()
        else:
            (stack[-1] if stack else roots).append(token)
    if stack or len(roots) != 1:
        raise ValueError('Truncated or multiple probe records')
    return roots[0]


def natural(value):
    if not isinstance(value, str) or not re.fullmatch(r'0|[1-9][0-9]*', value):
        raise ValueError(f'Expected a nonnegative integer: {value!r}')
    return int(value)


def fields(value):
    if not isinstance(value, list) or any(not isinstance(p, list) or len(p) != 2 for p in value):
        raise ValueError('Expected record fields')
    result = dict(value)
    if len(result) != len(value):
        raise ValueError('Duplicate record field')
    return result


def percentile(buckets, percent):
    count = sum(n for _, n in buckets)
    if not count:
        return None
    rank, seen = (count * percent + 99) // 100, 0
    for value, n in buckets:
        seen += n
        if seen >= rank:
            return value


def read_interval(take, phase, *, wall_clock, before_capture=0):
    if wall_clock:
        name, elapsed = take('wall-finish')
        if name != phase:
            raise ValueError('Wrong wall-clock finish phase')
        interval = {'elapsed_ns': natural(elapsed)}
        if interval['elapsed_ns'] == 0:
            raise ValueError('Empty wall-clock interval')
    else:
        name, event = take('finish')
        if name != phase or event[0] != 'Finished':
            raise ValueError('Wrong finish phase/event')
        summary = fields(event[1:])
        if set(summary) != {'elapsed_ns', 'capture_ns', 'dropped_inputs', 'counts'}:
            raise ValueError('Unexpected interval fields')
        counts = list(map(natural, summary['counts']))
        if len(counts) != 5:
            raise ValueError('Expected five histogram counts')
        interval = {k: natural(summary[k]) for k in ('elapsed_ns', 'capture_ns', 'dropped_inputs')}
        interval['begin_capture_ns'] = before_capture
        distributions = {}
        for metric, expected in enumerate(counts):
            buckets, total = [], None
            while total is None or len(buckets) < total:
                name, page = take('buckets')
                if name != phase or page[0] != 'Buckets':
                    raise ValueError('Wrong bucket phase/event')
                page = fields(page[1:])
                if set(page) != {'metric', 'offset', 'total', 'values'}:
                    raise ValueError('Unexpected bucket fields')
                if natural(page['metric']) != metric or natural(page['offset']) != len(buckets):
                    raise ValueError('Missing, reordered or duplicate bucket page')
                size = natural(page['total'])
                if total is not None and size != total:
                    raise ValueError('Bucket total changed')
                total = size
                values = [list(map(natural, pair)) for pair in page['values']]
                if not 0 <= total <= 100000 or len(values) > 128 or (not values and total):
                    raise ValueError('Invalid bucket page size')
                for pair in values:
                    if len(pair) != 2 or pair[1] == 0 or (buckets and pair[0] <= buckets[-1][0]):
                        raise ValueError('Invalid or unordered histogram bucket')
                    buckets.append(pair)
                if len(buckets) > total:
                    raise ValueError('Too many buckets')
            if sum(n for _, n in buckets) != expected:
                raise ValueError('Bucket counts differ from snapshot')
            distributions[METRICS[metric]] = dict(count=expected, buckets=buckets,
                                                  p95=percentile(buckets, 95),
                                                  p99=percentile(buckets, 99))
        interval['histograms'] = distributions
    return interval


def validate(output, *, smoke, background, wall_clock=False):
    records = []
    for line in output.splitlines():
        if line.startswith('GPUIO_PERF '):
            name, payload = line[len('GPUIO_PERF '):].split(' ', 1)
            records.append((name, sexp(payload)))
    position = 0

    def take(name):
        nonlocal position
        if position >= len(records) or records[position][0] != name:
            raise ValueError(f'Expected {name} at record {position}')
        value = records[position][1]
        position += 1
        return value

    rows, idle = (96, 2) if smoke else (10000, 60)
    if take('config') != [str(rows), str(smoke).lower(), str(background).lower(), str(idle)]:
        raise ValueError('Workload configuration mismatch')
    result = {}
    for phase in ('history', 'idle'):
        before_capture = 0
        if wall_clock:
            if take('wall-begin') != phase:
                raise ValueError('Wrong wall-clock begin phase')
        else:
            name, capture = take('begin')
            if name != phase:
                raise ValueError('Wrong begin phase')
            before_capture = natural(capture)
        if phase == 'history':
            before = fields(take('growth-start'))
            after = fields(take('growth-complete'))
            if (natural(before['visible_last']) <= 1 or after['visible_first'] != '0'
                    or after['visible_last'] != '1' or after['anchor'] != [['0', '0']]):
                raise ValueError('Growth failed to preserve the first-row anchor')
        interval = read_interval(take, phase, wall_clock=wall_clock, before_capture=before_capture)
        result[phase] = interval
        if phase == 'history':
            forward, backward, active = map(natural, take('history'))
            if forward != rows or backward != rows or not 0 < active <= 32:
                raise ValueError('Incomplete traversal or active-row budget exceeded')
            result['coverage'] = dict(forward=forward, backward=backward, peak_active=active)
        elif (not wall_clock and any(h['count'] for h in interval['histograms'].values())) or interval['elapsed_ns'] < idle * 1_000_000_000:
            raise ValueError('Idle interval was too short or contained native work')
    cleanup = fields(take('cleanup'))
    if any(natural(cleanup[k]) != 0 for k in ZERO_RESOURCES):
        raise ValueError('Owned resources or queued work remained after window close')
    queue = fields(cleanup['native_command_queue'])
    if natural(queue['commands']) or natural(queue['bytes']):
        raise ValueError('Native command queue was not drained')
    result['cleanup'] = cleanup
    if natural(take('complete')) != rows or position != len(records):
        raise ValueError('Missing/duplicate completion or extra records')
    return result


def check_budgets(workload, peak_rss):
    draw = workload['history']['histograms']['draw']
    failures = []
    if draw['count'] < 1000:
        failures.append('Fewer than 1000 draw samples')
    if draw['p95'] is None or draw['p95'] > 16_700_000:
        failures.append('Draw p95 exceeds 16.7 ms')
    if draw['p99'] is None or draw['p99'] > 33_400_000:
        failures.append('Draw p99 exceeds 33.4 ms')
    if peak_rss > 1024**3:
        failures.append('Peak RSS exceeds 1 GiB')
    if workload['history']['dropped_inputs'] or workload['idle']['dropped_inputs']:
        failures.append('Native input timestamps were dropped')
    return failures


def main(*, validate_workload=validate, default_executable=Path('_build/default/examples/performance/main.exe'),
         success_marker='LIST_HISTORY_MEASUREMENT_OK'):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--executable', type=Path, default=default_executable)
    parser.add_argument('--build-profile', choices=('dev', 'release'), required=True,
                        help='Record the profile used by the preceding Dune build; not inferred from its filename')
    parser.add_argument('--timeout', type=float, default=900)
    parser.add_argument('--smoke', action='store_true')
    parser.add_argument('--background', action='store_true')
    parser.add_argument('--check-budgets', action='store_true')
    parser.add_argument('--wall-clock', action='store_true',
                        help='CPU/elapsed/RSS comparison only; no histograms or zero-redraw acceptance')
    args = parser.parse_args()
    if not math.isfinite(args.timeout) or not 0 < args.timeout <= 3600:
        parser.error('Timeout must be finite and in (0, 3600] seconds')
    if args.check_budgets and (args.smoke or args.wall_clock or args.build_profile != 'release'):
        parser.error('Budget checks require a full release-profile workload with native histograms')
    args.output.mkdir(parents=True, exist_ok=False)
    report = dict(complete=False, platform=platform.platform(), architecture=platform.machine(),
                  build_profile=args.build_profile, smoke=args.smoke, background=args.background, wall_clock=args.wall_clock,
                  measurement=('Monotonic phase time and process resources; no frame/idle-draw acceptance' if args.wall_clock
                               else 'Native histogram deltas; no physical presentation/GPU-time claim'))
    log = args.output / 'application.log'

    def interrupted(signum, _frame):
        raise SystemExit(128 + signum)

    handlers = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGINT, signal.SIGTERM)}
    try:
        with args.executable.open('rb') as binary:
            executable_hash = hashlib.file_digest(binary, 'sha256').hexdigest()
        report.update(revision=command('git', 'rev-parse', 'HEAD'),
                      dirty=bool(command('git', 'status', '--porcelain')),
                      executable_sha256=executable_hash)
        if platform.system() == 'Darwin':
            report.update(hardware=command('sysctl', '-n', 'machdep.cpu.brand_string'),
                          memory_bytes=int(command('sysctl', '-n', 'hw.memsize')),
                          display=command('system_profiler', 'SPDisplaysDataType', '-json'),
                          power=command('pmset', '-g', 'batt'), thermal=command('pmset', '-g', 'therm'))
        arguments = [flag for enabled, flag in ((args.smoke, '--smoke'), (args.background, '--background'), (args.wall_clock, '--wall-clock')) if enabled]
        collect(args.executable, log, report, args.timeout, arguments=arguments)
        report['workload'] = validate_workload(log.read_text(), smoke=args.smoke, background=args.background, wall_clock=args.wall_clock)
        if args.check_budgets:
            report['budget_failures'] = check_budgets(report['workload'], report['peak_rss_bytes'])
            if report['budget_failures']:
                raise RuntimeError('; '.join(report['budget_failures']))
        report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        (args.output / 'report.json').write_text(json.dumps(report, indent=2, allow_nan=False) + '\n')
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
    print(f'{success_marker} report={args.output / "report.json"}', flush=True)


if __name__ == '__main__':
    main()
