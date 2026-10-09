"""Validate paired, bounded Metal presentation reports from qualification drivers.

No desktop access. CPU workload validation must succeed independently first.
Presentation timestamps are OS observations, not hardware/photon latency.
"""
import json
import math

PREFIX = 'GPUIO_PRESENTATION '
METRICS = ('submission_to_presentation', 'input_to_presentation', 'scheduled_presentation_interval')
LOSSES = ('saturated', 'not_submitted', 'missing', 'invalid_clock',
          'duplicate_callbacks', 'histogram_overflow')
COUNTS = ('attempted', 'admitted', 'presented', 'zero', 'trace_truncated', *LOSSES)
STARTUP_RESPONSE_NS = 100_000_000


def require(condition, message):
    if not condition:
        raise ValueError(message)


def natural(value):
    require(type(value) is int and 0 <= value < 2**64, 'Invalid presentation integer')
    return value


def check_partial_line(pending):
    # Full traces can exceed one driver's 1 MiB read. Keep the ordinary protocol
    # bound and admit only this explicitly identified diagnostic record up to 4 MiB.
    limit = 4 * 1024 * 1024 + len(PREFIX) if pending.startswith(PREFIX.encode()) else 65536
    require(len(pending) <= limit, 'Oversized qualification record')


def unique_object(pairs):
    result = dict(pairs)
    require(len(result) == len(pairs), 'Duplicate presentation field')
    return result


def percentile(buckets, percent):
    rank = (sum(n for _, n in buckets) * percent + 99) // 100
    seen = 0
    for value, n in buckets:
        seen += n
        if seen >= rank:
            return value
    return None


def histogram(data):
    require(isinstance(data, dict) and set(data) == {'count', 'buckets'}, 'Invalid histogram fields')
    count, buckets = natural(data['count']), data['buckets']
    require(isinstance(buckets, list) and len(buckets) <= 100000, 'Invalid histogram size')
    previous, total = -1, 0
    for pair in buckets:
        require(isinstance(pair, list) and len(pair) == 2, 'Invalid histogram bucket')
        value, size = map(natural, pair)
        require(previous < value <= 61_000_000_000 and size > 0, 'Invalid histogram ordering/range')
        previous, total = value, total + size
    require(total == count, 'Histogram count mismatch')
    return dict(count=count, buckets=buckets, p95=percentile(buckets, 95), p99=percentile(buckets, 99))


def observation(value, *, background):
    require(isinstance(value, dict) and type(value.get('active')) is bool,
            'Missing window activation observation')
    require(value.get('visible') is True, 'Window visibility unqualified')
    if not background:
        require(value['active'], 'Foreground window became inactive')


def trace_record(record, sequence):
    require(natural(record['sequence']) == sequence, 'Missing/reordered presentation trace')
    natural(record['drawable'])
    for key in ('active', 'animating', 'new_scene'):
        require(type(record[key]) is bool, 'Invalid frame metadata')
    inputs = natural(record['inputs'])
    require(record['outcome'] == 'Presented', 'Non-presented trace outcome')
    times = [record[key] for key in ('submit_host_s', 'presented_host_s', 'callback_host_s')]
    require(all(type(t) in (float, int) and math.isfinite(t) and t > 0 for t in times),
            'Invalid host-clock timestamp')
    submit, presented, callback = times
    require(submit <= presented <= callback, 'Invalid host-clock ordering')
    lower, upper = (natural(record[key]) for key in ('submission_lower_ns', 'submission_upper_ns'))
    require(lower <= (presented - submit) * 1e9 <= upper, 'Submission clock bounds mismatch')
    if inputs:
        il, iu = (natural(record[key]) for key in ('input_lower_ns', 'input_upper_ns'))
        require(il <= iu and iu >= upper, 'Invalid input clock bounds')
    else:
        require(record['input_lower_ns'] is None and record['input_upper_ns'] is None,
                'Input latency without contributing input')
    return presented, inputs > 0


def interval(data, cpu, *, background, idle):
    require(data.get('diagnostic_foreground_inputs') is None,
            'Foreground input diagnostic runs cannot qualify release measurements')
    require(data['schema'] == 1 and type(data['schema']) is int
            and data['kind'] == 'gpui_presentation_interval', 'Unknown presentation schema')
    require(natural(data['cpu_elapsed_ns']) == cpu['elapsed_ns'], 'CPU interval identity mismatch')
    require(natural(data['cpu_input_samples']) == cpu['histograms']['input_to_frame']['count'],
            'CPU input count mismatch')
    require(natural(data['cpu_draw_samples']) == cpu['histograms']['draw']['count'], 'CPU draw count mismatch')
    for key in ('start_capture_ns', 'stop_capture_ns', 'settlement_ns', 'snapshot_encode_ns'):
        natural(data[key])
    require(type(data['idle_observation_mode']) is bool, 'Invalid observation mode')
    require(data['observations_truncated'] is False, 'Window observation capacity exhausted')
    observations = data['observations']
    limit = 128 if data['idle_observation_mode'] else 2048
    require(isinstance(observations, list) and 0 < len(observations) < limit, 'Invalid window observations')
    previous = -1
    for value in observations:
        elapsed = natural(value['elapsed_ns'])
        require(elapsed > previous, 'Window observations reordered')
        observation(value, background=background)
        previous = elapsed
    observation(data['end_observation'], background=background)
    native = data['native']
    require(native.get('supported') is True, 'Native presentation collection unsupported')
    require(natural(native['session']) > 0 and isinstance(native['window'], str) and native['window'],
            'Missing native session/window identity')
    require(native['accepting'] is False and native['window_closed'] is False, 'Invalid interval lifetime')
    require(natural(native['pending']) == 0, 'Presentation callbacks did not settle')
    counts = native['counts']
    require(set(counts) == set(COUNTS), 'Unknown presentation counters')
    for value in counts.values():
        natural(value)
    require(not any(counts[key] for key in LOSSES), 'Presentation samples were lost or invalid')
    require(counts['attempted'] == counts['admitted'] == counts['presented'] + counts['zero'],
            'Presentation accounting mismatch')
    distributions = native['histograms']
    require(set(distributions) == set(METRICS), 'Unknown presentation histograms')
    distributions = {key: histogram(value) for key, value in distributions.items()}
    require(distributions['submission_to_presentation']['count'] == counts['presented'],
            'Presented histogram count mismatch')
    require(distributions['input_to_presentation']['count'] == data['cpu_input_samples'],
            'Native input pairing mismatch')
    require(distributions['scheduled_presentation_interval']['count'] <= max(0, counts['presented'] - 1),
            'Too many animation intervals')
    trace = native['trace']
    require(isinstance(trace, list) and len(trace) == min(4096, counts['admitted']), 'Invalid bounded trace length')
    require(len(trace) + counts['trace_truncated'] == counts['admitted'], 'Trace truncation accounting mismatch')
    previous, input_frames, skipped = 0, 0, 0
    initial_submit, recovery_ns = None, None
    for i, record in enumerate(trace):
        if record['outcome'] == 'Zero':
            require(previous == 0 and natural(record['inputs']) == 0,
                    'Skipped frame after presentation or containing input')
            require(natural(record['sequence']) == i, 'Missing/reordered startup trace')
            natural(record['drawable'])
            submit, callback = record['submit_host_s'], record['callback_host_s']
            require(all(type(t) in (int, float) and math.isfinite(t) and t > 0
                        for t in (submit, callback)) and callback >= submit,
                    'Invalid skipped-frame clock')
            require(type(record['presented_host_s']) in (int, float)
                    and record['presented_host_s'] == 0, 'Invalid zero-time outcome')
            require(all(record[key] is None for key in ('submission_lower_ns', 'submission_upper_ns',
                                                       'input_lower_ns', 'input_upper_ns')),
                    'Skipped frame contains latency sample')
            for key in ('active', 'animating', 'new_scene'):
                require(type(record[key]) is bool, 'Invalid skipped-frame metadata')
            initial_submit = submit if initial_submit is None else initial_submit
            skipped += 1
            continue
        timestamp, has_input = trace_record(record, i)
        require(timestamp > previous, 'Presentation timestamps not increasing')
        if previous == 0 and initial_submit is not None:
            slack = (abs(timestamp) + abs(initial_submit)) * 2**-52
            recovery_ns = math.ceil((timestamp - initial_submit + slack) * 1e9)
            require(0 <= recovery_ns <= STARTUP_RESPONSE_NS, 'Startup presentation exceeds 100 ms')
        previous, input_frames = timestamp, input_frames + has_input
    require(skipped == counts['zero'], 'Unaccounted skipped frames beyond startup prefix')
    require(not skipped or recovery_ns is not None, 'Startup never reached a presented frame')
    paired = distributions['input_to_presentation']['count']
    require(input_frames <= paired, 'Trace input accounting mismatch')
    if not counts['trace_truncated']:
        require(input_frames == paired, 'Complete trace input accounting mismatch')
    if idle:
        require(not counts['attempted'] and not any(h['count'] for h in distributions.values()),
                'Native work occurred during idle')
    result = dict(data)
    result['native'] = dict(native, histograms=distributions)
    result['startup_transition'] = dict(skipped_frames=skipped, first_presentation_ns=recovery_ns)
    return result


def validate(output, phases, *, enabled, background=False):
    """Phases are ordered (name, validated CPU interval) pairs; idle is named idle."""
    lines = [line[len(PREFIX):] for line in output.splitlines() if line.startswith(PREFIX)]
    if not enabled:
        require(not lines, 'Unexpected presentation reports in CPU-only measurement')
        return None
    require(len(lines) == len(phases), 'Missing/extra presentation interval reports')
    results, sessions, window = {}, set(), None
    for line, (phase, cpu) in zip(lines, phases):
        require(len(line) <= 4 * 1024 * 1024, 'Presentation report exceeds bounded size')
        data = json.loads(line, object_pairs_hook=unique_object)
        try:
            result = interval(data, cpu, background=background, idle=phase == 'idle')
        except (KeyError, TypeError, AttributeError) as error:
            raise ValueError('Missing/invalid presentation report fields') from error
        native = result['native']
        require(native['session'] not in sessions, 'Reused presentation session')
        require(window is None or window == native['window'], 'Workload window changed')
        sessions.add(native['session'])
        window = native['window']
        results[phase] = result
    return results


def budget_failures(results, *, require_input=False):
    failures = []
    for phase, data in results.items():
        if phase == 'idle':
            continue
        histograms = data['native']['histograms']
        submission, inputs = (histograms[key] for key in METRICS[:2])
        if submission['count'] < 1000:
            failures.append(f'{phase}: fewer than 1000 presentations')
        for metric, distribution, p95, p99 in (
            ('submission', submission, 33_400_000, 50_000_000),
            ('input', inputs, 75_000_000, 125_000_000),
        ):
            if metric == 'input' and not require_input:
                continue
            if distribution['p95'] is None or distribution['p95'] > p95 or distribution['p99'] > p99:
                failures.append(f'{phase}: {metric} presentation latency exceeds declared budget')
        if require_input and inputs['count'] < 1000:
            failures.append(f'{phase}: fewer than 1000 paired input frames')
    return failures
