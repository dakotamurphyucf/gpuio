#!/usr/bin/env python3
"""Drive the public metadata-only focus query through actual native shortcuts.

No OS preferences, VoiceOver, input-source or clipboard state are changed.
"""
import argparse
import ctypes as C
import json
from pathlib import Path
import platform
import signal
import subprocess
import time

from mac_input_source import foreground_keys
from package_macos_reference import digest
from test_agent_chat import Mac
from test_canvas import screenshot
from test_gallery import TITLE, SECOND, expect_field, expect_focus
from test_macos_text_selection import Selection, utf16

DRAFT_LABEL = 'Focus inspection draft'
MASKED_LABEL = 'Private focus inspection'
READ_ONLY_LABEL = 'Read-only focus inspection'
DRAFT = 'Inspect λ🙂 without replacing my draft'


def expect_application_focus(mac, title, label):
    """Check AppKit's focused-element route, independently of leaf AXFocused."""
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    target = mac.wait_find(title, label, 'AXTextField')
    actual_description = None
    try:
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            actual = mac.attr(mac.app, 'AXFocusedUIElement')
            try:
                if actual and equal(actual, target):
                    print('AX_APPLICATION_FOCUS_OK', title, label, flush=True)
                    return
                actual_description = ([mac.text(actual, 'AXRole'), mac.text(actual, 'AXTitle')]
                                      if actual else None)
            finally:
                if actual:
                    mac.release(actual)
            time.sleep(.025)
        raise AssertionError(('Application focus does not identify input', title, label, actual_description))
    finally:
        mac.release(target)


def focus(mac, title, label):
    window = mac.window(title)
    assert window
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    node = mac.wait_find(title, label, 'AXTextField')
    try:
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)
    expect_focus(mac, label, 'AXTextField', title=title)
    expect_application_focus(mac, title, label)
    # AX focus can precede the paint that installs native key handlers.
    time.sleep(.15)


def inspect(mac, title, label, owner):
    focus(mac, title, label)
    mac.key(34, flags=(1 << 20) | (1 << 17))  # Command+Shift+I.
    mac.wait_text(title, f'Focused: {owner} · kind: Input · metadata only; no value read')
    expect_focus(mac, label, 'AXTextField', title=title)
    expect_application_focus(mac, title, label)


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
    def interrupted(signum, _frame):
        raise RuntimeError(f'Focus query test interrupted: {signum}')
    signals = (signal.SIGTERM, signal.SIGALRM)
    handlers = {sig: signal.signal(sig, interrupted) for sig in signals}
    signal.alarm(120)
    try:
        with (output / 'application.log').open('w') as log:
            child = subprocess.Popen([str(args.executable.resolve())], stdout=log, stderr=subprocess.STDOUT)
            mac = Mac(child.pid, child)
            foreground_keys(mac)
            mac.wait_text(TITLE, 'A little context goes a long way')
            mac.press(TITLE, 'Runtime & windows')
            mac.wait_text(TITLE, 'Which input owns focus?')
            focus(mac, TITLE, DRAFT_LABEL)
            mac.field(TITLE, DRAFT_LABEL, 'AXTextField', DRAFT)
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT)
            node = mac.wait_find(TITLE, DRAFT_LABEL, 'AXTextField')
            selection = Selection(mac)
            start = utf16(DRAFT[:DRAFT.index('λ🙂')])
            selection.set(node, start, utf16('λ🙂'))
            selection.expect(node, 'λ🙂', start, utf16('λ🙂'))
            inspect(mac, TITLE, DRAFT_LABEL, 'Draft')
            selection.expect(node, 'λ🙂', start, utf16('λ🙂'))
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT)
            mac.key(51)
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT.replace('λ🙂', ''))
            mac.key(6, flags=1 << 20)
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT)
            report['checks'].append({'case': 'draft-identity-selection-focus-and-native-undo'})
            mac.release(node)
            node = None

            inspect(mac, TITLE, MASKED_LABEL, 'Masked value')
            report['checks'].append({'case': 'masked-owner-metadata-without-reading-value'})
            inspect(mac, TITLE, READ_ONLY_LABEL, 'Read-only value')
            mac.key(51)
            expect_field(mac, TITLE, READ_ONLY_LABEL, 'Read-only is still a text input')
            report['checks'].append({'case': 'read-only-owner-remains-noneditable'})
            screenshot(mac, output / 'focus-inspector.png', title=TITLE)

            mac.press(TITLE, 'New window')
            mac.wait_text(SECOND, 'A little context goes a long way')
            mac.press(SECOND, 'Runtime & windows')
            mac.wait_text(SECOND, 'Which input owns focus?')
            focus(mac, SECOND, DRAFT_LABEL)
            mac.field(SECOND, DRAFT_LABEL, 'AXTextField', 'Second window draft')
            inspect(mac, SECOND, DRAFT_LABEL, 'Draft')
            expect_field(mac, SECOND, DRAFT_LABEL, 'Second window draft')
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT)
            mac.close(SECOND)
            inspect(mac, TITLE, MASKED_LABEL, 'Masked value')
            report['checks'].append({'case': 'independent-window-identity-and-close'})

            mac.press(TITLE, 'Presentation')
            mac.wait_text(TITLE, 'A little context goes a long way')
            mac.press(TITLE, 'Runtime & windows')
            mac.wait_text(TITLE, 'Which input owns focus?')
            expect_field(mac, TITLE, DRAFT_LABEL, 'A thought worth exploring')
            inspect(mac, TITLE, DRAFT_LABEL, 'Draft')
            report['checks'].append({'case': 'remounted-native-editor-matches-current-controller'})
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
            report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        signal.alarm(0)
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
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
    print('GPUIO_FOCUSED_INPUT_OK: public metadata query, native editing, window identity and remount')


if __name__ == '__main__':
    main()
