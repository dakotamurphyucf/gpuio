#!/usr/bin/env python3
"""Exercise public file themes through the native picker and actual scoped Eio I/O.

Only disposable fixtures and one owned gallery process are modified. A held FIFO
writer delays EOF to test a real pending read without changing the application.
No VoiceOver, system appearance, input-source or clipboard settings are changed.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import tempfile
import time

from package_macos_reference import digest
from test_agent_chat import Mac
from test_canvas import screenshot
from test_gallery import (TITLE, SECOND, expect_field, expect_enabled,
                          focus_gallery_control, raise_gallery, reveal_gallery_control)
from test_gallery_desktop_macos import choose_path
from test_macos_text_selection import Selection, utf16
from window_pixels import read_png

LABEL = 'Theme preview draft'
DRAFT = 'Retained λ🙂 theme draft'


def reload_theme(mac):
    # A retained status/profile can already be present while this page's Eio
    # scope is reacquiring. Enabled is the actual Ready/not-busy boundary.
    expect_enabled(mac, 'Reload file', True)
    mac.press(TITLE, 'Reload file')


def wait_for_reader(child, path):
    # Loading status can precede the worker actually opening its flow. Observe
    # the owned application's descriptor before releasing the delayed fixture.
    # macOS lsof's filename filter omits FIFOs; query only the owned PID.
    # In the C locale it hex-escapes non-ASCII pathname bytes.
    expected = b'n' + re.sub(rb'[\x80-\xff]', lambda m: f'\\x{m[0][0]:02x}'.encode(),
                            os.fsencode(path))
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        result = subprocess.run(['/usr/sbin/lsof', '-a', '-p', str(child.pid), '-Fn'],
                                capture_output=True, timeout=5, env={**os.environ, 'LC_ALL': 'C'})
        if result.returncode == 0 and expected in result.stdout.splitlines():
            return
        assert child.poll() is None, 'Gallery exited before opening theme fixture'
        time.sleep(.05)
    raise RuntimeError('Gallery did not open the pending theme read')


def painted_surface(mac, output, name, rgb):
    raise_gallery(mac)
    reveal_gallery_control(mac, 'Choose theme', 'AXButton')
    time.sleep(.2)
    path = output / (name + '.png')
    screenshot(mac, path, title=TITLE)
    pixels = read_png(mac, path)
    count = sum(all(abs(a-b) <= 2 for a, b in zip(pixels.rgb(x, y), rgb))
                for y in range(0, pixels.height, 4) for x in range(0, pixels.width, 4))
    assert count > 1000, (name, 'Profile surface not visibly painted', rgb, count)
    return {'capture': path.name, 'surface_rgb': rgb, 'matching_sampled_pixels': count}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/gallery/main.exe'))
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    Mac.require_accessibility()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'platform': platform.platform(),
              'executable_sha256': digest(args.executable), 'checks': []}
    child = mac = None
    descriptor = None
    node = None
    def interrupted(signum, _frame):
        raise RuntimeError(f'Theme test interrupted: {signum}')
    signal.signal(signal.SIGALRM, interrupted)
    signal.signal(signal.SIGTERM, interrupted)
    signal.alarm(150)
    try:
        with tempfile.TemporaryDirectory(prefix='gpuio-theme-files-') as temporary, \
                (output / 'application.log').open('w') as log:
            path = Path(temporary).resolve() / 'theme λ profile.sexp'
            original = Path('examples/gallery/themes/aurora.sexp').read_text()
            changed = original.replace('Aurora', 'Amber').replace('#1c2033', '#30301c')
            path.write_text(original)
            child = subprocess.Popen([str(args.executable.resolve())], stdout=log,
                                     stderr=subprocess.STDOUT)
            mac = Mac(child.pid, child)
            mac.wait_text(TITLE, 'A little context goes a long way')
            raise_gallery(mac)
            mac.press(TITLE, 'Styling details')
            mac.wait_text(TITLE, 'A theme from your workspace')
            expect_enabled(mac, 'Reload file', False)
            focus_gallery_control(mac, LABEL, 'AXTextField')
            mac.field(TITLE, LABEL, 'AXTextField', DRAFT)
            expect_field(mac, TITLE, LABEL, DRAFT)
            select = Selection(mac)
            node = mac.wait_find(TITLE, LABEL, 'AXTextField')
            start = utf16(DRAFT[:DRAFT.index('λ🙂')])
            select.set(node, start, utf16('λ🙂'))
            select.expect(node, 'λ🙂', start, utf16('λ🙂'))
            choose_path(mac, path, open_label='Choose theme', accept_label='Load theme',
                        expected='Loaded Aurora')
            mac.wait_text(TITLE, 'Current profile: Aurora')
            expect_field(mac, TITLE, LABEL, DRAFT)
            select.expect(node, 'λ🙂', start, utf16('λ🙂'))
            report['checks'].append({'case': 'picker-load-selection',
                                     **painted_surface(mac, output, 'aurora', (28, 32, 51))})
            focus_gallery_control(mac, LABEL, 'AXTextField')
            select.set(node, utf16(DRAFT), 0)
            mac.key(51)  # Native Backspace produces a real undo history entry.
            expect_field(mac, TITLE, LABEL, DRAFT[:-1])
            path.write_text(changed)
            reload_theme(mac)
            mac.wait_text(TITLE, 'Loaded Amber')
            mac.wait_text(TITLE, 'Current profile: Amber')
            expect_field(mac, TITLE, LABEL, DRAFT[:-1])
            select.expect(node, '', utf16(DRAFT[:-1]), 0)
            report['checks'].append({'case': 'same-appearance-color-reload',
                                     **painted_surface(mac, output, 'amber', (48, 48, 28))})
            focus_gallery_control(mac, LABEL, 'AXTextField')
            mac.key(6, flags=1 << 20)  # Command+Z; history survives the theme change.
            expect_field(mac, TITLE, LABEL, DRAFT)
            report['checks'].append({'case': 'native-undo-after-reload'})
            select.set(node, start, utf16('λ🙂'))
            mac.press(TITLE, 'Comfortable')
            mac.release(mac.wait_find(TITLE, 'Large', 'AXButton'))
            select.expect(node, 'λ🙂', start, utf16('λ🙂'))
            expect_field(mac, TITLE, LABEL, DRAFT)
            mac.press(TITLE, 'Large')
            mac.press(TITLE, 'Compact')
            report['checks'].append({'case': 'scale-retains-selection-and-theme'})

            path.write_text('((version 999))')
            reload_theme(mac)
            mac.wait_text(TITLE, 'Theme kept.')
            mac.wait_text(TITLE, 'Current profile: Amber')
            report['checks'].append({'case': 'invalid-last-good',
                                     **painted_surface(mac, output, 'invalid-kept', (48, 48, 28))})
            mac.press(TITLE, 'Choose theme')
            mac.press(TITLE, 'Cancel')
            mac.wait_text(TITLE, 'Selection cancelled. Your theme was kept.')
            expect_field(mac, TITLE, LABEL, DRAFT)
            report['checks'].append({'case': 'picker-cancellation'})

            mac.press(TITLE, 'New window')
            mac.wait_text(SECOND, 'A little context goes a long way')
            mac.press(SECOND, 'Styling details')
            mac.wait_text(SECOND, 'Using the built-in appearance')
            expect_field(mac, SECOND, LABEL, 'This draft stays while the palette changes.')
            mac.close(SECOND)
            raise_gallery(mac)
            mac.wait_text(TITLE, 'Current profile: Amber')
            expect_field(mac, TITLE, LABEL, DRAFT)
            report['checks'].append({'case': 'independent-window'})

            # Replace only our selected disposable fixture. A held writer keeps
            # the real Eio read pending until the newer appearance is chosen.
            path.unlink()
            os.mkfifo(path)
            descriptor = os.open(path, os.O_RDWR | os.O_NONBLOCK)
            reload_theme(mac)
            mac.wait_text(TITLE, 'Loading theme…')
            wait_for_reader(child, path)
            expect_enabled(mac, 'Reload file', False)
            mac.press(TITLE, 'Dark')  # Switch to explicit Light, leaving the file profile.
            mac.wait_text(TITLE, 'Using the built-in appearance')
            os.write(descriptor, original.encode())
            os.close(descriptor)
            descriptor = None
            mac.wait_text(TITLE, 'A newer appearance choice was kept.')
            mac.release(mac.wait_find(TITLE, 'Light', 'AXButton'))
            expect_field(mac, TITLE, LABEL, DRAFT)
            report['checks'].append({'case': 'newer-explicit-choice-during-eio-read'})

            # Start from a file profile again so its disappearance acknowledges
            # the System choice before the test releases EOF. AXPress itself
            # only queues an action; it is not a Bonsai completion barrier.
            path.unlink()
            path.write_text(original)
            reload_theme(mac)
            mac.wait_text(TITLE, 'Current profile: Aurora')
            path.unlink()
            os.mkfifo(path)
            descriptor = os.open(path, os.O_RDWR | os.O_NONBLOCK)
            reload_theme(mac)
            mac.wait_text(TITLE, 'Loading theme…')
            wait_for_reader(child, path)
            mac.press(TITLE, 'Follow system')
            mac.wait_text(TITLE, 'Using the built-in appearance')
            os.write(descriptor, changed.encode())
            os.close(descriptor)
            descriptor = None
            mac.wait_text(TITLE, 'A newer appearance choice was kept.')
            mac.wait_text(TITLE, 'Using the built-in appearance')
            report['checks'].append({'case': 'newer-system-choice-during-eio-read'})

            descriptor = os.open(path, os.O_RDWR | os.O_NONBLOCK)
            reload_theme(mac)
            mac.wait_text(TITLE, 'Loading theme…')
            wait_for_reader(child, path)
            mac.press(TITLE, 'Presentation')
            mac.wait_text(TITLE, 'A little context goes a long way')
            os.close(descriptor)
            descriptor = None
            mac.release(node)
            node = None
            mac.press(TITLE, 'Styling details')
            mac.wait_text(TITLE, 'Using the built-in appearance')
            mac.wait_text(TITLE, 'Loading cancelled. Your theme was kept.')
            expect_enabled(mac, 'Choose theme', True)
            expect_enabled(mac, 'Reload file', True)
            expect_field(mac, TITLE, LABEL, 'This draft stays while the palette changes.')
            report['checks'].append({'case': 'pending-read-page-cancellation-fresh-editor'})

            path.unlink()
            path.write_text(original)
            reload_theme(mac)
            mac.wait_text(TITLE, 'Loaded Aurora')
            mac.press(TITLE, 'Presentation')
            mac.wait_text(TITLE, 'A little context goes a long way')
            mac.press(TITLE, 'Styling details')
            mac.wait_text(TITLE, 'Current profile: Aurora')
            mac.wait_text(TITLE, 'Loaded Aurora')
            report['checks'].append({'case': 'retry-and-applied-theme-survive-page-departure'})

            path.unlink()
            os.mkfifo(path)
            descriptor = os.open(path, os.O_RDWR | os.O_NONBLOCK)
            reload_theme(mac)
            mac.wait_text(TITLE, 'Loading theme…')
            wait_for_reader(child, path)
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
            # The writer stays open until the app has returned, so EOF cannot
            # accidentally stand in for scoped cancellation on window closure.
            os.close(descriptor)
            descriptor = None
            report['checks'].append({'case': 'window-close-cancels-read-before-eof'})
            report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        signal.alarm(0)
        if descriptor is not None:
            os.close(descriptor)
        if child and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
        if mac:
            if node:
                mac.release(node)
            mac.release(mac.app)
        report['child_reaped'] = child is None or child.poll() is not None
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print('GPUIO_THEME_FILES_OK: native picker, palette/selection/undo, last good, scoped delayed reads')


if __name__ == '__main__':
    main()
