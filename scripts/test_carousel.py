#!/usr/bin/env python3
"""Public carousel through macOS AX and real keyboard delivery to its owned child.

The focusable Region's arrows emit Carousel_requested, exercising the native FFI,
Eio dispatcher, Bonsai reducer and accepted view update together. Child cleanup is
bounded on success and failure. Build examples/navigation/main.exe first.
"""
import argparse
import ctypes as C
from pathlib import Path
import subprocess
import tempfile
import time

from test_agent_chat import Mac
from test_canvas import screenshot
from test_sidebar import absent, boolean

TITLE = 'GPUIO Navigation Lab'


def focus(mac, label, role):
    node = mac.wait_find(TITLE, label, role)
    try:
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)


def selected(mac, label):
    mac.wait_text(TITLE, 'Gallery selection: ' + label)


def gallery_button(mac, label):
    def visit(node):
        if mac.text(node, 'AXRole') == 'AXButton' and mac.text(node, 'AXTitle') == label:
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
    root = mac.wait_find(TITLE, 'Project gallery', 'AXGroup')
    try:
        node = visit(root)
        if not node:
            raise RuntimeError('Missing gallery control ' + label)
        return node
    finally:
        mac.release(root)


def press_gallery(mac, label):
    node = gallery_button(mac, label)
    try:
        mac.perform(node, 'AXPress')
    finally:
        mac.release(node)


def toggle(mac, label):
    node = mac.wait_find(TITLE, label, 'AXCheckBox')
    try:
        mac.perform(node, 'AXPress')
    finally:
        mac.release(node)


def move_to_heading(mac):
    # Send a window-relative native event only to the owned process. This gives
    # the carousel a definite outside-hover position without moving the user's
    # global cursor onto another application.
    class Point(C.Structure):
        _fields_ = [('x', C.c_double), ('y', C.c_double)]
    node = mac.wait_find(TITLE, 'Your work, in motion.', 'AXStaticText')
    value = mac.attr(node, 'AXPosition')
    point = Point()
    get = mac.ax.AXValueGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
    try:
        if not value or not get(value, 1, C.byref(point)):
            raise RuntimeError('Heading lacks native position')
    finally:
        if value:
            mac.release(value)
        mac.release(node)
    point.x += 4
    point.y += 4
    create = mac.cg.CGEventCreateMouseEvent
    create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, Point, C.c_int]
    event = create(None, 5, point, 0)  # MouseMoved.
    if not event:
        raise RuntimeError('Cannot create native pointer event')
    try:
        mac.post_key(mac.pid, event)
    finally:
        mac.release(event)


def exercise(mac, images):
    selected(mac, 'Draft')
    mac.set(mac.app, 'AXFrontmost', mac.true)
    focus(mac, 'Project gallery', 'AXGroup')
    mac.key(124)  # Right: native Carousel_requested through Eio/Bonsai.
    selected(mac, 'Review')
    absent(mac, 'Gallery draft', 'AXTextField')
    current = gallery_button(mac, '2')
    try:
        assert mac.text(current, 'AXHelp') == 'Current item'
    finally:
        mac.release(current)
    mac.key(119)  # End.
    selected(mac, 'Deliver')
    mac.key(124)  # Loop to first.
    selected(mac, 'Draft')
    mac.key(119)
    selected(mac, 'Deliver')
    mac.key(115)  # Home.
    selected(mac, 'Draft')
    assert mac.field(TITLE, 'Gallery draft', 'AXTextField', 'Retained native draft 👩🏽‍💻')
    mac.key(124)  # Editor owns Right, even if it reaches the end of the text.
    time.sleep(0.2)
    selected(mac, 'Draft')
    press_gallery(mac, 'Next')
    selected(mac, 'Review')
    press_gallery(mac, 'First')
    selected(mac, 'Draft')
    assert mac.field(TITLE, 'Gallery draft', 'AXTextField') == 'Retained native draft 👩🏽‍💻'
    if images:
        time.sleep(0.35)  # Capture the settled default 200ms transition.
        screenshot(mac, images / 'carousel-draft.png')
    mac.press(TITLE, 'Direction: horizontal')
    focus(mac, 'Project gallery', 'AXGroup')
    mac.key(125)  # Down: vertical carousel.
    selected(mac, 'Review')
    if images:
        time.sleep(0.35)
        screenshot(mac, images / 'carousel-review.png')
    # Nested modal scope cannot route native arrows to the background carousel.
    mac.press(TITLE, 'Gallery details')
    mac.wait_text(TITLE, 'Workspace details')
    mac.key(125)
    time.sleep(0.2)
    selected(mac, 'Review')
    mac.key(53)  # Escape accepts drawer dismissal through the application.
    absent(mac, 'Workspace details', 'AXGroup')
    # Explicit native Unmount destroys the draft lease; the Bonsai stars survive.
    press_gallery(mac, 'First')
    selected(mac, 'Draft')
    mac.press(TITLE, 'Add a star')
    mac.wait_text(TITLE, '1 star · shared Bonsai state')
    mac.press(TITLE, 'Content: retain')
    press_gallery(mac, 'Next')
    selected(mac, 'Review')
    absent(mac, 'Gallery draft', 'AXTextField')
    press_gallery(mac, 'First')
    selected(mac, 'Draft')
    assert mac.field(TITLE, 'Gallery draft', 'AXTextField') == 'A small idea, ready to become something useful.'
    mac.wait_text(TITLE, '1 star · shared Bonsai state')
    toggle(mac, 'Disable gallery')
    next_button = gallery_button(mac, 'Next')
    try:
        assert not boolean(mac, next_button, 'AXEnabled')
    finally:
        mac.release(next_button)
    toggle(mac, 'Disable gallery')
    press_gallery(mac, 'Last')
    selected(mac, 'Deliver')
    mac.press(TITLE, 'Remove delivery page')
    selected(mac, 'Review')
    mac.press(TITLE, 'Restore all pages')
    toggle(mac, 'Auto-advance')
    focus(mac, 'Gallery details', 'AXButton')
    move_to_heading(mac)
    selected(mac, 'Deliver')  # Four-second native clock proposal through Eio.
    toggle(mac, 'Auto-advance')
    if images:
        time.sleep(0.35)
        screenshot(mac, images / 'carousel-deliver.png')
    mac.close(TITLE)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--images', type=Path)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/navigation/main.exe'), '--carousel', '--motion-test'], cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac, args.images)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Carousel app exited unsuccessfully')
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
    print('GPUIO_CAROUSEL_PUBLIC_AX_OK: native arrows/Home/End/loop through FFI/Eio/Bonsai, current metadata, editor precedence, retained/unmounted draft, shared state, modal scope, disabled controls, shrink and native auto-advance')


if __name__ == '__main__':
    main()
