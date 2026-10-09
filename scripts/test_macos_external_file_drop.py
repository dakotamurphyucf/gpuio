#!/usr/bin/env python3
"""Drag fixture URLs from an independent Cocoa process into the public gallery.

Requires a real macOS desktop/accessibility access and the compiled Swift source
fixture. Owns both children; never opens Finder or writes the general pasteboard.
"""
import argparse
import ctypes as C
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import tempfile
import time

from package_macos_reference import digest
from test_agent_chat import Mac
from test_canvas import screenshot
from test_gallery import (TITLE, GalleryMouse, element_rect, raise_gallery,
                          reveal_gallery_control)

SOURCE_TITLE = 'GPUIO File Source Fixture'


def wait(predicate, label):
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.04)
    raise RuntimeError('Timed out: ' + label)


def place(mac, title, position, size):
    window = mac.window(title)
    assert window, title
    create = mac.ax.AXValueCreate
    create.restype, create.argtypes = C.c_void_p, [C.c_int, C.c_void_p]
    try:
        for attribute, kind, pair in [('AXSize', 2, size), ('AXPosition', 1, position)]:
            data = (C.c_double * 2)(*pair)
            value = create(kind, C.byref(data))
            try:
                mac.set(window, attribute, value)
            finally:
                mac.release(value)
    finally:
        mac.release(window)
    time.sleep(.2)


def stop(child):
    if child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait()


def drag(source, gallery):
    node = source.wait_find(SOURCE_TITLE, 'Fixture file source', 'AXGroup')
    try:
        x, y, w, h = element_rect(source, node)
    finally:
        source.release(node)
    start = (x + w / 2, y + h / 2)
    target = GalleryMouse(gallery)
    x, y, w, h = target.bounds('Idea transfer inbox')
    finish = (x + w / 2, y + h / 2)
    mouse = GalleryMouse(source)
    source.set(source.app, 'AXFrontmost', source.true)
    mouse.check_owner(start)
    target.check_owner(finish)
    point = start
    try:
        mouse.send(5, point)
        mouse.send(1, point)
        time.sleep(.1)
        # First establish the native drag inside the owned source window, then
        # cross the desktop without clicking anything along the route.
        for step in range(1, 5):
            point = (start[0] + step * 4, start[1])
            mouse.check_owner(point)
            mouse.send(6, point)
            time.sleep(.06)
        for step in range(1, 25):
            point = tuple(a + (b - a) * step / 24 for a, b in zip(start, finish))
            mouse.send(6, point)
            time.sleep(.025)
        target.check_owner(finish)
        time.sleep(.15)
    finally:
        mouse.send(2, point)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', required=True, type=Path)
    parser.add_argument('--gallery', type=Path,
                        default=Path('_build/default/examples/gallery/main.exe'))
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    Mac.require_accessibility()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'platform': platform.platform(),
              'gallery_sha256': digest(args.gallery), 'source_sha256': digest(args.source),
              'scope': 'independent Cocoa file-URL source to public GPUIO gallery; no filesystem transfer'}
    children, owners = [], []
    def interrupt(signum, _frame):
        raise RuntimeError(f'File-drop test interrupted: {signum}')
    signal.signal(signal.SIGTERM, interrupt)
    signal.signal(signal.SIGALRM, interrupt)
    signal.alarm(120)
    try:
        with tempfile.TemporaryDirectory(prefix='gpuio-external-drop-') as temporary, \
                (output / 'gallery.log').open('w') as gallery_log, \
                (output / 'source.log').open('w') as source_log:
            root = Path(temporary).resolve()
            paths = [root / 'hello λ 👋.txt', root / 'nested folder']
            paths[0].write_text('Fixture content stays untouched.\n')
            paths[1].mkdir()
            nested = paths[1] / 'retained.txt'
            nested.write_text('Nested fixture stays untouched.\n')
            expected = [os.fsencode(path).hex() for path in paths]
            before = [digest(paths[0]), digest(nested)]
            child = subprocess.Popen([str(args.gallery.resolve()), '--trace-input'],
                                     stdout=gallery_log, stderr=subprocess.STDOUT)
            children.append(child)
            gallery = Mac(child.pid, child)
            owners.append(gallery)
            gallery.wait_text(TITLE, 'A little context goes a long way')
            place(gallery, TITLE, (235, 40), (1060, 800))
            raise_gallery(gallery)
            gallery.press(TITLE, 'Input & transfers')
            reveal_gallery_control(gallery, 'Idea transfer inbox', 'AXGroup')
            source_child = subprocess.Popen([str(args.source.resolve()), *map(str, paths)],
                                            stdout=source_log, stderr=subprocess.STDOUT)
            children.append(source_child)
            source = Mac(source_child.pid, source_child)
            owners.append(source)
            node = source.wait_find(SOURCE_TITLE, 'Fixture file source', 'AXGroup')
            source.release(node)
            place(source, SOURCE_TITLE, (20, 260), (190, 140))
            results = []
            for attempt, enabled in enumerate([True, False, True], 1):
                if attempt > 1:
                    raise_gallery(gallery)
                    gallery.press(TITLE, 'Disable transfers' if not enabled else 'Enable transfers')
                    reveal_gallery_control(gallery, 'Idea transfer inbox', 'AXGroup')
                drag(source, gallery)
                wait(lambda: f'FILE_SOURCE_ENDED count={attempt} ' in (output / 'source.log').read_text(),
                     'native source drag session completion')
                count = 1 if attempt < 3 else 2
                gallery.wait_text(TITLE, f'Received items: {count}')
                gallery.wait_text(TITLE, 'Received 2 file path(s); no files opened')
                text = (output / 'gallery.log').read_text()
                received = re.findall(r'GALLERY_TRANSFER_FILE index=(\d+) path_hex=([0-9a-f]+) directory=(\S+)', text)
                assert received == [(str(index), value, '()') for _ in range(count)
                                    for index, value in enumerate(expected)], received
                assert len(re.findall(r'GALLERY_TRANSFER_TARGET .* phase=dropped', text)) == count, text
                assert 'GALLERY_TRANSFER_ORIGIN Desktop' in text, text
                results.append({'attempt': attempt, 'enabled': enabled, 'received_count': count})
            assert before == [digest(paths[0]), digest(nested)], 'Source files changed'
            raise_gallery(gallery)
            gallery.wait_text(TITLE, 'Drop text or files here')
            reveal_gallery_control(gallery, 'Received items: 2', 'AXStaticText')
            time.sleep(.2)  # Let the native presentation catch up before the artifact capture.
            screenshot(gallery, output / 'file-drop.png', title=TITLE)
            gallery.close(TITLE)
            assert child.wait(timeout=15) == 0, 'Gallery did not shut down cleanly'
            report.update(results=results, path_bytes_hex=expected, source_files_unchanged=True)
            report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        signal.alarm(0)
        for child in reversed(children):
            stop(child)
        for mac in owners:
            mac.release(mac.app)
        report['children_reaped'] = all(child.poll() is not None for child in children)
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print('GPUIO_EXTERNAL_FILE_DROP_OK: exact Unicode paths, directory, disabled/re-enabled target, cleanup')


if __name__ == '__main__':
    main()
