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


def wait_for_tab_exit(mac, observations, *, timeout=10):
    """Await one posted Tab; never resend it or accept a clipped target/window."""
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    start = time.monotonic()
    deadline = start + timeout
    while True:
        node = mac.attr(mac.app, 'AXFocusedUIElement')
        value = None
        try:
            role = mac.text(node, 'AXRole') if node else None
            title = mac.text(node, 'AXTitle') if node else None
            value = mac.attr(node, 'AXFocused') if node else None
            focused = bool(value and boolean(value))
            observations.append({'elapsed_ms': (time.monotonic() - start) * 1000,
                                 'role': role, 'title': title, 'focused': focused})
            assert title != 'Open scroll review', 'Tab entered the clipped profile control'
            if node and role != 'AXWindow' and focused and title != 'Show review end':
                return title
        finally:
            if value:
                mac.release(value)
            if node:
                mac.release(node)
        assert time.monotonic() < deadline, ('Tab did not leave its starting control', observations)
        time.sleep(.03)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/gallery/main.exe'))
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--large-density', action='store_true', help='Exercise clipped nested scrolling with larger gallery type')
    args = parser.parse_args()
    Mac.require_accessibility()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'platform': platform.platform(),
              'executable_sha256': digest(args.executable), 'large_density': args.large_density, 'checks': []}
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
            if args.large_density:
                mac.press(TITLE, 'Comfortable')
                mac.release(mac.wait_find(TITLE, 'Large', 'AXButton'))
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

            # Public plugin-owned viewport: always-visible native buttons offer
            # keyboard alternatives to pointer scrolling. They emit no app event.
            reveal_gallery_control(mac, 'Show review end', 'AXButton')
            focus_gallery_control(mac, 'Show review end', 'AXButton')
            time.sleep(.15)
            mac.key(36)
            # Obtain the clipped native bounds separately from focus. A clipped
            # control must become painted before it is eligible for traversal.
            report['review_geometry'] = []
            def review_visible():
                viewport = mac.wait_find(TITLE, 'Review checklist viewport', 'AXGroup')
                button = mac.find(TITLE, 'Open scroll review', 'AXButton')
                try:
                    vx, vy, vw, vh = element_rect(mac, viewport)
                    if not button:
                        return False, (vx, vy, vw, vh)
                    bx, by, bw, bh = element_rect(mac, button)
                    report['review_geometry'].append({'time': time.monotonic(),
                        'viewport': [vx, vy, vw, vh], 'button': [bx, by, bw, bh]})
                    return (bh > 0 and by >= vy and by+bh <= vy+vh, (vx, vy, vw, vh))
                finally:
                    mac.release(viewport)
                    if button:
                        mac.release(button)
            deadline = time.monotonic()+8
            while not review_visible()[0]:
                assert time.monotonic() < deadline, 'Keyboard did not reveal inner review'
                time.sleep(.05)
            mac.key(48)
            expect_focus(mac, 'Open scroll review', 'AXButton')
            assert [entry[1] for entry in events()] == expected
            time.sleep(.15)
            mac.key(36)
            accepted('Open_card')
            screenshot(mac, output / 'profile-inner-scroll-end.png', title=TITLE)
            report['checks'].append({'case': 'plugin-keyboard-scroll-reveal-and-activation'})

            focus_gallery_control(mac, 'Show review start', 'AXButton')
            time.sleep(.15)
            mac.key(36)
            deadline = time.monotonic()+8
            while review_visible()[0]:
                assert time.monotonic() < deadline, 'Keyboard did not return inner review to start'
                time.sleep(.05)
            report['before_focus_end_visible'] = review_visible()[0]
            # Starting at the end-button, Tab must escape rather than focus the
            # now-clipped target. The next ordinary Tab can enter later content.
            focus_gallery_control(mac, 'Show review end', 'AXButton')
            time.sleep(.15)
            report['after_focus_end_visible'] = review_visible()[0]
            assert not report['after_focus_end_visible'], 'Tab-exit target must start clipped'
            screenshot(mac, output / 'profile-before-tab-exit.png', title=TITLE)
            mac.key(48)
            report['tab_exit_observations'] = []
            try:
                exited_to = wait_for_tab_exit(mac, report['tab_exit_observations'])
            finally:
                review_visible()
                screenshot(mac, output / 'profile-tab-exit.png', title=TITLE)
            assert [entry[1] for entry in events()] == expected
            report['checks'].append({'case': 'plugin-clipped-control-does-not-trap-tab',
                                     'exited_to': exited_to})

            reveal_gallery_control(mac, 'Show review end', 'AXButton')
            visible, (x, y, w, h) = review_visible()
            assert not visible
            point = (x+w/2, y+h/2)
            mouse = GalleryMouse(mac)
            mouse.check_owner(point)
            mouse.send(5, point)
            wheel = mac.cg.CGEventCreateScrollWheelEvent
            wheel.restype, wheel.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
            locate = mac.cg.CGEventSetLocation
            locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]
            event = wheel(None, 0, 1, C.c_int(-350))
            assert event
            try:
                locate(event, GalleryMouse.Point(*point))
                mouse.post(0, event)
            finally:
                mac.release(event)
            deadline = time.monotonic()+8
            while not review_visible()[0]:
                assert time.monotonic() < deadline, 'Native wheel did not reveal inner review'
                time.sleep(.05)
            focus_gallery_control(mac, 'Show review end', 'AXButton')
            time.sleep(.15)
            mac.key(48)
            expect_focus(mac, 'Open scroll review', 'AXButton')
            time.sleep(.15)
            mac.key(36)
            accepted('Open_card')
            report['checks'].append({'case': 'plugin-native-wheel-and-retained-scroll'})

            toggle('Amber code highlights')
            reveal_gallery_control(mac, 'Show review end', 'AXButton')
            assert review_visible()[0], 'Property update reset mounted plugin scroll'
            focus_gallery_control(mac, 'Show review end', 'AXButton')
            time.sleep(.15)
            mac.key(48)
            expect_focus(mac, 'Open scroll review', 'AXButton')
            time.sleep(.15)
            mac.key(36)
            accepted('Open_card')
            report['checks'].append({'case': 'plugin-property-update-retains-offset-and-current-event'})
            toggle('Native document profile')
            wait_absent(mac, 'Show review end', 'AXButton')
            toggle('Native document profile')
            reveal_gallery_control(mac, 'Show review end', 'AXButton')
            assert not review_visible()[0], 'Unmounted plugin retained scroll state'
            report['checks'].append({'case': 'plugin-unmount-releases-scroll-state'})
            mac.press(TITLE, 'Runtime & windows')
            wait_for_resource_cleanup(mac, documents=True)
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
