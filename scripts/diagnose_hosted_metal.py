"""Bounded hosted-only experiment; results do not waive presentation gates."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
if os.environ.get('GITHUB_ACTIONS') != 'true':
    parser.error('This experiment runs only on the hosted CI desktop')
args.output.mkdir(parents=True, exist_ok=True)
source = Path('scripts/qualify_metal_presentation.swift').read_text()
# The plain case is byte-for-byte the maintained probe. The capture case only
# adds an owned-window screenshot every tenth submitted frame; no counters,
# timestamp source, validation, activation or deadline is relaxed.
needle = '            buffer.commit()\n            records.add(sequence, ["submit_after_s": CACurrentMediaTime()])'
assert source.count(needle) == 1
capture = source.replace(needle, needle + '''
            if sequence % 10 == 0, let window = self.window {
                let task = Process()
                task.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
                task.arguments = ["-x", "-l", String(window.windowNumber),
                                  "capture-\\(sequence).png"]
                do {
                    try task.run()
                    task.waitUntilExit()
                    fputs("CAPTURE \\(sequence) exit=\\(task.terminationStatus)\\n", stderr)
                } catch {
                    fputs("CAPTURE \\(sequence) error=\\(error)\\n", stderr)
                }
            }
''')
summary = []
serial_needle = '        if sent == 120 {'
assert source.count(serial_needle) == 1
serial = source.replace(serial_needle, '        // Diagnostic: one in-flight drawable, retaining every submitted frame.\n        // Wait for its presentation callback before admitting another submission.\n        if sent > 0 && records.snapshot().last?["presented_s"] == nil { return }\n' + serial_needle)
for mode, text in [('serial', serial)]:
    directory = (args.output / mode).resolve()
    directory.mkdir(exist_ok=False)
    swift = directory / 'probe.swift'
    swift.write_text(text)
    binary = directory / 'probe'
    with (directory / 'build.log').open('w') as log:
        subprocess.run(['xcrun', 'swiftc', '-O', str(swift), '-o', str(binary)],
                       stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
    awake = subprocess.Popen(['caffeinate', '-di', '-w', str(os.getpid())])
    try:
        with (directory / 'report.json').open('w') as out, (directory / 'stderr.log').open('w') as err:
            result = subprocess.run([str(binary)], cwd=directory, stdout=out, stderr=err, timeout=30)
        report = json.loads((directory / 'report.json').read_text())
        frames = report.get('frames', [])
        summary.append({'mode': mode, 'returncode': result.returncode,
                        'source_sha256': hashlib.sha256(swift.read_bytes()).hexdigest(),
                        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                        'complete': report.get('complete'), 'device': report.get('device'),
                        'failures': report.get('failures'), 'frames': len(frames),
                        'positive_presentations': sum(f.get('presented_s', 0) > 0 for f in frames),
                        'gpu_completions': sum(f.get('gpu_status') == 4 for f in frames),
                        'screenshots': len(list(directory.glob('capture-*.png')))})
    finally:
        awake.terminate()
        awake.wait(timeout=5)
        binary.unlink(missing_ok=True)
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
# Success here means the two diagnostics ran and retained their results. It is
# deliberately not named or used as a release qualification check.
