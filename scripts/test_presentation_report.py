"""Accounting, pairing and failure sensitivity for native presentation evidence."""
import copy
import json
import unittest

from presentation_report import PREFIX, COUNTS, METRICS, validate, budget_failures, check_partial_line


def fixture(count=3):
    trace = [dict(sequence=i, drawable=i, new_scene=True, active=True, animating=True,
                  inputs=int(i == 1), outcome='Presented', submit_host_s=10. + i,
                  presented_host_s=10.01 + i, callback_host_s=10.02 + i,
                  submission_lower_ns=9_999_999, submission_upper_ns=10_000_001,
                  input_lower_ns=19_999_999 if i == 1 else None,
                  input_upper_ns=20_000_001 if i == 1 else None) for i in range(min(count, 4096))]
    def h(n, value):
        return dict(count=n, buckets=[[value, n]] if n else [])
    inputs = int(count > 1)
    counts = dict.fromkeys(COUNTS, 0)
    counts.update(attempted=count, admitted=count, presented=count, trace_truncated=max(0, count - 4096))
    native = dict(supported=True, session=1, window='WindowId(1)', accepting=False,
                  window_closed=False, pending=0, counts=counts, trace=trace,
                  histograms=dict(zip(METRICS, [h(count, 10_002_431), h(inputs, 20_004_863), h(max(0,count-1),1_000_000_000)])))
    report = dict(schema=1, kind='gpui_presentation_interval', cpu_elapsed_ns=5_000_000_000,
                  cpu_input_samples=inputs, cpu_draw_samples=count, start_capture_ns=1000,
                  stop_capture_ns=10, settlement_ns=100, snapshot_encode_ns=200,
                  idle_observation_mode=False, observations_truncated=False,
                  observations=[dict(elapsed_ns=100, active=True, visible=True)],
                  end_observation=dict(active=True, visible=True), native=native)
    cpu = dict(elapsed_ns=5_000_000_000, histograms=dict(input_to_frame=dict(count=inputs),draw=dict(count=count)))
    return report, cpu


def run(report, cpu, phase='history'):
    return validate(PREFIX+json.dumps(report), [(phase, cpu)], enabled=True)


class Reports(unittest.TestCase):
    def test_large_trace_can_arrive_across_reads_without_unbounding_other_records(self):
        check_partial_line(PREFIX.encode() + b'x' * (2 * 1024 * 1024))
        for fragment in (b'GPUIO_STREAM ' + b'x' * 65536,
                         PREFIX.encode() + b'x' * (4 * 1024 * 1024 + 1)):
            with self.assertRaisesRegex(ValueError, 'Oversized'):
                check_partial_line(fragment)

    def test_paired_trace_histograms_and_idle(self):
        report, cpu = fixture()
        result = run(report, cpu)['history']['native']['histograms']
        self.assertEqual(result[METRICS[1]]['count'], 1)
        self.assertEqual(result[METRICS[0]]['p99'], 10_002_431)
        self.assertEqual(run(*fixture(0), phase='idle')['idle']['native']['counts']['attempted'], 0)
        with self.assertRaisesRegex(ValueError, 'during idle'):
            run(report, cpu, phase='idle')

    def test_no_missing_extra_or_unrequested_report(self):
        r, cpu = fixture()
        line = PREFIX+json.dumps(r)
        for output, enabled in [('', True), (line+'\n'+line, True), (line, False)]:
            with self.assertRaises(ValueError):
                validate(output, [('history',cpu)], enabled=enabled)
        self.assertIsNone(validate('', [], enabled=False))

    def test_reused_sessions_and_wrong_window(self):
        first, cpu = fixture()
        for session, window in [(1,'WindowId(1)'), (2,'WindowId(2)')]:
            second = copy.deepcopy(first)
            second['native'].update(session=session, window=window)
            with self.assertRaises(ValueError):
                validate('\n'.join(PREFIX+json.dumps(r) for r in (first,second)),
                         [('one',cpu),('two',cpu)], enabled=True)

    def test_failure_counters_pending_and_lifetime_are_not_ignored(self):
        for key in COUNTS:
            r,cpu=fixture()
            r['native']['counts'][key]+=1
            with self.subTest(key=key), self.assertRaises(ValueError):
                run(r,cpu)
        for key,value in [('pending',1),('accepting',True),('window_closed',True),('supported',False)]:
            r,cpu=fixture();r['native'][key]=value
            with self.subTest(key=key), self.assertRaises(ValueError):
                run(r,cpu)

    def test_pairing_and_sampled_visibility(self):
        for key in ('cpu_elapsed_ns','cpu_input_samples','cpu_draw_samples'):
            r,cpu=fixture();r[key]+=1
            with self.assertRaises(ValueError):
                run(r,cpu)
        for key,value in [('active',False),('visible',False),('visible',None)]:
            r,cpu=fixture();r['observations'][0][key]=value
            with self.assertRaises(ValueError):
                run(r,cpu)
        r,cpu=fixture();r['observations_truncated']=True
        with self.assertRaises(ValueError):
            run(r,cpu)

    def test_trace_clocks_bounds_and_input_must_agree(self):
        for key,value in [('sequence',4),('presented_host_s',float('nan')),
                          ('callback_host_s',1.),('submission_lower_ns',20_000_000),
                          ('input_upper_ns',1),('inputs',False)]:
            r,cpu=fixture();r['native']['trace'][1][key]=value
            with self.subTest(key=key), self.assertRaises(ValueError):
                run(r,cpu)
        r,cpu=fixture();r['native']['trace'][0]['input_upper_ns']=0
        with self.assertRaises(ValueError):
            run(r,cpu)

    def test_histogram_corruption_and_duplicate_json_fields(self):
        for buckets in ([[10,1],[9,2]], [[10,0]], [[10,4]], [[True,3]]):
            r,cpu=fixture();r['native']['histograms'][METRICS[0]]['buckets']=buckets
            with self.assertRaises(ValueError):
                run(r,cpu)
        r,cpu=fixture()
        with self.assertRaisesRegex(ValueError,'Duplicate'):
            validate(PREFIX+json.dumps(r)[:-1]+',"schema":1}', [('history',cpu)],enabled=True)

    def test_trace_capacity_is_not_histogram_sample_loss(self):
        result=run(*fixture(4100))
        self.assertEqual(result['history']['native']['counts']['trace_truncated'],4)
        self.assertEqual(budget_failures(result),[])
        self.assertIn('fewer than 1000 paired input frames',';'.join(budget_failures(result,require_input=True)))
        result['history']['native']['histograms'][METRICS[0]]['p99']=50_000_001
        self.assertTrue(budget_failures(result))


if __name__=='__main__':
    unittest.main()
