"""Physical-memory evidence must identify the owned process and valid units."""
import copy
import io
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from mac_process_memory import footprint_summary, sample
from measure_resource_lifecycle import Checkpoints
from test_measure_resource_lifecycle import snapshot


def fixture():
    return dict(unit='byte', **{'bytes per unit': 1}, errors=[], warnings=[],
                processes=[dict(pid=123, name='owned', footprint=100000,
                                categories={'IOSurface': dict(dirty=4096, swapped=0,
                                    clean=0, reclaimable=0, wired=0, regions=1)})])


class PhysicalMemory(unittest.TestCase):
    def test_identity_units_diagnostics_and_categories_are_required(self):
        good = fixture()
        result = footprint_summary(good, 123)
        self.assertEqual(result['footprint_bytes'], 100000)
        self.assertEqual(result['categories']['IOSurface']['dirty'], 4096)
        changes = [lambda r: r.update(unit='KiB'),
                   lambda r: r.update({'bytes per unit': True}),
                   lambda r: r.update(errors=['denied']),
                   lambda r: r.pop('warnings'),
                   lambda r: r.update(warnings=['partial']),
                   lambda r: r['processes'].append(copy.deepcopy(r['processes'][0])),
                   lambda r: r['processes'][0].update(pid=124),
                   lambda r: r['processes'][0].update(footprint=-1),
                   lambda r: r['processes'][0].update(categories={}),
                   lambda r: r['processes'][0]['categories']['IOSurface'].update(dirty=-1),
                   lambda r: r['processes'][0]['categories']['IOSurface'].pop('wired')]
        for change in changes:
            bad = copy.deepcopy(good)
            change(bad)
            with self.assertRaises(ValueError):
                footprint_summary(bad, 123)

    def test_raw_outputs_retained_and_vmmap_identity_checked(self):
        with tempfile.TemporaryDirectory() as tmp:
            for pid in (123, 124):
                directory = Path(tmp) / str(pid)
                def run(args, *, stdout, stderr, timeout):
                    self.assertEqual(timeout, 6)
                    if args[0] == '/usr/bin/footprint':
                        self.assertEqual(args[1:3], ['-p', '123'])
                        Path(args[-1]).write_text(json.dumps(fixture()))
                        stdout.write(b'raw footprint output\n')
                    else:
                        self.assertEqual(args, ['/usr/bin/vmmap', '-summary', '123'])
                        stdout.write(f'Process: owned [{pid}]\nPhysical footprint: 100K\n'.encode())
                    return SimpleNamespace(returncode=0)
                with patch('mac_process_memory.subprocess.run', side_effect=run):
                    if pid == 123:
                        result = sample(123, directory)
                        self.assertEqual(result['footprint_bytes'], 100000)
                        self.assertIn('vmmap.stdout', result['artifacts'])
                    else:
                        with self.assertRaisesRegex(ValueError, 'exact owned process'):
                            sample(123, directory)
                self.assertTrue((directory / 'tools.json').exists())
                self.assertTrue((directory / 'footprint.json').exists())

    def test_timeout_keeps_partial_output_and_does_not_acknowledge_child(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            log = root / 'application.log'
            log.write_text(f'GPUIO_LIFECYCLE checkpoint (1 {snapshot()})\n')
            samples = []
            checkpoints = Checkpoints(log, samples, cycles=4, physical_memory=True)
            child = SimpleNamespace(pid=123, stdin=io.BytesIO())
            def timeout(args, *, stdout, stderr, timeout):
                stdout.write(b'partial diagnostic\n')
                raise subprocess.TimeoutExpired(args, timeout)
            with patch('measure_resource_lifecycle.subprocess.check_output', return_value='100'), \
                    patch('mac_process_memory.subprocess.run', side_effect=timeout):
                with self.assertRaises(subprocess.TimeoutExpired):
                    checkpoints(child)
            self.assertEqual(child.stdin.getvalue(), b'')
            self.assertEqual(len(samples), 1)
            directory = root / 'physical-memory' / 'cycle-001'
            self.assertEqual((directory / 'footprint.stdout').read_text(), 'partial diagnostic\n')
            self.assertIsNone(json.loads((directory / 'tools.json').read_text())[0]['returncode'])

    def test_audit_finishes_before_acknowledgement(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / 'application.log'
            log.write_text(f'GPUIO_LIFECYCLE checkpoint (1 {snapshot()})\n')
            samples = []
            checkpoints = Checkpoints(log, samples, cycles=4, physical_memory=True)
            child = SimpleNamespace(pid=123, stdin=io.BytesIO())
            def memory(pid, path):
                self.assertEqual(pid, 123)
                self.assertEqual(child.stdin.getvalue(), b'')
                return dict(footprint_bytes=1000)
            with patch('measure_resource_lifecycle.subprocess.check_output', return_value='100'), \
                    patch('mac_process_memory.sample', side_effect=memory):
                checkpoints(child)
            self.assertEqual(samples[0]['physical_memory']['footprint_bytes'], 1000)
            self.assertEqual(child.stdin.getvalue(), b'continue 1\n')


if __name__ == '__main__':
    unittest.main()
