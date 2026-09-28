#!/usr/bin/env python3
"""Public chart-family labels/legends via macOS AX, with background screenshots.

Use --input for real AppKit keyboard/pointer selection and OCaml callback checks.
This is plotted-mark navigation, not the complete original-data alternative.
"""
import argparse
import ctypes as C
from pathlib import Path
import subprocess
import time

from test_agent_chat import Mac
from test_canvas import screenshot

TITLE = 'GPUIO · Chart Studio'
FAMILIES = [
    ('Line', '48 values', ['Atlas', 'Nova']),
    ('Area', '24 values', ['Active capacity']),
    ('Bar', '24 values', ['Completed evaluations']),
    ('Pie', '4 values', ['Reasoning', 'Code', 'Research', 'Other']),
    ('Radar', '10 values', ['Quality', 'Speed', 'Cost', 'Context', 'Reliability', 'Atlas', 'Nova']),
    ('Candlestick', '24 values', ['Rise · hollow', 'Fall · filled']),
    ('Sankey', '4 values', ['Incoming', 'Reasoning', 'Tools', 'Complete']),
]


DESCRIPTIONS = [
    'Response quality over time', 'Capacity throughout the day',
    'Throughput by evaluation run', 'Where the work happens',
    'A balanced model scorecard', 'The shape of market movement',
    'From request to resolution',
]
FIRST = ['Atlas · x 0', 'Active capacity · x 0', 'Completed evaluations · x 0',
         'Reasoning · 44', 'Atlas · Quality · 88 / 100', 'Session 1 · close',
         'Incoming → Reasoning · 65']
LAST = ['Nova · x 23', 'Active capacity · x 23', 'Completed evaluations · x 23',
        'Other · 10', 'Nova · Reliability · 86 / 100', 'Session 24 · close',
        'Complete']


def keyboard(mac, family_index):
    node = mac.wait_find(TITLE, DESCRIPTIONS[family_index], 'AXGroup')
    try:
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)
    mac.key(115)  # Home previews the first mark without an OCaml selection event.
    time.sleep(0.1)
    mac.wait_text(TITLE, 'Select a value ·')
    mac.key(36)  # Return commits it through the real AppKit -> Rust -> Eio path.
    mac.wait_text(TITLE, 'Selected: ' + FIRST[family_index])
    mac.key(119)  # End only previews; the public selection must remain unchanged.
    time.sleep(0.1)
    mac.wait_text(TITLE, 'Selected: ' + FIRST[family_index])
    mac.key(49)  # Space commits the last mark.
    mac.wait_text(TITLE, 'Selected: ' + LAST[family_index])
    mac.key(53)  # Escape explicitly clears.
    mac.wait_text(TITLE, 'Select a value ·')


def pointer(mac, output):
    # Chart is 760x330, with the legend occupying the bottom 28 logical pixels.
    # Discover its actual origin via AX; never infer the screen/window placement.
    class Point(C.Structure):
        _fields_ = [('x', C.c_double), ('y', C.c_double)]
    node = mac.wait_find(TITLE, DESCRIPTIONS[3], 'AXGroup')
    origin = Point()
    get = mac.ax.AXValueGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
    try:
        value = mac.attr(node, 'AXPosition')
        try:
            if not value or not get(value, 1, C.byref(origin)):
                raise RuntimeError('Chart has no accessible position')
        finally:
            if value:
                mac.release(value)
    finally:
        mac.release(node)
    create = mac.cg.CGEventCreateMouseEvent
    create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, Point, C.c_int]
    post = mac.cg.CGEventPost
    post.restype, post.argtypes = None, [C.c_int, C.c_void_p]
    system = mac.ax.AXUIElementCreateSystemWide
    system.restype, system.argtypes = C.c_void_p, []
    hit_test = mac.ax.AXUIElementCopyElementAtPosition
    hit_test.restype, hit_test.argtypes = C.c_int, [C.c_void_p, C.c_float, C.c_float, C.POINTER(C.c_void_p)]
    get_pid = mac.ax.AXUIElementGetPid
    get_pid.restype, get_pid.argtypes = C.c_int, [C.c_void_p, C.POINTER(C.c_int)]
    def send(kind, x, y):
        point = Point(origin.x + x, origin.y + y)
        root, hit, owner = system(), C.c_void_p(), C.c_int()
        try:
            if (hit_test(root, point.x, point.y, C.byref(hit)) or not hit.value
                    or get_pid(hit, C.byref(owner)) or owner.value != mac.pid):
                raise RuntimeError('Chart pointer target is occluded by another application')
        finally:
            if hit.value:
                mac.release(hit)
            mac.release(root)
        event = create(None, kind, point, 0)
        if not event:
            raise RuntimeError('Cannot create chart mouse event')
        try:
            post(0, event)
        finally:
            mac.release(event)
    send(5, 470, 150)  # Hover the right-hand Reasoning wedge.
    mac.wait_text(TITLE, '44 · 44.0%')
    mac.wait_text(TITLE, 'Select a value ·')
    send(1, 470, 150)
    send(2, 470, 150)
    mac.wait_text(TITLE, 'Selected: Reasoning · 44')
    screenshot(mac, output / 'pie-selected.png', title=TITLE)
    send(1, 470, 150)
    send(6, 300, 210)  # Drag into Code without committing until release.
    mac.wait_text(TITLE, '28 · 28.0%')
    mac.wait_text(TITLE, 'Selected: Reasoning · 44')
    send(2, 300, 210)
    mac.wait_text(TITLE, 'Selected: Code · 28')
    send(1, 300, 210)
    send(6, -10, 150)
    send(2, -10, 150)  # Outside release cancels, preserving Code.
    time.sleep(0.1)
    mac.wait_text(TITLE, 'Selected: Code · 28')
    mac.key(53)
    mac.wait_text(TITLE, 'Select a value ·')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--foreground', action='store_true', help='Activate the child when background windows do not receive frames')
    parser.add_argument('--input', action='store_true', help='Activate and exercise real keyboard/pointer input')
    parser.add_argument('--app', default='_build/default/examples/charts/main.exe')
    parser.add_argument('--output', type=Path, default=Path('.cache/chart-text-acceptance'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with (args.output / 'application.log').open('w') as log:
        child = subprocess.Popen([args.app, *([] if args.foreground or args.input else ['--background'])], stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Mac(child.pid, child)
            for index, (family, status, labels) in enumerate(FAMILIES):
                if family != 'Line':
                    mac.press(TITLE, family)
                mac.wait_text(TITLE, status)
                mac.wait_text(TITLE, 'Chart legend')
                for label in labels:
                    mac.wait_text(TITLE, label)
                if family in ['Line', 'Area', 'Bar', 'Candlestick']:
                    for tick in ['0', '23']:
                        node = mac.wait_find(TITLE, tick, 'AXStaticText')
                        mac.release(node)
                if args.input:
                    keyboard(mac, index)
                    if family == 'Pie':
                        pointer(mac, args.output)
                time.sleep(0.1)
                screenshot(mac, args.output / f'{family.lower()}.png', title=TITLE)
                mac.press(TITLE, 'Update data ↗')
                time.sleep(0.15)
                mac.wait_text(TITLE, status)
                for label in labels:
                    mac.wait_text(TITLE, label)
            if args.input:
                print('CHART_INPUT_OK: seven-family AppKit keys, pointer hover/click/drag/cancel, asynchronous OCaml selections', flush=True)
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
            print('CHART_TEXT_OK: seven families, native labels/legends, update, OS close', flush=True)
        finally:
            if mac is not None:
                mac.release(mac.app)
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()


if __name__ == '__main__':
    main()
