#!/usr/bin/env python3
"""Qualify public menu ownership across windows and retained native editors."""
import argparse
import subprocess
import tempfile
import time
from pathlib import Path

from mac_clipboard import preserved_clipboard
from test_agent_chat import Mac
from test_native_popup_macos import enabled, wait_popup

A = 'GPUIO menu window A'
B = 'GPUIO menu window B'
SENTINEL = 'GPUIO popup target guard'


def wait_for(predicate, description):
    deadline = time.monotonic() + 10
    while not predicate():
        if time.monotonic() >= deadline:
            raise RuntimeError(description)
        time.sleep(.03)


def activate(mac, title):
    mac.set(mac.app, 'AXFrontmost', mac.true)
    window = mac.window(title)
    assert window, title
    try:
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    mac.press(title, 'Activate window')
    mac.wait_text(title, 'Activate:')
    mac.press(title, 'Observe window')
    mac.wait_text(title, 'Observe: active=true')
    # Focus a real editor before using keys; AXRaise alone is not proof that
    # the native window has processed activation.
    focus(mac, title, 'Second')


def focus(mac, title, which):
    mac.press(title, f'Focus {which.lower()} editor')
    mac.wait_text(title, f'{which} focus: applied')


def select_all(mac, title, which):
    focus(mac, title, which)
    mac.key(0, 1 << 20)  # Command-A through the native editor.
    node = mac.wait_find(title, f'{which} editor', 'AXTextField')
    try:
        expected = mac.text(node, 'AXValue')
        wait_for(lambda: mac.text(node, 'AXSelectedText') == expected,
                 f'{title}: {which} editor did not select its text')
    finally:
        mac.release(node)
    return expected


def open_popup(mac, title):
    def ready():
        button = mac.wait_find(title, 'Open popup', 'AXButton')
        try:
            return enabled(mac, button)
        finally:
            mac.release(button)
    wait_for(ready, 'Menu subscription did not become ready')
    mac.press(title, 'Open popup')
    menu = wait_popup(mac, title=title)
    mac.wait_text(title, 'Open popup: accepted')
    return menu


def choose(mac, menu, label, key):
    rows = mac.children(menu)
    try:
        row = next(row for row in rows if mac.text(row, 'AXTitle') == label)
        assert enabled(mac, row), f'{label} must be available in the native snapshot'
    finally:
        for row in rows:
            mac.release(row)
    mac.key(key)
    def selected():
        rows = mac.children(menu, 'AXSelectedChildren')
        try:
            return [mac.text(row, 'AXTitle') for row in rows] == [label]
        finally:
            for row in rows:
                mac.release(row)
    wait_for(selected, f'Native typeahead did not select {label}')
    mac.key(36)


def copy(mac, title, clipboard, expected):
    menu = open_popup(mac, title)
    try:
        choose(mac, menu, 'Copy selection', 8)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=title)
    wait_for(lambda: clipboard.text() == expected, 'Copy did not reach its editor')


def sentinel(clipboard):
    clipboard.restore([[('public.utf8-plain-text', SENTINEL.encode())]])


def exercise(mac, clipboard):
    for title in (A, B):
        mac.wait_text(title, 'Total runs: 0')
    activate(mac, A)
    menu = open_popup(mac, A)
    try:
        # AXPress requests an ordinary public OCaml command without bringing
        # the inactive window forward or sharing its Bonsai graph with A.
        mac.press(B, 'Open popup')
        mac.wait_text(B, 'Open popup: Unavailable')
        wait_popup(mac, False, title=B)
        choose(mac, menu, 'Run', 15)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=A)
    mac.wait_text(A, 'Total runs: 1')
    mac.wait_text(B, 'Total runs: 0')
    activate(mac, B)
    menu = open_popup(mac, B)
    try:
        choose(mac, menu, 'Run', 15)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=B)
    mac.wait_text(B, 'Total runs: 1')
    mac.wait_text(A, 'Total runs: 1')
    print('POPUP_INDEPENDENT_WINDOWS_OK', flush=True)

    activate(mac, A)
    second = select_all(mac, A, 'Second')
    first = select_all(mac, A, 'First')
    # Positive control proves the original target can copy before testing
    # rejection; both editors retain nonempty selections.
    copy(mac, A, clipboard, first)
    sentinel(clipboard)
    menu = open_popup(mac, A)
    try:
        focus(mac, A, 'Second')
        choose(mac, menu, 'Copy selection', 8)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=A)
    # A subsequent successful command on the same window is the asynchronous
    # processing barrier before asserting that stale Copy did nothing.
    focus(mac, A, 'Second')
    assert clipboard.text() == SENTINEL, 'Tracking popup silently retargeted Copy'
    copy(mac, A, clipboard, second)
    print('POPUP_CHANGED_EDITOR_TARGET_OK', flush=True)

    focus(mac, A, 'First')
    sentinel(clipboard)
    menu = open_popup(mac, A)
    try:
        mac.press(A, 'Remove first editor')
        mac.wait_text(A, 'First editor removed')
        focus(mac, A, 'Second')
        choose(mac, menu, 'Copy selection', 8)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=A)
    focus(mac, A, 'Second')
    assert clipboard.text() == SENTINEL, 'Removed editor caused Copy to retarget'
    copy(mac, A, clipboard, second)
    print('POPUP_REMOVED_EDITOR_TARGET_OK', flush=True)

    menu = open_popup(mac, A)
    mac.release(menu)
    activate(mac, B)
    mac.press(A, 'Observe window')
    mac.wait_text(A, 'Observe: active=false')
    wait_popup(mac, False, title=A)
    mac.wait_text(A, 'Total runs: 1')
    menu = open_popup(mac, B)
    try:
        choose(mac, menu, 'Run', 15)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=B)
    mac.wait_text(B, 'Total runs: 2')
    print('POPUP_WINDOW_ACTIVATION_RECOVERY_OK', flush=True)

    activate(mac, A)
    menu = open_popup(mac, A)
    mac.release(menu)
    mac.press(A, 'Close window')
    def closed():
        window = mac.window(A)
        if window:
            mac.release(window)
            return False
        return True
    wait_for(closed, 'Popup owner window did not close')
    activate(mac, B)
    menu = open_popup(mac, B)
    try:
        choose(mac, menu, 'Run', 15)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=B)
    mac.wait_text(B, 'Total runs: 3')
    mac.press(B, 'Close window')
    print('POPUP_OWNER_CLOSE_OTHER_WINDOW_RECOVERY_OK', flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path,
                        default=Path('_build/default/examples/menu_controller/main.exe'))
    args = parser.parse_args()
    Mac.require_accessibility()
    with preserved_clipboard() as clipboard, tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(args.binary.resolve()), '--two-windows'],
                                 stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
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
            print(log.read(), end='')
    print('GPUIO_MENU_MULTIWINDOW_PUBLIC_OK')


if __name__ == '__main__':
    main()
