"""Reject redraws, incomplete idle intervals, activation changes and occlusion."""
import copy
import unittest

from measure_idle import validate
from test_measure_resource_lifecycle import snapshot


def fixture():
    lines = ['GPUIO_IDLE_PERF config (true 2)']
    evidence = dict(editor_focused_then_blurred=True)
    for phase, active in [('focused', 'true'), ('unfocused', 'false')]:
        lines += [f'GPUIO_IDLE_PERF ready {phase}',
                  f'GPUIO_IDLE_PERF begin ({phase} 100)',
                  f'GPUIO_IDLE_PERF state-before ({phase} ({active}))',
                  f'GPUIO_IDLE_PERF finish ({phase} (Finished (elapsed_ns 2000000000)(capture_ns 100)(dropped_inputs 0)(counts (0 0 0 0 0))))']
        lines.append(f'GPUIO_IDLE_PERF observations ({phase} ((0 {active} (true))(1000000000 {active} (true))))')
        for metric in range(5):
            lines.append(f'GPUIO_IDLE_PERF buckets ({phase} (Buckets (metric {metric})(offset 0)(total 0)(values ())))')
        lines.append(f'GPUIO_IDLE_PERF state-after ({phase} ({active}) 0)')
        evidence[phase] = [dict(elapsed_seconds=t, frontmost=active=='true') for t in (0.,1.)]
    lines += [f'GPUIO_IDLE_PERF cleanup {snapshot()}', 'GPUIO_IDLE_PERF complete 2']
    return '\n'.join(lines), evidence


class Idle(unittest.TestCase):
    def test_both_complete_intervals_and_cleanup(self):
        output, evidence = fixture()
        result = validate(output,evidence,smoke=True)
        self.assertEqual(set(result), {'focused','unfocused'})
        self.assertEqual(result['unfocused']['histograms']['draw']['count'],0)

    def test_short_interrupted_or_redrawing_idle_rejected(self):
        output, evidence = fixture()
        for invalid in [output.replace('2000000000','1999999999',1),
                        output.replace('state-after (focused (true) 0)','state-after (focused (true) 2)'),
                        output.replace('state-before (unfocused (false))','state-before (unfocused (true))'),
                        output.replace('(counts (0 0 0 0 0))','(counts (1 0 0 0 0))',1),
                        output.replace('(1000000000 false (true))','(1000000000 false ())'),
                        output.replace('(1000000000 false (true))','(1000000000 false (false))'),
                        output.replace('(1000000000 false (true))','(1000000000 true (true))'),
                        output.replace('(dropped_inputs 0)','(dropped_inputs 1)',1),
                        output.replace('(windows 0)','(windows 1)'),
                        output.rsplit('\n',1)[0]]:
            with self.assertRaises(ValueError):
                validate(invalid,evidence,smoke=True)
        for mutate in [lambda e:e.update(editor_focused_then_blurred=False),
                       lambda e:e['focused'][0].update(frontmost=False),
                       lambda e:e['focused'][1].update(elapsed_seconds=0.),
                       lambda e:e['focused'][1].update(elapsed_seconds=1.6),
                       lambda e:e['focused'].clear()]:
            bad=copy.deepcopy(evidence);mutate(bad)
            with self.assertRaises(ValueError):
                validate(output,bad,smoke=True)


if __name__ == '__main__':
    unittest.main()
