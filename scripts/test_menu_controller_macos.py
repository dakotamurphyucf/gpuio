#!/usr/bin/env python3
"""Public positioned-menu controller through AppKit and asynchronous OCaml effects."""
import argparse
import subprocess
import tempfile
import time
from pathlib import Path
from test_agent_chat import Mac
from test_gallery import element_rect
from test_native_popup_macos import wait_popup, enabled

TITLE = 'GPUIO positioned menus'


def select_run(mac, menu):
    mac.key(15)
    deadline = time.monotonic() + 5
    while True:
        selected = mac.children(menu, 'AXSelectedChildren')
        try:
            names = [mac.text(row, 'AXTitle') for row in selected]
        finally:
            for row in selected:
                mac.release(row)
        if names == ['Run']:
            break
        if time.monotonic() > deadline:
            raise RuntimeError(f'Run not selected: {names}')
        time.sleep(.03)
    mac.key(36)


def wait_enabled(mac, label):
    deadline = time.monotonic() + 5
    while True:
        button = mac.wait_find(TITLE, label, 'AXButton')
        try:
            ready = enabled(mac, button)
        finally:
            mac.release(button)
        if ready:
            return
        if time.monotonic() > deadline:
            raise RuntimeError(f'Button did not become available: {label}')
        time.sleep(.03)


def exercise(mac):
    mac.wait_text(TITLE, 'Ready')
    mac.set(mac.app, 'AXFrontmost', mac.true)
    window = mac.window(TITLE)
    try:
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    mac.field(TITLE, 'Draft', 'AXTextField', 'Positioned draft')
    wait_enabled(mac, 'Show first position')
    mac.press(TITLE, 'Show first position')
    menu = wait_popup(mac, title=TITLE)
    try:
        first = element_rect(mac, menu)
        mac.wait_text(TITLE, 'Show first: accepted')
        # A second request during the first tracking lease must be rejected.
        mac.press(TITLE, 'Show second position')
        mac.wait_text(TITLE, 'Show second: Busy')
        assert element_rect(mac, menu) == first
        wait_enabled(mac, 'Show other owner')
        mac.press(TITLE, 'Show other owner')
        mac.wait_text(TITLE, 'Other menu: Busy')
        assert element_rect(mac, menu) == first
        mac.press(TITLE, 'Close popup')
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=TITLE)
    mac.wait_text(TITLE, 'Close: accepted')
    mac.wait_text(TITLE, 'Total runs: 0')
    mac.press(TITLE, 'Show second position')
    menu = wait_popup(mac, title=TITLE)
    try:
        second = element_rect(mac, menu)
        # Points differ by (300,100) logical units, independently of titlebar
        # origin. Both positions fit this test display without OS clamping.
        assert abs((second[0] - first[0]) - 300) <= 2, (first, second)
        assert abs((second[1] - first[1]) - 100) <= 2, (first, second)
        select_run(mac, menu)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=TITLE)
    mac.wait_text(TITLE, 'Total runs: 1')
    assert mac.field(TITLE, 'Draft', 'AXTextField') == 'Positioned draft'
    print('POSITIONED_POPUP_GEOMETRY_BUSY_CLOSE_OK', first, second, flush=True)

    mac.press(TITLE, 'Show first position')
    menu = wait_popup(mac, title=TITLE)
    mac.release(menu)
    mac.press(TITLE, 'Detach observer')
    wait_popup(mac, False, title=TITLE)
    mac.wait_text(TITLE, 'Observer detached')
    mac.press(TITLE, 'Attach observer')
    wait_enabled(mac, 'Show first position')
    print('POSITIONED_POPUP_OBSERVER_RETIREMENT_OK', flush=True)

    mac.press(TITLE, 'Capture old menu')
    wait_enabled(mac, 'Show captured menu')
    mac.press(TITLE, 'Replace menu definition')
    mac.wait_text(TITLE, 'Menu subscription updated')
    # Observe the actual replacement popup before issuing a captured old request.
    mac.press(TITLE, 'Show first position')
    menu = wait_popup(mac, title=TITLE)
    try:
        rows = mac.children(menu)
        try:
            assert [mac.text(row, 'AXTitle') for row in rows] == ['Replacement actions', 'Run']
        finally:
            for row in rows:
                mac.release(row)
    finally:
        mac.release(menu)
    mac.key(53)
    wait_popup(mac, False, title=TITLE)
    mac.press(TITLE, 'Show captured menu')
    mac.wait_text(TITLE, 'Saved menu: Stale_menu')
    wait_popup(mac, False, title=TITLE)
    mac.wait_text(TITLE, 'Total runs: 1')
    mac.press(TITLE, 'Close popup')
    mac.wait_text(TITLE, 'Close: accepted')
    print('POSITIONED_POPUP_STALE_DEFINITION_OK', flush=True)
    mac.press(TITLE, 'Close window')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, default=Path('_build/default/examples/menu_controller/main.exe'))
    args = parser.parse_args()
    Mac.require_accessibility()
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(args.binary.resolve())], stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac)
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
    print('GPUIO_MENU_CONTROLLER_PUBLIC_OK')


if __name__ == '__main__':
    main()
