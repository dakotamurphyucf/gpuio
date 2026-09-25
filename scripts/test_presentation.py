#!/usr/bin/env python3
"""Exercise the public presentation example using its child macOS AX tree."""
import argparse
import ctypes as C
from pathlib import Path
import subprocess
import tempfile
import time

from test_agent_chat import Mac
from test_canvas import screenshot

TITLE = 'GPUIO Component Studio'


def assert_action_layout(mac, label):
    class Size(C.Structure):
        _fields_ = [('width', C.c_double), ('height', C.c_double)]
    get_value = mac.ax.AXValueGetValue
    get_value.restype, get_value.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
    node = mac.wait_find(TITLE, label, 'AXButton')
    value = mac.attr(node, 'AXSize')
    try:
        size = Size()
        if not value or not get_value(value, 2, C.byref(size)):
            raise RuntimeError(f'Missing native action bounds: {label}')
        if size.width <= size.height or size.height <= 0:
            raise RuntimeError(f'Action collapsed or wrapped vertically: {label}, {size.width}x{size.height}')
    finally:
        if value:
            mac.release(value)
        mac.release(node)


def assert_loading_semantics(mac):
    for label in ['Preparing content', 'Loading preview', 'Loading workspace']:
        node = mac.wait_find(TITLE, label, 'AXProgressIndicator')
        value = mac.attr(node, 'AXValue')
        try:
            if value:
                raise RuntimeError(f'Indeterminate loading must not invent a numeric value: {label}')
        finally:
            if value:
                mac.release(value)
            mac.release(node)


def exercise(mac, images):
    field = mac.wait_find(TITLE, 'Workspace name', 'AXTextField')
    try:
        assert mac.text(field, 'AXHelp') == 'A name for your local conversations.'
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.set(field, 'AXFocused', mac.true)
    finally:
        mac.release(field)
    # Real keyboard delivery into the native editor, targeted only to this child.
    mac.key(0, flags=1 << 20)  # Command+A
    mac.key(0)  # a
    end = time.monotonic() + 10
    while mac.field(TITLE, 'Workspace name', 'AXTextField') != 'a':
        if time.monotonic() >= end:
            raise RuntimeError('Native editor did not receive targeted keyboard input')
        time.sleep(0.05)
    mac.press(TITLE, 'Show validation')
    mac.wait_text(TITLE, 'This name is already in use')
    field = mac.wait_find(TITLE, 'Workspace name', 'AXTextField')
    try:
        assert mac.text(field, 'AXHelp') == ('A name for your local conversations.\n'
                                           'This name is already in use in the simulated example.')
        assert mac.text(field, 'AXValue') == 'a'
    finally:
        mac.release(field)
    mac.press(TITLE, 'Clear error')
    node = mac.wait_find(TITLE, 'Show validation', 'AXButton')
    mac.release(node)
    assert_loading_semantics(mac)
    mac.press(TITLE, 'Static indicators')
    node = mac.wait_find(TITLE, 'Animate indicators', 'AXButton')
    mac.release(node)
    assert_loading_semantics(mac)
    mac.press(TITLE, 'Hide indicators')
    node = mac.wait_find(TITLE, 'Show indicators', 'AXButton')
    mac.release(node)
    for label in ['Preparing content', 'Loading preview', 'Loading workspace']:
        node = mac.find(TITLE, label, 'AXProgressIndicator')
        if node:
            mac.release(node)
            raise RuntimeError(f'Hidden indicator remains in native accessibility: {label}')
    mac.press(TITLE, 'Show indicators')
    assert_loading_semantics(mac)
    for label in ['Details', 'Preview']:
        assert_action_layout(mac, label)
    if images:
        screenshot(mac, images / 'presentation-dark.png')
    mac.press(TITLE, 'Light appearance')
    node = mac.wait_find(TITLE, 'Dark appearance', 'AXButton')
    mac.release(node)
    assert mac.field(TITLE, 'Workspace name', 'AXTextField') == 'a'
    for label in ['Details', 'Preview']:
        assert_action_layout(mac, label)
    if images:
        screenshot(mac, images / 'presentation-light.png')
    node = mac.wait_find(TITLE, 'Documentation', 'AXLink')
    try:
        mac.perform(node, 'AXPress')
    finally:
        mac.release(node)
    mac.wait_text(TITLE, 'Documentation selected')
    mac.press(TITLE, 'Details')
    mac.wait_text(TITLE, 'This result is a reproducible local fixture.')
    window = mac.window(TITLE)
    try:
        close = mac.attr(window, 'AXCloseButton')
        if not close:
            raise RuntimeError('Native window close button is missing')
        try:
            mac.perform(close, 'AXPress')
        finally:
            mac.release(close)
    finally:
        mac.release(window)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--images', type=Path)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/presentation/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac, args.images)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Presentation app exited unsuccessfully')
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
    print('GPUIO_PRESENTATION_APP_AX_OK: keyboard, field help/error, theme, Link, card action, close')


if __name__ == '__main__':
    main()
