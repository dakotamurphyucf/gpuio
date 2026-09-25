#!/usr/bin/env python3
"""Check public presentation helpers with real macOS layout and AX actions."""
import argparse
import ctypes as C
import math
from pathlib import Path
import subprocess
import tempfile
import time

from test_agent_chat import Mac
from test_canvas import screenshot

TITLE = 'GPUIO Component Studio'
CASES = ['label', 'tag', 'badge', 'marker', 'link', 'group', 'settings',
         'description', 'empty state', 'alert', 'banner', 'shortcut', 'status',
         'attachment', 'message', 'bubble', 'tool result', 'vertical field',
         'horizontal field']
CONTENTS = ['long', 'empty', 'localized']
ACTION_CASES = {'tag', 'group', 'settings', 'empty state', 'alert', 'banner',
                'status', 'attachment', 'message', 'tool result'}
TEXT = {
    'long': 'A descriptive workspace item with enough words to wrap across several lines in a narrow panel, while keeping its actions visible and available.',
    'localized': '共有ワークスペースの設定 · Résultats et pièces jointes · Grüße 👩‍👩‍👧‍👦',
}


class Pair(C.Structure):
    _fields_ = [('x', C.c_double), ('y', C.c_double)]


def geometry(mac, node, name, kind):
    value = mac.attr(node, name)
    try:
        result = Pair()
        get = mac.ax.AXValueGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        if not value or not get(value, kind, C.byref(result)):
            raise RuntimeError(f'Missing {name}')
        return result.x, result.y
    finally:
        if value:
            mac.release(value)


def inspect(mac, case):
    window = mac.window(TITLE)
    try:
        left, _ = geometry(mac, window, 'AXPosition', 1)
        width, _ = geometry(mac, window, 'AXSize', 2)
        def visit(node):
            role = mac.text(node, 'AXRole')
            if role in ['AXStaticText', 'AXButton', 'AXLink', 'AXCheckBox']:
                x, y = geometry(mac, node, 'AXPosition', 1)
                w, h = geometry(mac, node, 'AXSize', 2)
                label = mac.text(node, 'AXValue') or mac.text(node, 'AXTitle') or mac.text(node, 'AXDescription')
                if not all(math.isfinite(v) for v in [x, y, w, h]) or min(w, h) < 0:
                    raise RuntimeError(f'Invalid bounds {case}: {label!r}: {(x,y,w,h)}')
                if w > 0 and (x < left - 1 or x + w > left + width + 1):
                    raise RuntimeError(f'Horizontal overflow {case}: {role} {label!r}: {(x,y,w,h)}, window {(left,width)}')
                if role in ['AXButton', 'AXCheckBox'] and (w <= 0 or h <= 0):
                    raise RuntimeError(f'Collapsed action {case}: {label!r}')
            children = mac.children(node)
            try:
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        children = mac.children(window)
        try:
            # Native title-bar buttons are checked too, but not the AXWindow itself.
            for child in children:
                visit(child)
        finally:
            for child in children:
                mac.release(child)
    finally:
        mac.release(window)


def resize(mac, width):
    window = mac.window(TITLE)
    create = mac.ax.AXValueCreate
    create.restype, create.argtypes = C.c_void_p, [C.c_int, C.c_void_p]
    size = Pair(width, 800.)
    value = create(2, C.byref(size))
    try:
        mac.set(window, 'AXSize', value)
    finally:
        mac.release(value)
        mac.release(window)
    time.sleep(0.1)


def exercise(mac, images):
    mac.wait_text(TITLE, 'Case 0 / label / long / dark')
    actions = 0
    for width in [440, 800]:
        resize(mac, width)
        for index, (case, content) in enumerate((case, content) for case in CASES for content in CONTENTS):
            for theme in ['dark', 'light']:
                marker = f'Case {index} / {case} / {content} / {theme}'
                mac.wait_text(TITLE, marker)
                inspect(mac, marker)
                if content in TEXT:
                    text_node = mac.wait_find(TITLE, TEXT[content])
                    try:
                        if width == 440 and case == 'label' and content == 'long':
                            _, height = geometry(mac, text_node, 'AXSize', 2)
                            if height <= 20:
                                raise RuntimeError('Long label failed to wrap into multiple lines')
                    finally:
                        mac.release(text_node)
                action = mac.find(TITLE, 'Open item', 'AXButton')
                if not action and case in ACTION_CASES:
                    raise RuntimeError(f'Missing component action: {marker}')
                if action:
                    try:
                        mac.perform(action, 'AXPress')
                        actions += 1
                    finally:
                        mac.release(action)
                    mac.wait_text(TITLE, f'Fixture end · actions {actions}')
                if images and width == 440 and content == 'localized' and case in ['message', 'attachment', 'horizontal field']:
                    screenshot(mac, images / f'content-{case.replace(" ", "-")}-{theme}.png')
                mac.press(TITLE, 'Change theme')
            mac.press(TITLE, 'Next case')
    if actions != 120:
        raise RuntimeError(f'Expected 120 component actions, observed {actions}')
    window = mac.window(TITLE)
    try:
        button = mac.attr(window, 'AXCloseButton')
        try:
            mac.perform(button, 'AXPress')
        finally:
            mac.release(button)
    finally:
        mac.release(window)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--images', type=Path)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/presentation/main.exe'), '--content-check'], cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac, args.images)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Content fixture exited unsuccessfully')
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
    print('GPUIO_PRESENTATION_CONTENT_OK: 19 families, long/empty/localized, two widths/themes, AX bounds/actions and close')


if __name__ == '__main__':
    main()
