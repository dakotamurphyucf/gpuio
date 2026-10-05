#!/usr/bin/env python3
"""Exercise the packaged gallery through Launch Services and Notification Center.

Creates only disposable app/file fixtures. Requests permission for Component
Studio explicitly through its UI; never changes global notification settings.
"""
import argparse
import ctypes as C
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import tempfile
import time

from test_agent_chat import Mac
from test_gallery import TITLE, exercise_desktop, expect_enabled, raise_gallery
from test_notifications_macos import Notifications, pids
from desktop_service_fixture import ServiceFixture, LSREGISTER
from test_desktop_links_macos import owned_pids

APP_NAME = 'GPUIO Component Studio'
INITIAL = 'Your preview is ready'
UPDATED = 'Your preview has an update'


def panel_key(mac, key, flags=0):
    # Native AppKit panel shortcuts need the foreground event stream.
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    frontmost = mac.attr(mac.app, 'AXFrontmost')
    try:
        assert frontmost and boolean(frontmost), 'Picker app must own foreground input'
    finally:
        if frontmost:
            mac.release(frontmost)
    post = mac.cg.CGEventPost
    post.restype, post.argtypes = None, [C.c_int, C.c_void_p]
    for down in (True, False):
        event = mac.key_event(None, key, down)
        assert event
        try:
            mac.key_flags(event, flags)
            post(0, event)
        finally:
            mac.release(event)


def go_to_field(mac):
    # Sonoma exposes an unnamed text field in a second, nested sheet.
    # Do not confuse it with the outer file panel's search field.
    def visit(node, sheets=0):
        role = mac.text(node, 'AXRole')
        if role == 'AXTextField' and sheets == 2:
            return mac.retain(node)
        if role in ('AXBrowser', 'AXOutline', 'AXTable'):
            return None
        children = mac.children(node)
        try:
            for child in children:
                found = visit(child, sheets + (role == 'AXSheet'))
                if found:
                    return found
        finally:
            for child in children:
                mac.release(child)
        return None
    root = mac.window(TITLE)
    if not root:
        return None
    try:
        return visit(root)
    finally:
        mac.release(root)


def choose_path(mac, path, *, open_label='Choose represented file',
                accept_label='Represent', expected='Represented file: selected'):
    mac.press(TITLE, open_label)
    mac.release(mac.wait_find(TITLE, 'Cancel', 'AXButton'))
    # The panel can expose AX controls before its native sheet transition ends.
    time.sleep(.5)
    panel_key(mac, 5, (1 << 20) | (1 << 17))  # Command+Shift+G.
    deadline = time.monotonic() + 10
    node = None
    while time.monotonic() < deadline:
        node = go_to_field(mac)
        if node:
            break
        time.sleep(.05)
    assert node, 'Native Go to Folder sheet is absent'
    try:
        value = mac.string(str(path))
        try:
            mac.set(node, 'AXValue', value)
        finally:
            mac.release(value)
    finally:
        mac.release(node)
    panel_key(mac, 36)
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        node = go_to_field(mac)
        if not node:
            break
        mac.release(node)
        time.sleep(.05)
    else:
        raise AssertionError('Go to Folder did not navigate to the fixture')
    mac.release(mac.wait_find(TITLE, path.name, contains=True, search_files=True))
    mac.press(TITLE, accept_label)
    mac.wait_text(TITLE, expected)


def exercise(binary, artifact):
    artifact.mkdir(parents=True, exist_ok=True)
    # Launch Services ignores handlers under the system temporary directory.
    root = Path(tempfile.mkdtemp(prefix='gallery-desktop-', dir=artifact)).resolve()
    bundle = root / (APP_NAME + '.app')
    executable = bundle / 'Contents/MacOS/gpuio-studio'
    executable.parent.mkdir(parents=True)
    shutil.copy2(binary, executable)
    metadata = subprocess.check_output([str(executable), '--print-info-plist'], timeout=10)
    parsed = plistlib.loads(metadata)
    assert parsed['CFBundleIdentifier'] == 'com.gpuio.component-studio'
    assert parsed['CFBundleExecutable'] == executable.name
    assert parsed['CFBundleURLTypes'][0]['CFBundleURLSchemes'] == ['gpuio-studio']
    (bundle / 'Contents/Info.plist').write_bytes(metadata)
    subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', '--identifier',
                    parsed['CFBundleIdentifier'], '--timestamp=none', str(bundle)], check=True, timeout=15)
    subprocess.run([LSREGISTER, '-f', str(bundle)], check=True, timeout=15)
    log = root / 'application.log'
    log.write_text('')
    proxy = mac = notifications = fixture = None
    try:
        fixture = ServiceFixture(root, owned_pids)
        proxy = subprocess.Popen(['/usr/bin/open', '-W', '-n', '-a', str(bundle),
                                  '--stdout', str(log), '--stderr', str(log), '--args',
                                  '--open-uri=gpuio-studio://preview/startup'])
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline and not pids(executable):
            if proxy.poll() is not None:
                raise AssertionError(log.read_text())
            time.sleep(.05)
        owned = pids(executable)
        assert len(owned) == 1, owned
        mac = Mac(owned[0], proxy)
        # A newly launched bundle can precede its first AppKit window. Repeat
        # activation only during setup; behavioral focus assertions stay strict.
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            mac.set(mac.app, 'AXFrontmost', mac.true)
            window = mac.window(TITLE)
            if window:
                mac.release(window)
                break
            time.sleep(.25)
        else:
            raise AssertionError('Packaged gallery did not expose its initial window')
        exercise_desktop(mac, None, bundled=True)

        def send(link):
            subprocess.run(['/usr/bin/open', '-g', '-a', str(bundle), '-u', link], check=True, timeout=5)
        send('gpuio-studio://preview/warm')
        mac.wait_text(TITLE, 'Received link: gpuio-studio://preview/warm')
        mac.press(TITLE, 'Presentation')
        send('gpuio-studio://preview/away')
        mac.press(TITLE, 'Desktop services')
        mac.wait_text(TITLE, 'Received link: gpuio-studio://preview/away')
        send('gpuio-studio://user@preview/invalid')
        mac.wait_text(TITLE, 'Rejected link: Invalid_authority')
        assert pids(executable) == owned, 'Link delivery launched another process'
        print('GALLERY_PACKAGED_LINKS_OK: generated metadata, live OS links, hidden-page delivery and rejection', flush=True)

        raise_gallery(mac)
        choose_path(mac, fixture.path)
        mac.press(TITLE, 'Open represented file')
        mac.wait_text(TITLE, 'File open: request accepted')
        receipt = root / 'opened-path.txt'
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline and not receipt.exists():
            time.sleep(.05)
        assert os.path.samefile(receipt.read_text(), fixture.path)
        mac.press(TITLE, 'Reveal represented file')
        mac.wait_text(TITLE, 'File reveal: request accepted')
        finder_pid = int(subprocess.check_output(['pgrep', '-u', str(os.getuid()), '-x', 'Finder'], text=True).strip())
        fixture.finder = Mac(finder_pid)
        node = fixture.finder.wait_find(root.name, 'Fixture résumé $()', contains=True, search_files=True)
        fixture.finder.release(node)
        raise_gallery(mac)
        mac.press(TITLE, 'Clear represented file')
        mac.wait_text(TITLE, 'Represented file: none')
        expect_enabled(mac, 'Open represented file', False)
        print('GALLERY_PACKAGED_FILES_OK: actual picker, exact fixture consumed by OS handler, Finder visibility and clear', flush=True)

        mac.press(TITLE, 'Check desktop support')
        mac.wait_text(TITLE, 'Notification support: body, actions, activation, replacement, dismissal, permission request, sound')
        notifications = Notifications(APP_NAME)
        undetermined = mac.find(TITLE, 'Notification access: Not_determined')
        if undetermined:
            mac.release(undetermined)
            mac.press(TITLE, 'Post preview notification')
            mac.wait_text(TITLE, 'Notification post: Not_ready')
            mac.press(TITLE, 'Allow OS notifications')
            notifications.act(f'“{APP_NAME}” Notifications', 'Allow')
        denied = mac.find(TITLE, 'Notification access: Denied')
        if denied:
            mac.release(denied)
            raise AssertionError('Component Studio notifications are denied; enable only this app in System Settings before rerunning')
        mac.wait_text(TITLE, 'Notification access: Authorized')
        mac.press(TITLE, 'Post preview notification')
        mac.wait_text(TITLE, 'Notification submitted; presentation is decided by the OS')
        notifications.mac.release(notifications.wait(INITIAL))
        expect_enabled(mac, 'Post preview notification', False)
        mac.press(TITLE, 'Replace preview notification')
        mac.wait_text(TITLE, 'Notification replacement: request accepted')
        notifications.open_center()
        notifications.mac.release(notifications.wait(UPDATED))
        mac.press(TITLE, 'Presentation')
        notifications.act(UPDATED, 'Inspect preview')
        mac.press(TITLE, 'Desktop services')
        mac.wait_text(TITLE, 'Notification action: inspect')
        expect_enabled(mac, 'Post preview notification', True)
        expect_enabled(mac, 'Replace preview notification', False)
        mac.press(TITLE, 'New window')
        second = 'GPUIO · Component Studio 3'
        mac.wait_text(second, 'A little context goes a long way')
        mac.press(second, 'Desktop services')
        mac.wait_text(second, 'Notification action: inspect')
        mac.press(second, 'Post preview notification')
        mac.wait_text(TITLE, 'Notification submitted; presentation is decided by the OS')
        mac.close(second)
        notifications.mac.release(notifications.wait(INITIAL))
        mac.press(TITLE, 'Dismiss preview notification')
        mac.wait_text(TITLE, 'Notification dismissal: request accepted')
        expect_enabled(mac, 'Post preview notification', True)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            node = notifications.find(INITIAL)
            if not node:
                break
            notifications.mac.release(node)
            time.sleep(.05)
        else:
            raise AssertionError('Notification remained after dismissal')
        print('GALLERY_PACKAGED_NOTIFICATIONS_OK: explicit permission, native presentation, replacement/action, cross-window receipt and dismissal', flush=True)
        notifications.close()
        notifications = None
        mac.close(TITLE)
        assert proxy.wait(timeout=15) == 0
        assert not pids(executable)
        print('GPUIO_GALLERY_PACKAGED_OK', flush=True)
    finally:
        # A failed AX cleanup must not leave the application or its file handler
        # running. Attempt every owner independently and preserve cleanup errors.
        errors = []
        def cleanup(label, operation):
            try:
                operation()
            except Exception as error:
                errors.append((label, repr(error)))
        def stop_application():
            for pid in pids(executable):
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            if proxy:
                try:
                    proxy.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    for pid in pids(executable):
                        try:
                            os.kill(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                    proxy.wait(timeout=5)
        if notifications:
            cleanup('Notification Center', notifications.close)
        if mac:
            cleanup('AX application', lambda: mac.release(mac.app))
        cleanup('gallery process', stop_application)
        if fixture:
            cleanup('file fixture', fixture.close)
        cleanup('bundle registration', lambda: subprocess.run(
            [LSREGISTER, '-u', str(bundle)], check=True, timeout=10))
        print('Artifacts:', root, flush=True)
        print(log.read_text(errors='replace'), end='')
        if errors:
            raise RuntimeError('Packaged gallery cleanup failed', errors)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', type=Path, default=Path('scratch/gallery-desktop'))
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    exercise(repo / '_build/default/examples/gallery/main.exe', args.artifacts)


if __name__ == '__main__':
    main()
