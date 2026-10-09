#!/usr/bin/env python3
"""Exercise the public OCaml context-menu adapter through real AppKit input."""
import argparse
import ctypes as C
import subprocess
import tempfile
import time
from pathlib import Path
from test_agent_chat import Mac
from test_gallery import GalleryMouse, element_rect
from mac_clipboard import preserved_clipboard

TITLE = 'GPUIO menus'


def popup(mac, *, title=TITLE):
    window = mac.window(title)
    if not window:
        return None
    children = mac.children(window)
    try:
        for child in children:
            if mac.text(child, 'AXRole') == 'AXMenu':
                return mac.retain(child)
    finally:
        for child in children:
            mac.release(child)
        mac.release(window)
    return None


def wait_popup(mac, opened=True, *, title=TITLE):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if mac.child.poll() is not None:
            raise RuntimeError('Menu example exited during tracking')
        menu = popup(mac, title=title)
        if bool(menu) == opened:
            return menu
        if menu:
            mac.release(menu)
        time.sleep(.03)
    raise RuntimeError(f'AppKit popup did not become opened={opened}')


def focus_draft(mac):
    node = mac.wait_find(TITLE, 'Draft', 'AXTextField')
    try:
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)


def open_keyboard(mac):
    focus_draft(mac)
    mac.key(109, 1 << 17)  # Shift-F10.
    return wait_popup(mac)


def enabled(mac, node):
    value = mac.attr(node, 'AXEnabled')
    if not value:
        raise RuntimeError('Native menu item has no enabled state')
    read = mac.cf.CFBooleanGetValue
    read.restype, read.argtypes = C.c_bool, [C.c_void_p]
    try:
        return read(value)
    finally:
        mac.release(value)


def exercise(mac, clipboard):
    mac.wait_text(TITLE, 'Total runs: 0')
    mac.set(mac.app, 'AXFrontmost', mac.true)
    window = mac.window(TITLE)
    try:
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    menu = open_keyboard(mac)
    try:
        children = mac.children(menu)
        try:
            assert [mac.text(row, 'AXTitle') for row in children] == [
                'Run (0)', 'Copy selection', '', 'More actions']
            assert enabled(mac, children[0])
            assert not enabled(mac, children[1]), 'Empty editor Copy must be disabled'
        finally:
            for row in children:
                mac.release(row)
    finally:
        mac.release(menu)
    mac.key(125)
    mac.key(36)
    wait_popup(mac, False)
    mac.wait_text(TITLE, 'Total runs: 1')
    print('NATIVE_POPUP_KEYBOARD_OK: AppKit rows, disabled Copy, Shift-F10/Enter invocation', flush=True)

    node = mac.wait_find(TITLE, 'Draft', 'AXTextField')
    try:
        x, y, width, height = element_rect(mac, node)
    finally:
        mac.release(node)
    point = x + width - 5, y + height / 2
    mouse = GalleryMouse(mac)
    mouse.check_owner(point)
    for kind in (3, 4):  # Physical right button down/up in owned content.
        event = mouse.create(None, kind, mouse.Point(*point), 1)
        assert event
        try:
            mouse.post(0, event)
        finally:
            mac.release(event)
    menu = wait_popup(mac)
    window = mac.window(TITLE)
    try:
        mx, my, mw, mh = element_rect(mac, menu)
        wx, wy, ww, wh = element_rect(mac, window)
        assert (mx < wx or my < wy or mx + mw > wx + ww or my + mh > wy + wh), (
            'Expected an OS popup beyond the application window',
            (mx, my, mw, mh), (wx, wy, ww, wh))
        print('NATIVE_POPUP_OUTSIDE_WINDOW_OK', (mx, my, mw, mh), (wx, wy, ww, wh), flush=True)
    finally:
        mac.release(menu)
        mac.release(window)
    mac.key(53)
    wait_popup(mac, False)
    mac.wait_text(TITLE, 'Total runs: 1')

    menu = open_keyboard(mac)
    try:
        rows = mac.children(menu)
        try:
            # Move to the submenu by native typeahead; the prior physical cursor
            # position can otherwise affect the initial highlighted row.
            mac.key(46)  # m: More actions.
            deadline = time.monotonic() + 5
            while True:
                selected = mac.children(menu, 'AXSelectedChildren')
                try:
                    names = [mac.text(row, 'AXTitle') for row in selected]
                finally:
                    for row in selected:
                        mac.release(row)
                if names == ['More actions']:
                    break
                if time.monotonic() > deadline:
                    raise RuntimeError(f'Native submenu selection not observed: {names}')
                time.sleep(.03)
            mac.key(124)
            children = mac.children(rows[3])
            try:
                assert len(children) == 1
                submenu = children[0]
                mac.key(15)  # r: Run.
                deadline = time.monotonic() + 5
                while True:
                    selected = mac.children(submenu, 'AXSelectedChildren')
                    try:
                        names = [mac.text(row, 'AXTitle') for row in selected]
                    finally:
                        for row in selected:
                            mac.release(row)
                    if names == ['Run (1)']:
                        break
                    if time.monotonic() > deadline:
                        raise RuntimeError(f'Native child selection not observed: {names}')
                    time.sleep(.03)
                mac.key(36)
            finally:
                for child in children:
                    mac.release(child)
        finally:
            for row in rows:
                mac.release(row)
    finally:
        mac.release(menu)
    wait_popup(mac, False)
    mac.wait_text(TITLE, 'Total runs: 2')
    print('NATIVE_POPUP_REPEAT_SUBMENU_OK: Escape, repeat open, nested keyboard action', flush=True)

    mac.field(TITLE, 'Draft', 'AXTextField', 'Native popup selection')
    mac.key(0, 1 << 20)  # Cmd-A selects in the existing native editor.
    menu = open_keyboard(mac)
    try:
        rows = mac.children(menu)
        try:
            assert enabled(mac, rows[1]), 'Selected editor Copy must be enabled'
            mac.key(8)  # c: Copy selection.
            deadline = time.monotonic() + 5
            while True:
                selected = mac.children(menu, 'AXSelectedChildren')
                try:
                    names = [mac.text(row, 'AXTitle') for row in selected]
                finally:
                    for row in selected:
                        mac.release(row)
                if names == ['Copy selection']:
                    break
                if time.monotonic() > deadline:
                    raise RuntimeError(f'Native Copy selection not observed: {names}')
                time.sleep(.03)
            mac.key(36)
        finally:
            for row in rows:
                mac.release(row)
    finally:
        mac.release(menu)
    wait_popup(mac, False)
    deadline = time.monotonic() + 5
    while clipboard.text() != 'Native popup selection':
        if time.monotonic() > deadline:
            raise RuntimeError('Native Copy did not use the captured editor')
        time.sleep(.03)
    assert mac.field(TITLE, 'Draft', 'AXTextField') == 'Native popup selection'
    print('NATIVE_POPUP_EDIT_OK: native editor selection and Copy; clipboard preserved', flush=True)

    checkbox = mac.wait_find(TITLE, 'Enable the outer Run command', 'AXCheckBox')
    try:
        mac.perform(checkbox, 'AXPress')
    finally:
        mac.release(checkbox)
    # Wait for the ordinary control to reflect accepted availability first.
    deadline = time.monotonic() + 10
    while True:
        button = mac.wait_find(TITLE, 'Run (2)', 'AXButton')
        try:
            is_enabled = enabled(mac, button)
        finally:
            mac.release(button)
        if not is_enabled:
            break
        if time.monotonic() > deadline:
            raise RuntimeError('Run command did not become disabled')
        time.sleep(.03)
    menu = open_keyboard(mac)
    try:
        rows = mac.children(menu)
        try:
            assert not enabled(mac, rows[0])
        finally:
            for row in rows:
                mac.release(row)
    finally:
        mac.release(menu)
    mac.key(53)
    wait_popup(mac, False)
    mac.wait_text(TITLE, 'Total runs: 2')
    mac.press(TITLE, 'Close window')


def exercise_lifecycle(mac, mode):
    mac.wait_text(TITLE, 'Total runs: 0')
    mac.set(mac.app, 'AXFrontmost', mac.true)
    window = mac.window(TITLE)
    try:
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    if mode in ('replace', 'hide', 'disable', 'modal'):
        mac.field(TITLE, 'Draft', 'AXTextField', 'Retained popup draft')
    mac.press(TITLE, 'Arm popup transition')
    menu = open_keyboard(mac)
    print('NATIVE_POPUP_TRACKING_BEFORE_LIFECYCLE', mode, flush=True)
    try:
        if mode == 'close':
            assert mac.child.wait(timeout=8) == 0
            return
        if mode == 'retire':
            wait_popup(mac, False)
            mac.wait_text(TITLE, 'Popup owner retired')
        elif mode in ('replace', 'hide', 'disable', 'modal'):
            wait_popup(mac, False)
            mac.wait_text(TITLE, 'Restore popup owner')
            mac.wait_text(TITLE, 'Total runs: 0')
            if mode == 'replace':
                replacement = open_keyboard(mac)
                try:
                    rows = mac.children(replacement)
                    try:
                        assert [mac.text(row, 'AXTitle') for row in rows] == [
                            'Replacement definition', 'Run (0)']
                        assert not enabled(mac, rows[0])
                    finally:
                        for row in rows:
                            mac.release(row)
                finally:
                    mac.release(replacement)
                mac.key(53)
                wait_popup(mac, False)
            elif mode == 'disable':
                draft = mac.wait_find(TITLE, 'Draft', 'AXTextField')
                try:
                    assert not enabled(mac, draft), 'Ancestor must disable its editor'
                finally:
                    mac.release(draft)
            elif mode == 'hide':
                draft = mac.find(TITLE, 'Draft', 'AXTextField')
                try:
                    assert not draft, 'Hidden editor must leave the accessible tree'
                finally:
                    if draft:
                        mac.release(draft)
            if mode == 'modal':
                # The modal must receive actual keyboard focus after AppKit
                # cancellation, not merely appear in the accessibility tree.
                mac.key(48)  # Tab to the dialog's sole control.
                mac.key(36)
            else:
                mac.press(TITLE, 'Restore popup owner')
            deadline = time.monotonic() + 5
            while True:
                restore = mac.find(TITLE, 'Restore popup owner', 'AXButton')
                if not restore:
                    break
                mac.release(restore)
                if time.monotonic() > deadline:
                    raise RuntimeError('Owner restoration was not published')
                time.sleep(.03)
            # Observing the original menu again fences the restoration, and
            # actually invoking it proves the application-wide lease was freed.
            restored = open_keyboard(mac)
            try:
                rows = mac.children(restored)
                try:
                    assert [mac.text(row, 'AXTitle') for row in rows] == [
                        'Run (0)', 'Copy selection', '', 'More actions']
                finally:
                    for row in rows:
                        mac.release(row)
                mac.key(15)  # r: Run.
                deadline = time.monotonic() + 5
                while True:
                    selected = mac.children(restored, 'AXSelectedChildren')
                    try:
                        names = [mac.text(row, 'AXTitle') for row in selected]
                    finally:
                        for row in selected:
                            mac.release(row)
                    if names == ['Run (0)']:
                        break
                    if time.monotonic() > deadline:
                        raise RuntimeError(f'Restored menu selection not observed: {names}')
                    time.sleep(.03)
                mac.key(36)
            finally:
                mac.release(restored)
            wait_popup(mac, False)
            mac.wait_text(TITLE, 'Total runs: 1')
            assert mac.field(TITLE, 'Draft', 'AXTextField') == 'Retained popup draft'
            print('NATIVE_POPUP_OWNER_RECOVERY_OK:', mode, flush=True)
        else:
            deadline = time.monotonic() + 8
            while True:
                button = mac.wait_find(TITLE, 'Run (0)', 'AXButton')
                try:
                    disabled = not enabled(mac, button)
                finally:
                    mac.release(button)
                if disabled:
                    break
                if time.monotonic() > deadline:
                    raise RuntimeError('Application did not update while menu was tracking')
                time.sleep(.03)
            rows = mac.children(menu)
            try:
                # AppKit owns the original snapshot. Its old enabled row must
                # not invoke the now-retired registry command when selected.
                assert enabled(mac, rows[0])
                mac.key(15)  # r: Run.
                deadline = time.monotonic() + 5
                while True:
                    selected = mac.children(menu, 'AXSelectedChildren')
                    try:
                        names = [mac.text(row, 'AXTitle') for row in selected]
                    finally:
                        for row in selected:
                            mac.release(row)
                    if names == ['Run (0)']:
                        break
                    if time.monotonic() > deadline:
                        raise RuntimeError('Old native command could not be selected')
                    time.sleep(.03)
                mac.key(36)
            finally:
                for row in rows:
                    mac.release(row)
            wait_popup(mac, False)
            checkbox = mac.wait_find(TITLE, 'Enable the outer Run command', 'AXCheckBox')
            try:
                # A later control roundtrip flushes any earlier invocation;
                # checking the old text immediately would permit a false pass.
                mac.perform(checkbox, 'AXPress')
            finally:
                mac.release(checkbox)
            # Phase 1 keeps Run disabled, so observe the checkbox's state edge
            # as the later accepted application publication.
            deadline = time.monotonic() + 5
            while True:
                checkbox = mac.wait_find(TITLE, 'Enable the outer Run command', 'AXCheckBox')
                value = mac.attr(checkbox, 'AXValue')
                mac.release(checkbox)
                try:
                    number = C.c_longlong()
                    read = mac.cf.CFNumberGetValue
                    read.restype, read.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
                    changed = value and read(value, 4, C.byref(number)) and number.value == 0
                finally:
                    if value:
                        mac.release(value)
                if changed:
                    break
                if time.monotonic() > deadline:
                    raise RuntimeError('Later application publication was not observed')
                time.sleep(.03)
            mac.wait_text(TITLE, 'Total runs: 0')
    finally:
        mac.release(menu)
    mac.press(TITLE, 'Close window')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, default=Path('_build/default/examples/menus/main.exe'))
    parser.add_argument('--lifecycle', choices=[
        'close', 'retire', 'invalidate', 'replace', 'hide', 'disable', 'modal'])
    args = parser.parse_args()
    Mac.require_accessibility()
    with tempfile.TemporaryFile(mode='w+') as log:
        arguments = [str(args.binary.resolve()), '--platform-popup']
        if args.lifecycle:
            arguments.append(f'--popup-{args.lifecycle}-test')
        child = subprocess.Popen(arguments, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            if args.lifecycle:
                exercise_lifecycle(mac, args.lifecycle)
            else:
                with preserved_clipboard() as clipboard:
                    exercise(mac, clipboard)
            assert child.wait(timeout=15) == 0
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
            log.seek(0)
            output = log.read()
            print(output, end='')
        if args.lifecycle:
            assert f'GPUIO_POPUP_{args.lifecycle.upper()}_REQUESTED' in output
    if args.lifecycle:
        print('GPUIO_NATIVE_POPUP_LIFECYCLE_OK:', args.lifecycle)
    else:
        print('GPUIO_NATIVE_POPUP_PUBLIC_OK: physical keyboard/pointer, OS bounds, native states, native Copy, clean shutdown')


if __name__ == '__main__':
    main()
