#!/usr/bin/env python3
"""Physical system appearance, per-window overrides and retained native editing.

Temporarily switches macOS Dark/Light and restores the original state. No
VoiceOver, input-source, clipboard or contrast preferences are changed.
"""
import argparse
import json
from pathlib import Path
import platform
import re
import signal
import subprocess
import time

from mac_system_appearance import preserved_appearance
from mac_input_source import foreground_keys
from package_macos_reference import digest
from test_agent_chat import Mac
from test_canvas import screenshot
from test_gallery import TITLE, SECOND, expect_field, expect_focus, focus_gallery_control
from test_macos_text_selection import Selection, utf16
from window_pixels import read_png

LABEL = 'Theme preview draft'
DRAFT = 'Retained λ🙂 appearance draft'
OTHER_DRAFT = 'Independent second window'


def wait_native_appearance(log, offset, windows, dark):
    expected = {window: str(dark).lower() for window in windows}
    deadline = time.monotonic() + 10
    while True:
        text = log.read_bytes()[offset:].decode()
        observed = {int(window): value for window, value in re.findall(
            r'GALLERY_NATIVE_APPEARANCE window=(\d+) dark=(true|false)', text)}
        if all(observed.get(window) == value for window, value in expected.items()):
            return
        if time.monotonic() >= deadline:
            raise RuntimeError(f'Native appearance observation missing: {expected}, saw {observed}')
        time.sleep(.05)


def activate(mac, title):
    window = mac.window(title)
    assert window
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)


def palette(mac, output, name, title, dark):
    # The label acknowledges Bonsai's resolved preference; pixels independently
    # verify actual painting. Capturing never requests editor focus.
    mac.release(mac.wait_find(title, 'Dark' if dark else 'Light', 'AXButton'))
    expected = (16, 21, 29) if dark else (241, 244, 247)
    deadline = time.monotonic() + 8
    path = output / (name + '.png')
    while True:
        screenshot(mac, path, title=title)
        pixels = read_png(mac, path)
        count = sum(all(abs(a-b) <= 2 for a, b in zip(pixels.rgb(x, y), expected))
                    for y in range(0, pixels.height, 8) for x in range(0, pixels.width, 8))
        if count > 3000:
            return {'capture': path.name, 'background_rgb': expected, 'matching_samples': count}
        if time.monotonic() >= deadline:
            raise AssertionError(f'{name}: expected palette not painted ({count} samples)')
        time.sleep(.1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, default=Path('_build/default/examples/gallery/main.exe'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    Mac.require_accessibility()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'platform': platform.platform(), 'checks': [],
              'executable_sha256': digest(args.executable),
              'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], text=True))}
    child = mac = node = None
    recovery = output / 'appearance-recovery.json'
    log_path = output / 'application.log'
    def interrupted(signum, _frame):
        raise RuntimeError(f'Appearance test interrupted: {signum}')
    previous_alarm = signal.signal(signal.SIGALRM, interrupted)
    signal.alarm(150)
    try:
        with preserved_appearance(recovery) as system, log_path.open('w') as log:
            system.set_dark(True)
            child = subprocess.Popen([str(args.executable.resolve()), '--trace-window-appearance'],
                                     stdout=log, stderr=subprocess.STDOUT)
            mac = Mac(child.pid, child)
            foreground_keys(mac)
            mac.wait_text(TITLE, 'A little context goes a long way')
            wait_native_appearance(log_path, 0, [1], True)
            activate(mac, TITLE)
            mac.press(TITLE, 'Follow system')
            mac.press(TITLE, 'Styling details')
            mac.wait_text(TITLE, 'A theme from your workspace')
            focus_gallery_control(mac, LABEL, 'AXTextField')
            mac.field(TITLE, LABEL, 'AXTextField', DRAFT)
            expect_field(mac, TITLE, LABEL, DRAFT)
            report['checks'].append({'case': 'initial-system-dark',
                                     **palette(mac, output, 'initial-dark', TITLE, True)})

            mac.press(TITLE, 'New window')
            mac.wait_text(SECOND, 'A little context goes a long way')
            wait_native_appearance(log_path, 0, [2], True)
            activate(mac, SECOND)
            mac.press(SECOND, 'Dark')  # Explicit Light independently of the OS.
            mac.press(SECOND, 'Styling details')
            mac.wait_text(SECOND, 'A theme from your workspace')
            mac.field(SECOND, LABEL, 'AXTextField', OTHER_DRAFT)
            expect_field(mac, SECOND, LABEL, OTHER_DRAFT)
            report['checks'].append({'case': 'second-explicit-light',
                                     **palette(mac, output, 'second-light', SECOND, False)})

            focus_gallery_control(mac, LABEL, 'AXTextField')
            node = mac.wait_find(TITLE, LABEL, 'AXTextField')
            selection = Selection(mac)
            start = utf16(DRAFT[:DRAFT.index('λ🙂')])
            selection.set(node, start, utf16('λ🙂'))
            selection.expect(node, 'λ🙂', start, utf16('λ🙂'))
            for dark in (False, True):
                offset = log_path.stat().st_size
                system.set_dark(dark)
                wait_native_appearance(log_path, offset, [1, 2], dark)
                painted = palette(mac, output, 'system-dark' if dark else 'system-light', TITLE, dark)
                expect_focus(mac, LABEL, 'AXTextField')
                selection.expect(node, 'λ🙂', start, utf16('λ🙂'))
                expect_field(mac, TITLE, LABEL, DRAFT)
                report['checks'].append({'case': 'system-dark' if dark else 'system-light',
                                         **painted})
            # No refocusing after the OS changes: native editing must still reach
            # the original editor and undo must restore the original selection.
            mac.key(51)
            expect_field(mac, TITLE, LABEL, DRAFT.replace('λ🙂', ''))
            mac.key(6, flags=1 << 20)
            expect_field(mac, TITLE, LABEL, DRAFT)
            selection.expect(node, 'λ🙂', start, utf16('λ🙂'))
            mac.key(0)
            expect_field(mac, TITLE, LABEL, DRAFT.replace('λ🙂', 'a'))
            mac.key(6, flags=1 << 20)
            expect_field(mac, TITLE, LABEL, DRAFT)
            report['checks'].append({'case': 'retained-focus-selection-native-edit-and-undo'})

            activate(mac, SECOND)
            expect_field(mac, SECOND, LABEL, OTHER_DRAFT)
            report['checks'].append({'case': 'explicit-light-survives-os-dark',
                                     **palette(mac, output, 'second-still-light', SECOND, False)})
            mac.close(SECOND)
            activate(mac, TITLE)
            # Explicit Dark must stay dark across a real OS Light transition.
            mac.press(TITLE, 'Dark')
            mac.release(mac.wait_find(TITLE, 'Light', 'AXButton'))
            mac.press(TITLE, 'Light')
            mac.release(mac.wait_find(TITLE, 'Dark', 'AXButton'))
            focus_gallery_control(mac, LABEL, 'AXTextField')
            selection.set(node, start, utf16('λ🙂'))
            offset = log_path.stat().st_size
            system.set_dark(False)
            wait_native_appearance(log_path, offset, [1], False)
            painted = palette(mac, output, 'explicit-dark', TITLE, True)
            expect_focus(mac, LABEL, 'AXTextField')
            selection.expect(node, 'λ🙂', start, utf16('λ🙂'))
            report['checks'].append({'case': 'explicit-dark-survives-os-light',
                                     **painted})
            mac.press(TITLE, 'Follow system')
            report['checks'].append({'case': 'resume-follow-current-system',
                                     **palette(mac, output, 'follow-again-light', TITLE, False)})
            expect_field(mac, TITLE, LABEL, DRAFT)
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
        report['complete'] = True  # Only after successful preference restoration.
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, previous_alarm)
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
        if recovery.exists():
            report['appearance_recovery'] = json.loads(recovery.read_text())
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print('GPUIO_SYSTEM_APPEARANCE_OK: native palettes, independent windows, retained editing, restored OS')


if __name__ == '__main__':
    main()
