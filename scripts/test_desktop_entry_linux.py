#!/usr/bin/env python3
"""Validate generated Exec paths through real GIO without a display or GUI app."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def exercise(binary):
    with tempfile.TemporaryDirectory(prefix='gpuio-desktop-entry-') as temporary:
        root = Path(temporary)
        capture = root / 'argv.json'
        env = {**os.environ, 'GPUIO_DESKTOP_CAPTURE': str(capture)}
        uri = 'gpuio-desktop-lab://document/literal%25?value=$HOME'
        for name in ['App $`"\\ runtime', 'App $`"\\% runtime']:
            executable = root / name
            executable.write_text(
                '#!/usr/bin/python3\n'
                'import json, os, sys\n'
                'from pathlib import Path\n'
                'path = Path(os.environ["GPUIO_DESKTOP_CAPTURE"])\n'
                'temporary = path.with_suffix(".pending")\n'
                'temporary.write_text(json.dumps(sys.argv))\n'
                'temporary.replace(path)\n')
            executable.chmod(0o700)
            entry = root / 'com.gpuio.desktop-lab.desktop'
            entry.write_bytes(subprocess.check_output(
                [str(binary), '--print-desktop-entry', str(executable), '--self-test'],
                timeout=10))
            subprocess.run(['desktop-file-validate', str(entry)], check=True, timeout=10)
            capture.unlink(missing_ok=True)
            subprocess.run(['gio', 'launch', str(entry), uri], env=env, check=True, timeout=10)
            end = time.monotonic() + 10
            while not capture.exists() and time.monotonic() < end:
                time.sleep(.02)
            assert capture.exists(), 'GIO did not launch the generated desktop executable'
            actual = json.loads(capture.read_text())
            assert actual == [str(executable), '--self-test', '--open-uris', uri], actual
            print('DESKTOP_ENTRY_EXEC_OK', repr(name), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('_build/default/examples/desktop/main.exe'))
    args = parser.parse_args()
    if sys.platform != 'linux':
        parser.error('Requires Linux GDesktopAppInfo; no display is needed')
    exercise(args.binary.resolve(strict=True))
