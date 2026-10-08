"""A missing presentation clock never excuses incomplete work or cleanup."""
import copy
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import subprocess

from metal_timing_availability import classify_api, classify_hook, combined_status, check_pair, load_report, main


def hook(*, zero=False):
    sessions = []
    for window in (1, 2):
        count = 90
        records = [dict(sequence=i, drawable=i, new_scene=True, active=window == 2,
                        animating=True, inputs=0, outcome='Zero' if zero else 'Presented',
                        submit_host_s=10. + i, presented_host_s=0. if zero else 10.01 + i,
                        callback_host_s=10.02 + i,
                        submission_lower_ns=None if zero else 9_999_999,
                        submission_upper_ns=None if zero else 10_000_001)
                   for i in range(count)]
        sessions.append(dict(session=window, window=f'WindowId({window})', accepting=False,
                             window_closed=True, pending=0,
                             counts=dict(attempted=count, admitted=count, saturated=0,
                                         presented=0 if zero else count, not_submitted=0,
                                         missing=0, zero=count if zero else 0, invalid_clock=0,
                                         duplicate_callbacks=0, trace_truncated=0, histogram_overflow=0),
                             submission_samples=0 if zero else count, input_samples=0,
                             animation_samples=0 if zero else count - 1, records=records))
    return dict(schema=1, kind='gpui_metal_hook_qualification', passed=not zero,
                idle_before_frames_ms=0, visible_at_start=True, visible_at_end=True,
                finished=True, frames_per_window=90, distinct=True,
                animation_samples=not zero, sessions=sessions)


def api(*, zero=False):
    frames = [dict(sequence=i, drawable_id=i, active=True, visible=True,
                   before_presented_s=0., submit_before_s=10. + i, submit_after_s=10.001 + i,
                   gpu_start_s=10.002 + i, gpu_end_s=10.003 + i, gpu_status=4,
                   completion_callback_host_s=10.004 + i,
                   presented_s=0. if zero else 10.005 + i, presentation_callback_host_s=10.006 + i)
              for i in range(120)]
    failures = ([f'Invalid presentation clock ordering at {i}' for i in range(120)]
                + ['Presented timestamps are not strictly increasing in submitted order']) if zero else []
    return dict(schema=2, kind='metal_api_qualification', requested_frames=120,
                submitted_frames=120, complete=not zero, failures=failures, frames=frames,
                cleanup=dict(window_closed=True, timer_stopped=True, layer_released=True, queue_released=True))


def set_path(data, path, value):
    for field in path[:-1]:
        data = data[field]
    data[path[-1]] = value


class Availability(unittest.TestCase):
    def test_independent_positive_api_cannot_excuse_zero_gpui_hook(self):
        self.assertEqual(combined_status('passed', 'passed'), 'passed')
        self.assertEqual(combined_status('unavailable', 'unavailable'), 'unavailable')
        self.assertEqual(combined_status('passed', 'unavailable'), 'unavailable')
        with self.assertRaisesRegex(ValueError, 'independent Metal API probe passed'):
            combined_status('unavailable', 'passed')
        with tempfile.TemporaryDirectory() as temporary:
            hook_output, api_output = Path(temporary) / 'hook', Path(temporary) / 'api'
            hook_output.mkdir()
            api_output.mkdir()
            for output, data, code in ((hook_output, hook(zero=True), 0),
                                       (api_output, api(zero=True), 1)):
                (output / 'report.json').write_text(json.dumps(data))
                (output / 'availability.json').write_text(json.dumps(dict(status='unavailable', child_exit=code)))
            self.assertEqual(check_pair(hook_output, api_output)['status'], 'unavailable')
            # Do not trust a successful earlier classifier stamp after raw data drift.
            data = hook(zero=True)
            data['sessions'][0]['pending'] = 1
            (hook_output / 'report.json').write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                check_pair(hook_output, api_output)

    def test_positive_and_wholly_zero_are_different_outcomes(self):
        self.assertEqual(classify_hook(hook()), 'passed')
        self.assertEqual(classify_hook(hook(zero=True)), 'unavailable')
        self.assertEqual(classify_api(api(), 0), 'passed')
        self.assertEqual(classify_api(api(zero=True), 1), 'unavailable')

    def test_hook_failure_sensitivity(self):
        mutations = [
            (['passed'], True), (['finished'], False), (['visible_at_start'], False),
            (['visible_at_end'], False), (['distinct'], False), (['animation_samples'], True),
            (['frames_per_window'], 89), (['sessions', 1, 'session'], 1),
            (['sessions', 1, 'window'], 'WindowId(1)'),
            (['sessions', 0, 'window_closed'], False), (['sessions', 0, 'accepting'], True),
            (['sessions', 0, 'pending'], 1), (['sessions', 0, 'submission_samples'], 1),
            (['sessions', 0, 'input_samples'], 1), (['sessions', 0, 'animation_samples'], 1),
            (['sessions', 0, 'records', 1, 'sequence'], 0),
            (['sessions', 0, 'records', 1, 'drawable'], 0),
            (['sessions', 0, 'records', 0, 'inputs'], 1),
            (['sessions', 0, 'records', 0, 'outcome'], 'Missing'),
            (['sessions', 0, 'records', 0, 'callback_host_s'], 9.),
            (['sessions', 0, 'records', 0, 'presented_host_s'], 0.01),
            (['sessions', 0, 'records', 0, 'submission_lower_ns'], 0),
            (['sessions', 0, 'records', 0, 'active'], 1),
        ]
        for field in ('saturated', 'missing', 'not_submitted', 'invalid_clock',
                      'duplicate_callbacks', 'trace_truncated', 'histogram_overflow'):
            mutations.append((['sessions', 0, 'counts', field], 1))
        for field in ('attempted', 'admitted', 'zero'):
            mutations.append((['sessions', 0, 'counts', field], 89))
        for path, value in mutations:
            with self.subTest(path=path):
                data = hook(zero=True)
                set_path(data, path, value)
                with self.assertRaises(ValueError):
                    classify_hook(data)

    def test_mixed_timing_is_not_the_preview_exception(self):
        data = hook(zero=True)
        data['sessions'][1] = hook()['sessions'][1]
        data['animation_samples'] = True
        with self.assertRaises(ValueError):
            classify_hook(data)
        data = api(zero=True)
        data['frames'][1] = api()['frames'][1]
        with self.assertRaises(ValueError):
            classify_api(data, 1)

    def test_api_failure_sensitivity(self):
        mutations = [
            (['complete'], True), (['failures'], []), (['submitted_frames'], 119),
            (['frames', 0, 'gpu_status'], 5), (['frames', 0, 'visible'], False),
            (['frames', 0, 'active'], False), (['frames', 1, 'sequence'], 0),
            (['frames', 1, 'drawable_id'], 0), (['frames', 0, 'gpu_start_s'], 0.),
            (['frames', 0, 'gpu_end_s'], 9.), (['frames', 0, 'completion_callback_host_s'], 9.),
            (['frames', 0, 'presentation_callback_host_s'], 9.),
            (['frames', 0, 'submit_after_s'], 9.), (['frames', 0, 'before_presented_s'], 1.),
        ]
        for field in api()['cleanup']:
            mutations.append((['cleanup', field], False))
        for path, value in mutations:
            with self.subTest(path=path):
                data = api(zero=True)
                set_path(data, path, value)
                with self.assertRaises(ValueError):
                    classify_api(data, 1)
        for code in (-11, -9, 0, 2, 124, 137, 139, True, False):
            with self.subTest(code=code), self.assertRaises(ValueError):
                classify_api(api(zero=True), code)

    def test_missing_nonfinite_and_wrong_type_fields_fail(self):
        for make, classify, rows in ((hook, classify_hook, lambda d: d['sessions'][0]['records']),
                                     (api, lambda d: classify_api(d, 1), lambda d: d['frames'])):
            original = make(zero=True)
            for field in list(rows(original)[0]):
                data = copy.deepcopy(original)
                del rows(data)[0][field]
                with self.subTest(missing=field), self.assertRaises((KeyError, ValueError)):
                    classify(data)
            for field in (f for f in rows(original)[0] if f.endswith('_s')):
                for value in (float('nan'), float('inf'), -1, True, '0'):
                    data = copy.deepcopy(original)
                    rows(data)[0][field] = value
                    with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                        classify(data)

    def test_historical_hosted_evidence(self):
        # Replaying old evidence tests classification; it is not a new native run.
        archive = Path(__file__).resolve().parent.parent / 'docs/evidence/hosted-metal-diagnostic-och17/foundation-37744396036.tar.gz'
        with tarfile.open(archive) as source:
            data = json.load(source.extractfile('metal-presentation-hook/report.json'))
            self.assertEqual(classify_hook(data), 'unavailable')
            old_api = json.load(source.extractfile('metal-presentation-calibration.json'))
            # Old API reports precede explicit cleanup evidence and stay unaccepted.
            with self.assertRaises((KeyError, ValueError)):
                classify_api(old_api, 1)

    def test_duplicate_and_nonfinite_json_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'report.json'
            for text in ('{"passed": false, "passed": true}', '{"time": NaN}',
                         '{"time": Infinity}', '{"time": -Infinity}'):
                path.write_text(text)
                with self.subTest(text=text), self.assertRaises(ValueError):
                    load_report(path)

    def test_runner_retains_failures_and_does_not_accept_crashes_or_timeout(self):
        for code in (1, -11, 2):
            with self.subTest(code=code), tempfile.TemporaryDirectory() as temporary:
                output = Path(temporary) / 'fresh'
                def child(*args, **kwargs):
                    json.dump(api(zero=True), kwargs['stdout'])
                    return subprocess.CompletedProcess(args[0], code)
                with patch('sys.argv', ['probe', '--api-binary', '/unused', '--output', str(output)]), \
                     patch('metal_timing_availability.subprocess.run', side_effect=child), \
                     patch('builtins.print'):
                    if code == 1:
                        main()
                        self.assertEqual(load_report(output / 'availability.json')['status'], 'unavailable')
                    else:
                        with self.assertRaises(ValueError):
                            main()
                        self.assertFalse((output / 'availability.json').exists())
                self.assertEqual(load_report(output / 'report.json'), api(zero=True))
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / 'fresh'
            with patch('sys.argv', ['probe', '--api-binary', '/unused', '--output', str(output)]), \
                 patch('metal_timing_availability.subprocess.run',
                       side_effect=subprocess.TimeoutExpired('probe', 25)):
                with self.assertRaises(subprocess.TimeoutExpired):
                    main()
            self.assertFalse((output / 'availability.json').exists())


if __name__ == '__main__':
    unittest.main()
