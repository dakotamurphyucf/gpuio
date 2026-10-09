"""Scoped physical appearance switching for native qualification only.

Save a recovery record before any changes. Restore both effective dark mode and
the automatic-switch preference, including its original absence. No VoiceOver,
contrast, input-source or clipboard settings are accessed.
"""
import argparse
import contextlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time

AUTO_KEY = 'AppleInterfaceStyleSwitchesAutomatically'


def checked_state(value):
    if (not isinstance(value, dict) or set(value) != {'dark', 'automatic'}
            or type(value['dark']) is not bool
            or (value['automatic'] is not None and type(value['automatic']) is not bool)):
        raise ValueError('Invalid appearance recovery state')
    return value


class Appearance:
    def script(self, action):
        return subprocess.check_output(['/usr/bin/osascript', '-e',
                                        'tell application "System Events" to tell appearance preferences to ' + action],
                                       text=True, timeout=10).strip()

    def dark(self):
        value = self.script('get dark mode')
        if value not in ('true', 'false'):
            raise RuntimeError('Unexpected macOS appearance response')
        return value == 'true'

    def automatic(self):
        result = subprocess.run(['/usr/bin/defaults', 'read', '-g', AUTO_KEY],
                                capture_output=True, text=True, timeout=10,
                                env={**os.environ, 'LC_ALL': 'C'})
        if result.returncode == 1 and 'does not exist' in result.stderr:
            return None
        result.check_returncode()
        if result.stdout.strip() not in ('0', '1'):
            raise RuntimeError('Unsupported automatic appearance preference')
        return result.stdout.strip() == '1'

    def snapshot(self):
        return {'dark': self.dark(), 'automatic': self.automatic()}

    def set_dark(self, value):
        if type(value) is not bool:
            raise ValueError('Expected a Boolean appearance')
        self.script('set dark mode to ' + str(value).lower())
        deadline = time.monotonic() + 5
        while self.dark() != value:
            if time.monotonic() >= deadline:
                raise RuntimeError('macOS appearance did not settle')
            time.sleep(.05)

    def restore(self, saved):
        checked_state(saved)
        if self.dark() != saved['dark']:
            self.set_dark(saved['dark'])
        if self.automatic() != saved['automatic']:
            arguments = (['delete', '-g', AUTO_KEY] if saved['automatic'] is None else
                         ['write', '-g', AUTO_KEY, '-bool', str(saved['automatic']).lower()])
            subprocess.run(['/usr/bin/defaults', *arguments], check=True, timeout=10)
        if self.snapshot() != saved:
            raise RuntimeError('Original macOS appearance was not restored')


def write_record(path, saved, restored):
    path.write_text(json.dumps({'version': 1, 'original': saved, 'restored': restored}, indent=2) + '\n')


@contextlib.contextmanager
def preserved_appearance(path, factory=Appearance):
    appearance = factory()
    saved = checked_state(appearance.snapshot())
    # The existing directory is caller-owned; do not overwrite another recovery.
    with path.open('x') as stream:
        json.dump({'version': 1, 'original': saved, 'restored': False}, stream, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())
    previous = signal.getsignal(signal.SIGTERM)
    def interrupted(signum, _frame):
        raise SystemExit(128 + signum)
    signal.signal(signal.SIGTERM, interrupted)
    try:
        yield appearance
    finally:
        try:
            appearance.restore(saved)
            write_record(path, saved, True)
        finally:
            signal.signal(signal.SIGTERM, previous)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--restore', type=Path, required=True)
    args = parser.parse_args()
    record = json.loads(args.restore.read_text())
    if record.get('version') != 1:
        raise ValueError('Unknown appearance recovery version')
    saved = checked_state(record['original'])
    Appearance().restore(saved)
    write_record(args.restore, saved, True)
    print('GPUIO_SYSTEM_APPEARANCE_RESTORED')


if __name__ == '__main__':
    main()
