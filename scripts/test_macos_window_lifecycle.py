#!/usr/bin/env python3
"""Public window commands and native custom chrome on a real macOS desktop.

No OS preferences, clipboard, input-source or VoiceOver changes. Each invocation
owns one application and always closes/reaps it, including failed transitions.
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
from test_gallery import (TITLE, SECOND, GalleryMouse, element_rect, expect_field,
                          raise_gallery)
from test_macos_focused_input import focus, DRAFT_LABEL
from test_macos_text_selection import Selection, utf16

DRAFT = 'Keep this λ🙂 draft across window transitions'


def wait_for(operation, predicate, description, timeout=12):
    deadline = time.monotonic() + timeout
    while True:
        value = operation()
        if predicate(value):
            return value
        if time.monotonic() >= deadline:
            raise RuntimeError(f'{description}: {value!r}')
        time.sleep(.05)


def boolean(mac, node, name):
    value = mac.attr(node, name)
    assert value, f'Missing {name}'
    get = mac.cf.CFBooleanGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
    try:
        return bool(get(value))
    finally:
        mac.release(value)


def geometry(mac):
    window = mac.window(TITLE)
    assert window
    try:
        return element_rect(mac, window)
    finally:
        mac.release(window)


def settled_geometry(mac):
    last, since = geometry(mac), time.monotonic()
    deadline = since + 8
    while time.monotonic() < deadline:
        time.sleep(.05)
        current = geometry(mac)
        if current != last:
            last, since = current, time.monotonic()
        elif time.monotonic()-since >= .4:
            return current
    raise RuntimeError('Window geometry did not settle')


def fixture_geometry(display):
    """Leave room for the later right/down drag and AppKit borders."""
    x, y, width, height = display
    width, height = min(1120., width-100.), min(820., height-140.)
    if width < 960 or height < 650:
        raise RuntimeError(f'Window fixture needs at least a 1060x790 display: {display}')
    return x+20., y+60., width, height


def prepare_window(mac, report):
    # Centered hosted windows put the custom Fullscreen button under notification
    # banners. Move only our window before establishing any transition baseline.
    # This reduces overlap; ready_pointer still refuses any actual occlusion.
    class Point(C.Structure):
        _fields_ = [('x', C.c_double), ('y', C.c_double)]

    class Rect(C.Structure):
        _fields_ = [('origin', Point), ('size', Point)]

    main = mac.cg.CGMainDisplayID
    main.restype, main.argtypes = C.c_uint32, []
    bounds = mac.cg.CGDisplayBounds
    bounds.restype, bounds.argtypes = Rect, [C.c_uint32]
    display = bounds(main())
    screen = (display.origin.x, display.origin.y, display.size.x, display.size.y)
    target = fixture_geometry(screen)
    placement = {'display': screen, 'before': geometry(mac), 'requested': target}
    report['fixture_placement'] = placement
    create = mac.ax.AXValueCreate
    create.restype, create.argtypes = C.c_void_p, [C.c_int, C.c_void_p]
    window = mac.window(TITLE)
    assert window
    try:
        for attribute, kind, pair in [('AXSize', 2, target[2:]),
                                     ('AXPosition', 1, target[:2])]:
            data = Point(*pair)
            value = create(kind, C.byref(data))
            assert value
            try:
                mac.set(window, attribute, value)
            finally:
                mac.release(value)
            indices = slice(2, 4) if kind == 2 else slice(0, 2)
            wait_for(lambda: geometry(mac)[indices],
                     lambda actual: all(abs(a-b) < 2 for a, b in zip(actual, pair)),
                     f'Fixture {attribute} did not settle')
    finally:
        mac.release(window)
    placement['actual'] = settled_geometry(mac)


def drag(mouse, start, finish):
    mouse.check_owner(start)
    mouse.send(5, start)
    mouse.send(1, start)
    try:
        time.sleep(.1)
        for step in range(1, 13):
            point = tuple(a + (b-a)*step/12 for a, b in zip(start, finish))
            mouse.send(6, point)
            time.sleep(.025)
    finally:
        mouse.send(2, finish)


def point_for(mac, label, role, *, deadline=None):
    node = (mac.wait_find(TITLE, label, role) if deadline is None
            else mac.find(TITLE, label, role, deadline=deadline))
    if not node:
        return None
    try:
        x, y, w, h = element_rect(mac, node)
        return x+w/2, y+h/2
    finally:
        mac.release(node)


def ready_pointer(mac, mouse, label, role, report, timeout=12):
    """Wait for a current, stable, foreground target; never click an occluder.

    Raising the app requests an asynchronous OS transition. Re-read the AX
    position after it settles rather than trusting one post-drag observation.
    Retain bounded ownership transitions, without logging other apps' UI text.
    """
    started = time.monotonic()
    observation = {'label': label, 'ready': False, 'transitions': [], 'dropped': 0}
    report.setdefault('pointer_readiness', []).append(observation)
    names = {}
    previous, stable_since = None, started
    raise_gallery(mac)
    while True:
        point = point_for(mac, label, role, deadline=started+timeout)
        target = (mouse.owner_at(point) if point is not None else
                  {'expected_pid': mac.pid, 'actual_pid': 0, 'hit_status': None,
                   'pid_status': None, 'found': False, 'role': None, 'subrole': None})
        foreground = boolean(mac, mac.app, 'AXFrontmost')
        state = {'point': point, 'foreground': foreground, **target}
        now = time.monotonic()
        if state != previous:
            previous, stable_since = state, now
            pid = target['actual_pid']
            if pid and pid not in names:
                try:
                    process = subprocess.run(['ps', '-p', str(pid), '-o', 'comm='],
                                             capture_output=True, text=True, timeout=1)
                    names[pid] = Path(process.stdout.strip()).name or 'unavailable'
                except subprocess.TimeoutExpired:
                    names[pid] = 'lookup timed out'
            if len(observation['transitions']) < 24:
                observation['transitions'].append(
                    {'elapsed_s': round(now-started, 4), 'process': names.get(pid), **state})
            else:
                observation['dropped'] += 1
        valid = (foreground and target['hit_status'] == 0 and target['found']
                 and target['pid_status'] == 0 and target['actual_pid'] == mac.pid
                 and target['subrole'] not in ('AXCloseButton', 'AXMinimizeButton', 'AXZoomButton'))
        if now-started >= timeout:
            observation['elapsed_s'] = round(now-started, 4)
            raise RuntimeError(f'Pointer target did not become ready: {label}: {state}')
        if valid and now-stable_since >= .1:
            mouse.check_owner(point)  # Recheck immediately before the caller posts input.
            observation.update(ready=True, elapsed_s=round(now-started, 4), point=point)
            return point
        time.sleep(.025)


def double_click(mouse, point):
    mouse.check_owner(point)
    set_integer = mouse.mac.cg.CGEventSetIntegerValueField
    set_integer.restype, set_integer.argtypes = None, [C.c_void_p, C.c_int, C.c_longlong]
    for count in (1, 2):
        for kind in (1, 2):
            event = mouse.create(None, kind, mouse.Point(*point), 0)
            assert event
            try:
                set_integer(event, 1, count)
                mouse.post(0, event)
            finally:
                mouse.mac.release(event)
            time.sleep(.04)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/gallery/main.exe'))
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--custom-chrome', action='store_true')
    args = parser.parse_args()
    Mac.require_accessibility()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'platform': platform.platform(), 'checks': [],
              'custom_chrome': args.custom_chrome,
              'executable_sha256': digest(args.executable),
              'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], text=True))}
    child = mac = node = window = None
    def interrupted(signum, _frame):
        raise RuntimeError(f'Window lifecycle interrupted: {signum}')
    signals = (signal.SIGTERM, signal.SIGALRM)
    handlers = {sig: signal.signal(sig, interrupted) for sig in signals}
    signal.alarm(150)
    try:
        with (output / 'application.log').open('w') as log:
            command = [str(args.executable.resolve())]
            if args.custom_chrome:
                command.append('--custom-chrome')
            child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
            mac = Mac(child.pid, child)
            foreground_keys(mac)
            mac.wait_text(TITLE, 'A little context goes a long way')
            prepare_window(mac, report)
            mac.press(TITLE, 'Runtime & windows')
            focus(mac, TITLE, DRAFT_LABEL)
            mac.field(TITLE, DRAFT_LABEL, 'AXTextField', DRAFT)
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT)
            node = mac.wait_find(TITLE, DRAFT_LABEL, 'AXTextField')
            window = mac.window(TITLE)
            selection = Selection(mac)
            offset = utf16(DRAFT[:DRAFT.index('λ🙂')])
            selection.set(node, offset, utf16('λ🙂'))
            selection.expect(node, 'λ🙂', offset, utf16('λ🙂'))
            false = C.c_void_p.in_dll(mac.cf, 'kCFBooleanFalse').value

            def retained():
                expect_field(mac, TITLE, DRAFT_LABEL, DRAFT)
                selection.expect(node, 'λ🙂', offset, utf16('λ🙂'))

            for cycle in range(3):
                assert not boolean(mac, window, 'AXMinimized')
                mac.press(TITLE, 'Minimize this window')
                wait_for(lambda: boolean(mac, window, 'AXMinimized'), bool, 'Did not minimize')
                # The same retained AXWindow must accept native restoration.
                mac.set(window, 'AXMinimized', false)
                wait_for(lambda: boolean(mac, window, 'AXMinimized'), lambda x: not x,
                         'Did not restore')
                raise_gallery(mac)
                mac.wait_text(TITLE, 'Minimize requested')
                retained()
                report['checks'].append({'case': 'minimize-restore-retains-window-and-editor',
                                         'cycle': cycle+1})

            if args.custom_chrome:
                mouse = GalleryMouse(mac)
                before = settled_geometry(mac)
                start = point_for(mac, 'GPUIO  /  COMPONENT STUDIO', 'AXStaticText')
                finish = (start[0]+35, start[1]+25)
                drag(mouse, start, finish)
                after = wait_for(lambda: settled_geometry(mac),
                                 lambda g: abs(g[0]-before[0]) > 10 and abs(g[1]-before[1]) > 10,
                                 'Title-bar drag did not move window')
                assert abs(after[2]-before[2]) < 2 and abs(after[3]-before[3]) < 2
                retained()
                report['checks'].append({'case': 'custom-title-bar-native-move',
                                         'before': before, 'after': after})

                # The child button must perform its action, without moving the
                # outer window on an ordinary native pointer click.
                point = ready_pointer(mac, mouse, 'Fullscreen', 'AXButton', report)
                mouse.send(5, point)
                mouse.send(1, point)
                mouse.send(2, point)
                wait_for(lambda: boolean(mac, window, 'AXFullScreen'), bool, 'Did not enter fullscreen')
                mac.wait_text(TITLE, 'Leave fullscreen')
                retained()
                # Wait for AppKit's Space animation to settle before capturing
                # and issuing the next real transition.
                time.sleep(1)
                screenshot(mac, output / 'fullscreen.png', title=TITLE)
                mac.press(TITLE, 'Leave fullscreen')
                wait_for(lambda: boolean(mac, window, 'AXFullScreen'), lambda x: not x,
                         'Did not leave fullscreen')
                mac.release(mac.wait_find(TITLE, 'Fullscreen', 'AXButton'))
                time.sleep(1)
                restored = wait_for(lambda: geometry(mac),
                                    lambda g: all(abs(a-b) < 3 for a, b in zip(g, after)),
                                    'Fullscreen did not restore prior window geometry')
                retained()
                report['checks'].append({'case': 'title-bar-child-fullscreen-and-restore',
                                         'restored': restored})

                # macOS resize uses AppKit's native border. Custom Resize regions
                # are intentionally inert on this backend.
                before = geometry(mac)
                start = (before[0]+before[2]-2, before[1]+before[3]-2)
                finish = (start[0]-70, start[1]-45)
                drag(mouse, start, finish)
                after = wait_for(lambda: settled_geometry(mac),
                                 lambda g: g[2] < before[2]-30 and g[3] < before[3]-20,
                                 'Native border did not resize')
                retained()
                report['checks'].append({'case': 'appkit-border-resize',
                                         'before': before, 'after': after})
                screenshot(mac, output / 'custom-window.png', title=TITLE)

                preference = subprocess.run(['defaults', 'read', '-g', 'AppleActionOnDoubleClick'],
                                            text=True, capture_output=True)
                if preference.returncode:
                    assert 'does not exist' in preference.stderr, preference.stderr
                    action = 'Default (zoom)'
                else:
                    action = preference.stdout.strip()
                report['double_click_preference'] = action
                # Qualify the existing preference without changing it. A machine
                # using Do Nothing/Fill retains that policy and records the gap.
                if action in ('Default (zoom)', 'Maximize', 'Minimize'):
                    before = settled_geometry(mac)
                    double_click(mouse, point_for(mac, 'GPUIO  /  COMPONENT STUDIO', 'AXStaticText'))
                    if action == 'Minimize':
                        wait_for(lambda: boolean(mac, window, 'AXMinimized'), bool,
                                 'Title-bar double click did not minimize')
                        mac.set(window, 'AXMinimized', false)
                        wait_for(lambda: boolean(mac, window, 'AXMinimized'), lambda x: not x,
                                 'Title-bar minimize did not restore')
                        raise_gallery(mac)
                    else:
                        wait_for(lambda: settled_geometry(mac),
                                 lambda g: abs(g[2]-before[2]) > 20 or abs(g[3]-before[3]) > 20,
                                 'Title-bar double click did not zoom')
                        double_click(mouse, point_for(mac, 'GPUIO  /  COMPONENT STUDIO', 'AXStaticText'))
                        wait_for(lambda: settled_geometry(mac),
                                 lambda g: all(abs(a-b) < 3 for a, b in zip(g, before)),
                                 'Title-bar zoom did not restore')
                    retained()
                    report['checks'].append({'case': 'title-bar-double-click', 'action': action})
                else:
                    report['double_click_unqualified_reason'] = 'Existing None/Fill preference left unchanged'

            # Re-focus the same native editor after window operations and prove
            # its actual selected range and undo stack still work.
            focus(mac, TITLE, DRAFT_LABEL)
            mac.key(51)
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT.replace('λ🙂', ''))
            mac.key(6, flags=1 << 20)
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT)
            report['checks'].append({'case': 'native-selection-delete-and-undo-after-transitions'})
            mac.press(TITLE, 'New window')
            mac.wait_text(SECOND, 'A little context goes a long way')
            mac.close(SECOND)
            raise_gallery(mac)
            expect_field(mac, TITLE, DRAFT_LABEL, DRAFT)
            report['checks'].append({'case': 'second-window-close-retains-original'})
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
            if window:
                mac.release(window)
            mac.release(mac.app)
        report['child_reaped'] = child is None or child.poll() is not None
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print('GPUIO_WINDOW_LIFECYCLE_OK: actual window commands and retained native editing')


if __name__ == '__main__':
    main()
