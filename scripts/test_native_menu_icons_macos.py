#!/usr/bin/env python3
"""Real AppKit icon pixels, retained registration lifetime and command delivery."""
import argparse
import json
import subprocess
import time
from pathlib import Path
from test_agent_chat import Mac
from test_gallery import GalleryMouse, element_rect
from test_menu_controller_macos import TITLE, select_run, wait_enabled
from test_native_popup_macos import wait_popup
from window_pixels import read_png


def capture(mac, menu, path):
    rows = mac.children(menu)
    try:
        assert [mac.text(row, 'AXTitle') for row in rows] == ['Original actions', 'Run']
        row = element_rect(mac, rows[1])
    finally:
        for item in rows:
            mac.release(item)
    bounds = element_rect(mac, menu)
    GalleryMouse(mac).check_owner((row[0] + row[2] / 2, row[1] + row[3] / 2))
    subprocess.run(['screencapture', '-x', '-R' + ','.join(str(round(n)) for n in bounds), str(path)],
                   check=True, timeout=10)
    pixels = read_png(mac, path)
    sx, sy = pixels.width / bounds[2], pixels.height / bounds[3]
    top = row[1] - bounds[1]
    background = pixels.rgb(8 * sx, (top + 11) * sy)
    # Compare the triangle silhouette, not just foreground area: AppKit shifts
    # "Run" left when no row has an image, putting text in the former icon slot.
    count = matched = total = 0
    for y in range(round((top + 4) * sy), round((top + 18) * sy)):
        for x in range(round(14 * sx), round(32 * sx)):
            lx, ly = (x + .5) / sx, (y + .5) / sy - top
            expected = 17 <= lx <= 28 and abs(ly - 11) <= 6 * (28 - lx) / 11
            actual = max(abs(a - b) for a, b in zip(pixels.rgb(x, y), background)) > 60
            count += actual
            matched += actual == expected
            total += 1
    return {'bounds': bounds, 'row': row, 'scale': [sx, sy], 'contrast_pixels': count,
            'template_match': matched / total}


def exercise(mac, output):
    mac.wait_text(TITLE, 'Ready')
    mac.set(mac.app, 'AXFrontmost', mac.true)
    window = mac.window(TITLE)
    try:
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    wait_enabled(mac, 'Release icon source')
    wait_enabled(mac, 'Show first position')
    evidence = {}

    def snapshot(name, present, retry=False):
        deadline = time.monotonic() + 5
        attempt = 0
        while True:
            mac.press(TITLE, 'Show first position')
            menu = wait_popup(mac, title=TITLE)
            try:
                result = capture(mac, menu, output / f'{name}-{attempt}.png')
            finally:
                mac.release(menu)
            mac.key(53)
            wait_popup(mac, False, title=TITLE)
            score = result['template_match']
            if (score > .92 if present else score < .85):
                evidence[name] = result
                return
            if not retry or time.monotonic() > deadline:
                raise AssertionError((name, present, result))
            attempt += 1
            time.sleep(.05)

    snapshot('ready', True, retry=True)
    mac.press(TITLE, 'Hide menu icons')
    wait_enabled(mac, 'Show menu icons')
    snapshot('cleared', False)
    mac.press(TITLE, 'Show menu icons')
    wait_enabled(mac, 'Hide menu icons')
    snapshot('restored', True, retry=True)
    mac.press(TITLE, 'Show first position')
    menu = wait_popup(mac, title=TITLE)
    try:
        mac.press(TITLE, 'Release icon source')
        button = mac.wait_find(TITLE, 'Icon source released', 'AXButton')
        mac.release(button)
        result = capture(mac, menu, output / 'released-during-tracking.png')
        assert result['template_match'] > .92, result
        evidence['released_during_tracking'] = result
    finally:
        mac.release(menu)
    mac.key(53)
    wait_popup(mac, False, title=TITLE)
    snapshot('retained-after-release', True)
    mac.press(TITLE, 'Hide menu icons')
    wait_enabled(mac, 'Show menu icons')
    mac.press(TITLE, 'Show menu icons')
    wait_enabled(mac, 'Hide menu icons')
    snapshot('new-reader-after-release', False)
    mac.press(TITLE, 'Show first position')
    menu = wait_popup(mac, title=TITLE)
    try:
        select_run(mac, menu)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=TITLE)
    mac.wait_text(TITLE, 'Total runs: 1')
    mac.press(TITLE, 'Close window')
    return evidence


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, default=Path('_build/default/examples/menu_controller/main.exe'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    Mac.require_accessibility()
    with (args.output / 'application.log').open('w') as log:
        child = subprocess.Popen([str(args.binary.resolve())], stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Mac(child.pid, child)
            evidence = exercise(mac, args.output)
            assert child.wait(timeout=15) == 0
            (args.output / 'report.json').write_text(json.dumps({'complete': True, 'cases': evidence}, indent=2) + '\n')
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
    print('GPUIO_NATIVE_MENU_ICONS_OK')


if __name__ == '__main__':
    main()
