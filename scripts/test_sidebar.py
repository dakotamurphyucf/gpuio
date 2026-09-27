#!/usr/bin/env python3
"""Exercise the public sidebar through the owned macOS window and AX tree."""
import argparse
import ctypes as C
from pathlib import Path
import subprocess
import tempfile
import time

from test_agent_chat import Mac
from test_canvas import screenshot

TITLE = 'GPUIO Navigation Lab'


def absent(mac, label, role):
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        node = mac.find(TITLE, label, role)
        if not node:
            return
        mac.release(node)
        time.sleep(0.05)
    raise RuntimeError(f'Hidden sidebar content remains accessible: {label}')


def boolean(mac, node, attribute):
    value = mac.attr(node, attribute)
    if not value:
        raise RuntimeError(f'Missing {attribute}')
    try:
        getter = mac.cf.CFBooleanGetValue
        getter.restype, getter.argtypes = C.c_bool, [C.c_void_p]
        return getter(value)
    finally:
        mac.release(value)


def sidebar_link(mac, label):
    # Breadcrumbs also contain an "Archive" link. Search within the sidebar
    # landmark so changing its side cannot change which control the test invokes.
    def visit(node):
        if mac.text(node, 'AXRole') == 'AXLink' and mac.text(node, 'AXTitle') == label:
            return mac.retain(node)
        children = mac.children(node)
        try:
            for child in children:
                result = visit(child)
                if result:
                    return result
        finally:
            for child in children:
                mac.release(child)
        return None
    root = mac.wait_find(TITLE, 'Sidebar', 'AXGroup')
    try:
        result = visit(root)
        if not result:
            raise RuntimeError(f'Sidebar link is missing: {label}')
        return result
    finally:
        mac.release(root)


def press_link(mac, label):
    link = sidebar_link(mac, label)
    try:
        mac.perform(link, 'AXPress')
    finally:
        mac.release(link)


def sidebar_width(mac):
    class Size(C.Structure):
        _fields_ = [('width', C.c_double), ('height', C.c_double)]
    node = mac.wait_find(TITLE, 'Sidebar', 'AXGroup')
    size = mac.attr(node, 'AXSize')
    try:
        getter = mac.ax.AXValueGetValue
        getter.restype, getter.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        result = Size()
        if not size or not getter(size, 2, C.byref(result)):
            raise RuntimeError('Sidebar lacks finite native geometry')
        return result.width
    finally:
        if size:
            mac.release(size)
        mac.release(node)


def sidebar_allocation(mac):
    """Window width minus adjacent content: works with a left or right sidebar."""
    class Size(C.Structure):
        _fields_ = [('width', C.c_double), ('height', C.c_double)]
    nodes = [mac.window(TITLE), mac.wait_find(TITLE, 'Navigation content', 'AXGroup')]
    widths = []
    try:
        for node in nodes:
            value = mac.attr(node, 'AXSize')
            try:
                getter = mac.ax.AXValueGetValue
                getter.restype, getter.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
                result = Size()
                if not value or not getter(value, 2, C.byref(result)):
                    raise RuntimeError('Content lacks native geometry')
                widths.append(result.width)
            finally:
                if value:
                    mac.release(value)
    finally:
        for node in nodes:
            mac.release(node)
    return widths[0] - widths[1]


def await_allocation(mac, predicate, timeout=5):
    deadline = time.monotonic() + timeout
    values = []
    while time.monotonic() < deadline:
        value = sidebar_allocation(mac)
        values.append(round(value, 2))
        if predicate(value):
            print('SIDEBAR_MOTION_SAMPLES', values, flush=True)
            return value
        time.sleep(0.03)
    raise RuntimeError(f'Width transition did not meet expectation: {values}')


def exercise_motion(mac, images):
    mac.wait_text(TITLE, 'Selected destination: Inbox')
    mac.set(mac.app, 'AXFrontmost', mac.true)
    full = sidebar_allocation(mac)
    assert 235 <= sidebar_width(mac) <= 245
    mac.press(TITLE, 'Collapse sidebar')
    absent(mac, 'Inbox', 'AXLink')
    # Outer allocation animates, while the content immediately takes icon width.
    await_allocation(mac, lambda x: full - 150 < x < full - 40)
    assert 51 <= sidebar_width(mac) <= 61
    mac.press(TITLE, 'Expand sidebar')
    assert sidebar_allocation(mac) < full - 10, 'Interruption snapped to the target'
    await_allocation(mac, lambda x: abs(x - full) < 1)
    mac.press(TITLE, 'Collapse sidebar')
    compact = full - 184
    await_allocation(mac, lambda x: abs(x - compact) < 1)
    mac.press(TITLE, 'Offcanvas mode')
    absent(mac, 'Sidebar', 'AXGroup')
    await_allocation(mac, lambda x: compact - 45 < x < compact - 10)
    await_allocation(mac, lambda x: abs(x - (full - 240)) < 1)
    mac.press(TITLE, 'Expand sidebar')
    await_allocation(mac, lambda x: full - 180 < x < full - 40)
    await_allocation(mac, lambda x: abs(x - full) < 1)
    mac.press(TITLE, 'Collapse sidebar')
    absent(mac, 'Sidebar', 'AXGroup')
    await_allocation(mac, lambda x: full - 75 < x < full - 35)
    if images:
        screenshot(mac, images / 'sidebar-exiting.png')
    await_allocation(mac, lambda x: abs(x - (full - 240)) < 1)
    mac.press(TITLE, 'Expand sidebar')
    await_allocation(mac, lambda x: abs(x - full) < 1)
    mac.press(TITLE, 'Icon mode')
    mac.press(TITLE, 'Reduce motion')
    mac.press(TITLE, 'Collapse sidebar')
    # Two-second full-motion runs settle under Reduce rather than playing out.
    await_allocation(mac, lambda x: abs(x - compact) < 1, timeout=0.9)
    mac.close(TITLE)
    print('GPUIO_SIDEBAR_MOTION_AX_OK: native allocation samples, fixed inner width, interruption, offcanvas hiding/reveal, reduced motion and close')


def exercise(mac, images):
    mac.wait_text(TITLE, 'Selected destination: Inbox')
    mac.set(mac.app, 'AXFrontmost', mac.true)
    link = mac.wait_find(TITLE, 'Inbox', 'AXLink')
    try:
        assert mac.text(link, 'AXHelp') == 'Current destination'
    finally:
        mac.release(link)
    locked = mac.wait_find(TITLE, 'Locked', 'AXLink')
    try:
        assert not boolean(mac, locked, 'AXEnabled')
    finally:
        mac.release(locked)
    assert 235 <= sidebar_width(mac) <= 245
    if images:
        screenshot(mac, images / 'sidebar-expanded.png')
    press_link(mac, 'Archive')
    mac.wait_text(TITLE, 'Selected destination: Archive')
    node = mac.wait_find(TITLE, 'Inbox', 'AXLink')
    mac.release(node)  # Selecting a parent must not toggle its subtree.
    mac.press(TITLE, 'Collapse Archive')
    absent(mac, 'Inbox', 'AXLink')
    mac.press(TITLE, 'Expand Archive')
    node = mac.wait_find(TITLE, 'Starred', 'AXLink')
    mac.release(node)
    press_link(mac, 'Starred')
    mac.wait_text(TITLE, 'Selected destination: Starred')
    mac.press(TITLE, 'Collapse sidebar')
    node = mac.wait_find(TITLE, 'Expand sidebar', 'AXButton')
    mac.release(node)
    absent(mac, 'Starred', 'AXLink')
    absent(mac, 'Collapse Archive', 'AXButton')
    assert 51 <= sidebar_width(mac) <= 61
    press_link(mac, 'Settings')
    mac.wait_text(TITLE, 'Selected destination: Settings')
    if images:
        screenshot(mac, images / 'sidebar-icon.png')
    mac.press(TITLE, 'Offcanvas mode')
    absent(mac, 'Settings', 'AXLink')
    absent(mac, 'Sidebar', 'AXGroup')
    if images:
        screenshot(mac, images / 'sidebar-offcanvas.png')
    mac.press(TITLE, 'Expand sidebar')
    node = mac.wait_find(TITLE, 'Inbox', 'AXLink')
    mac.release(node)  # Expansion survives both collapse modes.
    assert 235 <= sidebar_width(mac) <= 245
    link = sidebar_link(mac, 'Archive')
    try:
        mac.set(link, 'AXFocused', mac.true)
    finally:
        mac.release(link)
    mac.key(109, flags=1 << 17)  # Shift-F10: existing context-menu route.
    mac.wait_text(TITLE, 'Inspect destination')
    mac.key(36)  # Enter invokes the highlighted command.
    mac.wait_text(TITLE, 'Destination inspected')
    mac.close(TITLE)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--images', type=Path)
    parser.add_argument('--motion', action='store_true')
    parser.add_argument('--right', action='store_true')
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        command = [str(repo / '_build/default/examples/navigation/main.exe')]
        if args.motion:
            command.append('--motion-test')
        if args.right:
            command.append('--right-sidebar')
        child = subprocess.Popen(command,
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            if args.motion:
                exercise_motion(mac, args.images)
            else:
                exercise(mac, args.images)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Navigation app exited unsuccessfully')
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
    if not args.motion:
        print('GPUIO_SIDEBAR_AX_OK: parent navigation/expansion, current help, disabled route, icon/offcanvas geometry and hiding, restored expansion, context command and close')


if __name__ == '__main__':
    main()
