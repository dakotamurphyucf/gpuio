#!/usr/bin/env python3
"""Drive the independent public document profile through the native gallery."""
import argparse
import ctypes as C
import json
from pathlib import Path
import platform
import re
import signal
import subprocess
import time

from package_macos_reference import digest
from mac_input_source import foreground_keys
from test_agent_chat import Mac
from test_canvas import screenshot
from test_gallery import (TITLE, GalleryMouse, activate, element_rect, expect_focus,
                          focus_gallery_control, raise_gallery, reveal_document_control,
                          reveal_gallery_control, wait_absent, wait_for_resource_cleanup)


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
    expected = []
    def interrupted(signum, _frame):
        raise RuntimeError(f'Document profile test interrupted: {signum}')
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGALRM, interrupted)
    signal.alarm(150)
    try:
        with (output / 'application.log').open('w') as log:
            child = subprocess.Popen([str(args.executable.resolve()), '--trace-document-profile'],
                                     stdout=log, stderr=subprocess.STDOUT)
            mac = Mac(child.pid, child)
            mac.wait_text(TITLE, 'A little context goes a long way')
            raise_gallery(mac)
            foreground_keys(mac)
            mac.press(TITLE, 'Markdown & code')
            mac.wait_text(TITLE, 'Markdown preview')
            def toggle(label):
                boolean = mac.cf.CFBooleanGetValue
                boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
                def checked():
                    node = mac.wait_find(TITLE, label, 'AXCheckBox')
                    value = mac.attr(node, 'AXValue')
                    try:
                        assert value, ('Missing checkbox value', label)
                        return bool(boolean(value))
                    finally:
                        if value:
                            mac.release(value)
                        mac.release(node)
                before = checked()
                activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
                deadline = time.monotonic() + 5
                while checked() == before:
                    assert time.monotonic() < deadline, ('Checkbox did not change', label)
                    time.sleep(.025)
            def events():
                text = (output / 'application.log').read_text()
                records = re.findall(r'GALLERY_DOCUMENT_PROFILE revision=(\d+) signal=\(Data (\w+)\)', text)
                assert 'signal=(Failed' not in text, text
                return records
            def accepted(event):
                expected.append(event)
                mac.wait_text(TITLE, '(Data ' + event + ')')
                deadline = time.monotonic() + 5
                while [entry[1] for entry in events()] != expected:
                    assert time.monotonic() < deadline, (events(), expected)
                    time.sleep(.025)
            toggle('Native document profile')
            for label, event in [('Inspect highlighted code', 'Inspect_code'),
                                 ('Summarize native table', 'Summarize_table')]:
                reveal_document_control(mac, label)
                reveal_gallery_control(mac, label, 'AXButton')
                focus_gallery_control(mac, label, 'AXButton')
                # GPUI installs a focused element's keyboard handlers at paint;
                # the AX focus acknowledgement alone precedes that frame.
                time.sleep(.15)
                if not expected:
                    screenshot(mac, output / 'first-action-focused.png', title=TITLE)
                assert [entry[1] for entry in events()] == expected, 'Focus activated a profile action'
                mac.key(36)
                accepted(event)
                report['checks'].append({'case': 'keyboard-action', 'label': label})

            reveal_gallery_control(mac, 'Native review controls', 'AXGroup')
            focus_gallery_control(mac, 'Review details', 'AXButton')
            time.sleep(.15)
            assert [entry[1] for entry in events()] == expected
            mac.key(36)
            accepted('Open_badge')
            # Native Tab must reach the block plugin without activating it.
            mac.key(48)
            expect_focus(mac, 'Open review card', 'AXButton')
            time.sleep(.15)
            assert [entry[1] for entry in events()] == expected
            mac.key(36)
            accepted('Open_card')
            report['checks'].append({'case': 'inline-to-block-tab-and-keyboard'})

            toggle('Amber code highlights')
            reveal_gallery_control(mac, 'Native review controls', 'AXGroup')
            node = mac.wait_find(TITLE, 'Open review card', 'AXButton')
            try:
                x, y, w, h = element_rect(mac, node)
            finally:
                mac.release(node)
            point = (x+w/2, y+h/2)
            mouse = GalleryMouse(mac)
            mouse.check_owner(point)
            mouse.send(5, point)
            try:
                mouse.send(1, point)
            finally:
                mouse.send(2, point)
            accepted('Open_card')
            report['checks'].append({'case': 'changed-properties-native-pointer'})
            time.sleep(.15)
            screenshot(mac, output / 'native-controls.png', title=TITLE)

            toggle('Native document profile')
            wait_absent(mac, 'Review details', 'AXButton')
            wait_absent(mac, 'Open review card', 'AXButton')
            toggle('Native document profile')
            reveal_gallery_control(mac, 'Native review controls', 'AXGroup')
            focus_gallery_control(mac, 'Review details', 'AXButton')
            time.sleep(.15)
            mac.key(36)
            accepted('Open_badge')
            report['checks'].append({'case': 'profile-remove-and-remount'})
            mac.press(TITLE, 'Runtime & windows')
            wait_for_resource_cleanup(mac)
            mac.wait_text(TITLE, 'Documents: 0')
            mac.wait_text(TITLE, 'Registered source bytes: 0')
            assert [entry[1] for entry in events()] == expected
            report['checks'].append({'case': 'page-departure-source-release', 'events': events()})
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
            report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        signal.alarm(0)
        if child and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
        if mac:
            mac.release(mac.app)
        report['child_reaped'] = child is None or child.poll() is not None
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print('GPUIO_DOCUMENT_PROFILE_OK: native actions/plugins, navigation, updates, remount and source release')


if __name__ == '__main__':
    main()
