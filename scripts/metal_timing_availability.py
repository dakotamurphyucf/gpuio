"""Classify bounded Metal probes without turning unavailable clocks into a pass.

The preview exception is deliberately narrow: every expected callback arrived,
all presentation timestamps are zero, and ownership/cleanup checks passed. Mixed
results, malformed reports, dropped callbacks and process failures stay failures.
This does not change physical workload budgets or their report validator.
"""
import argparse
import json
import math
from pathlib import Path
import subprocess


def require(condition, message):
    if not condition:
        raise ValueError(message)


def natural(value):
    require(type(value) is int and value >= 0, 'Invalid nonnegative count')
    return value


def clock(value):
    require(type(value) in (float, int) and math.isfinite(value) and value >= 0,
            'Invalid clock sample')
    return value


def load_report(path):
    def record(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, 'Duplicate report field')
            result[key] = value
        return result

    def invalid_constant(value):
        raise ValueError(f'Non-finite JSON constant: {value}')

    return json.loads(path.read_text(), object_pairs_hook=record, parse_constant=invalid_constant)


def classify_hook(data):
    require(natural(data['schema']) == 1 and data['kind'] == 'gpui_metal_hook_qualification',
            'Unknown hook report')
    for field in ('visible_at_start', 'visible_at_end', 'finished', 'distinct'):
        require(data[field] is True, f'Hook did not complete {field}')
    require(natural(data['frames_per_window']) == 90, 'Unexpected hook workload')
    require(natural(data['idle_before_frames_ms']) <= 5000, 'Invalid idle setup')
    sessions = data['sessions']
    require(len(sessions) == 2, 'Missing independent windows')
    require(len({s['session'] for s in sessions}) == 2
            and len({s['window'] for s in sessions}) == 2, 'Reused session/window')
    zero_only = True
    presented_only = True
    for session in sessions:
        require(natural(session['session']) > 0 and type(session['window']) is str
                and bool(session['window']), 'Invalid session/window identity')
        require(session['window_closed'] is True and session['accepting'] is False
                and natural(session['pending']) == 0, 'Hook ownership did not retire')
        counts = session['counts']
        expected_counts = {'attempted', 'admitted', 'saturated', 'presented',
                           'not_submitted', 'missing', 'zero', 'invalid_clock',
                           'duplicate_callbacks', 'trace_truncated', 'histogram_overflow'}
        require(set(counts) == expected_counts, 'Unexpected hook counters')
        for value in counts.values():
            natural(value)
        require(all(counts[k] == 0 for k in expected_counts
                    - {'attempted', 'admitted', 'presented', 'zero'}),
                'Hook lost or invalidated a callback')
        count = counts['attempted']
        require(90 <= count <= 4096 and counts['admitted'] == count
                and counts['presented'] + counts['zero'] == count,
                'Incomplete hook accounting')
        require(natural(session['submission_samples']) == counts['presented']
                and natural(session['input_samples']) == 0
                and natural(session['animation_samples']) <= max(0, counts['presented'] - 1),
                'Hook histogram accounting mismatch')
        records = session['records']
        require(len(records) == count, 'Missing hook records')
        require([natural(r['sequence']) for r in records] == list(range(count)),
                'Missing or reordered hook sequence')
        require(len({natural(r['drawable']) for r in records}) == count,
                'Reused drawable identity')
        zeros = 0
        for row in records:
            require(natural(row['inputs']) == 0, 'Unexpected input in hook probe')
            require(all(type(row[k]) is bool for k in ('new_scene', 'active', 'animating')),
                    'Invalid hook flags')
            submit = clock(row['submit_host_s'])
            callback = clock(row['callback_host_s'])
            presented = clock(row['presented_host_s'])
            require(submit > 0 and callback >= submit, 'Invalid callback clock order')
            if row['outcome'] == 'Zero':
                require(presented == 0 and row['submission_lower_ns'] is None
                        and row['submission_upper_ns'] is None, 'Invalid zero-time record')
                zeros += 1
            else:
                require(row['outcome'] == 'Presented' and submit <= presented <= callback,
                        'Invalid presentation record')
                require(natural(row['submission_lower_ns']) <= natural(row['submission_upper_ns']),
                        'Invalid latency interval')
        require(zeros == counts['zero'], 'Hook trace/counter mismatch')
        zero_only = zero_only and zeros == count
        presented_only = presented_only and zeros == 0
    animation = any(s['animation_samples'] >= 45 for s in sessions)
    require(data['animation_samples'] is animation, 'Animation summary mismatch')
    if presented_only:
        require(animation and data['passed'] is True, 'Positive timing qualification failed')
        return 'passed'
    require(zero_only and data['passed'] is False, 'Not a wholly unavailable presentation clock')
    return 'unavailable'


def classify_api(data, exit_code):
    require(natural(data['schema']) == 2 and data['kind'] == 'metal_api_qualification',
            'Unknown API report or missing cleanup evidence')
    require(type(exit_code) is int and exit_code in (0, 1), 'API probe crashed or exited unexpectedly')
    require(set(data['cleanup']) == {'window_closed', 'timer_stopped', 'layer_released', 'queue_released'}
            and all(v is True for v in data['cleanup'].values()), 'API cleanup failed')
    require(natural(data['requested_frames']) == 120 and natural(data['submitted_frames']) == 120,
            'Incomplete API workload')
    rows = data['frames']
    require(len(rows) == 120 and [natural(r['sequence']) for r in rows] == list(range(120)),
            'Missing or reordered API records')
    require(len({natural(r['drawable_id']) for r in rows}) == 120, 'Reused drawable identity')
    times = []
    for row in rows:
        require(row['visible'] is True and row['active'] is True, 'API window not active/visible')
        before = clock(row['submit_before_s'])
        after = clock(row['submit_after_s'])
        callback = clock(row['presentation_callback_host_s'])
        gpu_start = clock(row['gpu_start_s'])
        gpu_end = clock(row['gpu_end_s'])
        completion = clock(row['completion_callback_host_s'])
        presented = clock(row['presented_s'])
        require(clock(row['before_presented_s']) == 0, 'Drawable already presented')
        require(before > 0 and after >= before and callback >= before,
                'Invalid API host clock order')
        require(before <= gpu_start <= gpu_end <= completion
                and natural(row['gpu_status']) == 4, 'GPU completion failed')
        require(presented == 0 or before <= presented <= callback, 'Invalid presentation clock order')
        times.append(presented)
    if all(t > 0 for t in times):
        require(all(a < b for a, b in zip(times, times[1:])), 'Unordered presentations')
        require(exit_code == 0 and data['complete'] is True and data['failures'] == [],
                'Positive API qualification failed')
        return 'passed'
    require(all(t == 0 for t in times), 'Not a wholly unavailable API presentation clock')
    expected = [f'Invalid presentation clock ordering at {i}' for i in range(120)]
    expected.append('Presented timestamps are not strictly increasing in submitted order')
    require(exit_code == 1 and data['complete'] is False and data['failures'] == expected,
            'API reported failures beyond unavailable presentation timestamps')
    return 'unavailable'


def combined_status(hook, api):
    require(hook in ('passed', 'unavailable') and api in ('passed', 'unavailable'),
            'Unknown probe classification')
    require(not (hook == 'unavailable' and api == 'passed'),
            'GPUI has no presentation timestamps although the independent Metal API probe passed')
    return 'passed' if hook == api == 'passed' else 'unavailable'


def check_pair(hook_output, api_output):
    hook_metadata = load_report(hook_output / 'availability.json')
    api_metadata = load_report(api_output / 'availability.json')
    require(natural(hook_metadata['child_exit']) == 0, 'Native hook process did not succeed')
    hook = classify_hook(load_report(hook_output / 'report.json'))
    api = classify_api(load_report(api_output / 'report.json'), api_metadata['child_exit'])
    require(hook_metadata['status'] == hook and api_metadata['status'] == api,
            'Probe classification does not match raw evidence')
    return dict(status=combined_status(hook, api), hook=hook, api=api)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--api-binary', type=Path, help='Run the standalone API probe')
    mode.add_argument('--hook-output', type=Path, help='Cross-check completed hook/API probe outputs')
    parser.add_argument('--api-output', type=Path, help='Required with --hook-output')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if bool(args.hook_output) != bool(args.api_output):
        parser.error('--hook-output and --api-output must be supplied together')
    args.output.mkdir(parents=True, exist_ok=False)
    if args.hook_output:
        classification = check_pair(args.hook_output, args.api_output)
    else:
        report = args.output / 'report.json'
        with report.open('w') as stdout, (args.output / 'stderr.log').open('w') as stderr:
            result = subprocess.run([str(args.api_binary.resolve())], stdout=stdout,
                                    stderr=stderr, timeout=25)
        status = classify_api(load_report(report), result.returncode)
        classification = dict(status=status, child_exit=result.returncode)
    classification['scope'] = 'Hosted preview timing availability; not physical workload acceptance'
    (args.output / 'availability.json').write_text(json.dumps(classification, indent=2) + '\n')
    print(json.dumps(classification))
    if classification['status'] == 'unavailable':
        print('::warning::Hosted Metal timing is not fully qualified; inspect availability.json. Physical qualification remains required.')


if __name__ == '__main__':
    main()
