#!/usr/bin/env python3
"""Check real AppKit scrollbar snapshots and public gallery application.

Uses process-local argument defaults for both styles; never writes OS settings.
This does not test live System Settings notifications or physical fade timing.
"""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import time

from test_agent_chat import Mac
from test_canvas import screenshot
from test_gallery import (TITLE, expect_focus, focus_gallery_control,
                          reveal_gallery_control)

BAR = 'Collection preview — vertical'


def value(mac):
    node = mac.wait_find(TITLE, BAR, 'AXScrollBar')
    number = None
    try:
        number = mac.attr(node, 'AXValue')
        get = mac.cf.CFNumberGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        result = C.c_double()
        assert number and get(number, 13, C.byref(result)), 'Expected numeric scrollbar value'
        return result.value
    finally:
        if number:
            mac.release(number)
        mac.release(node)


def exercise(mac, expected, output, observations):
    mac.wait_text(TITLE, 'A little context goes a long way')
    mac.press(TITLE, 'Lists, trees & tables')
    mac.press(TITLE, 'Scrollbars')
    reveal_gallery_control(mac, BAR, 'AXScrollBar', scroll_fraction=.24)
    focus_gallery_control(mac, BAR, 'AXScrollBar')
    expect_focus(mac, BAR, 'AXScrollBar')
    before = value(mac)
    mac.key(125)  # Actual native scrollbar keyboard action.
    deadline = time.monotonic() + 5
    while value(mac) <= before and time.monotonic() < deadline:
        time.sleep(.025)
    scrolled = value(mac)
    assert scrolled > before, ('Down did not scroll', before, scrolled)
    observations.update(before=before, scrolled=scrolled)
    # Begin from the opposite explicit choice so the snapshot must change mode.
    mac.press(TITLE, 'Always visible' if expected == 'Auto_hide' else 'While scrolling')
    mac.wait_text(TITLE, 'Using your explicit choice.')
    mac.press(TITLE, 'Use system preference')
    status = ('System preference applied: hide when idle.' if expected == 'Auto_hide'
              else 'System preference applied: always visible.')
    mac.wait_text(TITLE, status)
    after = value(mac)
    observations['after_preference'] = after
    assert abs(after - scrolled) < .01, ('Applying preference moved the viewport', scrolled, after)
    # The native range remains usable after the async mode replacement.
    focus_gallery_control(mac, BAR, 'AXScrollBar')
    mac.key(119)  # End.
    deadline = time.monotonic() + 5
    while value(mac) <= after and time.monotonic() < deadline:
        time.sleep(.025)
    observations['end'] = value(mac)
    assert observations['end'] > after, observations
    mac.key(115)  # Home.
    deadline = time.monotonic() + 5
    while abs(value(mac)) > .01 and time.monotonic() < deadline:
        time.sleep(.025)
    assert abs(value(mac)) < .01, 'Home did not restore the viewport start'
    reveal_gallery_control(mac, 'Use system preference', 'AXButton')
    screenshot(mac, output / 'applied.png', title=TITLE)
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A little context goes a long way')
    mac.press(TITLE, 'Lists, trees & tables')
    # Bonsai retains this page's published model across branch deactivation.
    # A fresh scoped request must work after returning; an old completed status
    # need not reset, and this does not simulate a delayed in-flight reply.
    mac.wait_text(TITLE, status)
    mac.press(TITLE, 'On hover')
    mac.wait_text(TITLE, 'Using your explicit choice.')
    mac.press(TITLE, 'Use system preference')
    mac.wait_text(TITLE, status)
    observations['reactivation'] = 'retained published status, explicit choice, fresh successful read'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--executable', type=Path)
    args = parser.parse_args()
    Mac.require_accessibility()
    root = Path(__file__).resolve().parent.parent
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    executable = (args.executable or root / '_build/default/examples/gallery/main.exe').resolve()
    with executable.open('rb') as binary:
        digest = hashlib.file_digest(binary, 'sha256').hexdigest()
    report = {'platform': platform.platform(), 'binary_sha256': digest,
              'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
              'cases': []}
    oracle = output / 'oracle'
    with (output / 'oracle-build.log').open('w') as log:
        subprocess.run(['xcrun', 'swiftc', '-module-cache-path', str(output / 'modules'),
                        str(root / 'scripts/macos_scrollbar_preference.swift'), '-o', str(oracle)],
                       stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
    try:
        for name, override in [('current', None), ('legacy', 'Always'), ('overlay', 'WhenScrolling')]:
            directory = output / name
            directory.mkdir()
            flags = ['-AppleShowScrollBars', override] if override else []
            oracle_result = json.loads(subprocess.check_output(
                [str(oracle), *flags], text=True, timeout=10))
            expected = oracle_result['settled']
            assert expected in ('Auto_hide', 'Always_visible'), expected
            if override:
                assert expected == ('Always_visible' if override == 'Always' else 'Auto_hide')
            case = {'case': name, 'argument_default': override, 'appkit': expected,
                    'oracle': oracle_result}
            report['cases'].append(case)
            with (directory / 'application.log').open('w') as log:
                child = subprocess.Popen([str(executable), '--trace-windows', *flags], cwd=root,
                                         stdout=log, stderr=subprocess.STDOUT)
                mac = None
                try:
                    mac = Mac(child.pid, child)
                    exercise(mac, expected, directory, case)
                    mac.close(TITLE)
                    assert child.wait(timeout=15) == 0
                    case['result'] = 'pass'
                finally:
                    if mac:
                        mac.release(mac.app)
                    if child.poll() is None:
                        child.terminate()
                        try:
                            child.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            child.kill()
                            child.wait()
                    case['child_exit'] = child.returncode
        report['result'] = 'pass'
    except BaseException as error:
        report.update(result='fail', error=repr(error))
        raise
    finally:
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print('GPUIO_SCROLLBAR_PREFERENCE_OK: AppKit current/legacy/overlay, native keys, position retention, remount')


if __name__ == '__main__':
    main()
