#!/usr/bin/env python3
"""Exercise the public gallery's native navigation, editing and window isolation."""
import argparse
from pathlib import Path
import subprocess
import tempfile
import time

from test_agent_chat import Mac
from test_canvas import screenshot

TITLE = 'GPUIO · Component Studio 1'
SECOND = 'GPUIO · Component Studio 2'


def expect_field(mac, title, label, expected):
    deadline = time.monotonic() + 10
    actual = None
    while time.monotonic() < deadline:
        actual = mac.field(title, label, 'AXTextField')
        if actual == expected:
            return
        time.sleep(0.025)
    raise RuntimeError(f'{label}: expected {expected!r}, got {actual!r}')


def exercise(mac, images):
    mac.wait_text(TITLE, 'A little context goes a long way')
    if images:
        screenshot(mac, images / 'gallery-presentation-dark.png', title=TITLE)
    mac.press(TITLE, 'Selection & actions')
    mac.press(TITLE, 'Pressed 0 times')
    mac.wait_text(TITLE, 'Pressed 1 times')
    mac.press(TITLE, 'Numbers & codes')
    mac.wait_text(TITLE, 'Level: 35')
    slider = mac.wait_find(TITLE, 'Preview level', 'AXSlider')
    try:
        mac.perform(slider, 'AXIncrement')
    finally:
        mac.release(slider)
    mac.wait_text(TITLE, 'Level: 36')
    mac.wait_text(TITLE, 'Committed quantity: 12')
    mac.press(TITLE, 'Text editing')
    field = mac.wait_find(TITLE, 'Document title', 'AXTextField')
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.set(field, 'AXFocused', mac.true)
        mac.key(0, flags=1 << 20)  # Command+A
        mac.key(0)  # a
    finally:
        mac.release(field)
    expect_field(mac, TITLE, 'Document title', 'a')
    mac.key(36)  # Return: real OS submit into OCaml effect.
    mac.wait_text(TITLE, 'Submitted: a')
    mac.press(TITLE, 'Dark')
    mac.release(mac.wait_find(TITLE, 'Light', 'AXButton'))
    expect_field(mac, TITLE, 'Document title', 'a')
    for current, next_label in [('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')]:
        mac.press(TITLE, current)
        mac.release(mac.wait_find(TITLE, next_label, 'AXButton'))
        expect_field(mac, TITLE, 'Document title', 'a')
    if images:
        screenshot(mac, images / 'gallery-editing-light.png', title=TITLE)
    mac.press(TITLE, 'New window')
    mac.wait_text(SECOND, 'A little context goes a long way')
    mac.press(SECOND, 'Text editing')
    expect_field(mac, SECOND, 'Document title', 'A place for good ideas')
    expect_field(mac, TITLE, 'Document title', 'a')
    mac.close(SECOND)
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        node = mac.find(TITLE, 'Document title', 'AXTextField')
        if node:
            mac.release(node)
            raise RuntimeError('Unmounted editor remains accessible')
        mac.press(TITLE, 'Text editing')
        expect_field(mac, TITLE, 'Document title', 'A place for good ideas')
    mac.close(TITLE)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--images', type=Path)
    args = parser.parse_args()
    if args.images:
        args.images.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/gallery/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac, args.images)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Gallery exited unsuccessfully')
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
    print('GPUIO_GALLERY_AX_OK: navigation, button, native typing/submit, appearance/size preservation, independent windows, remount, shutdown')


if __name__ == '__main__':
    main()
