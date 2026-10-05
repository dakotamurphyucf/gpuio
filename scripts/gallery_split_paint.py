"""Physical macOS pixel checks for the public split-button composition."""
from pathlib import Path
import json
import tempfile
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_enabled,
        focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
        wait_absent,
    )
    from test_canvas import screenshot
    from window_pixels import read_png

    names = ('Run split action', 'More split actions')
    mac.press(TITLE, 'Selection & actions')
    reveal_gallery_control(mac, names[1], 'AXButton')
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-split-paint-')
    directory = Path(images) if images is not None else Path(temporary.name)
    directory.mkdir(parents=True, exist_ok=True)
    mouse = GalleryMouse(mac)
    samples = []

    def bounds():
        result = []
        for name in names:
            node = mac.wait_find(TITLE, name, 'AXButton')
            try:
                result.append(element_rect(mac, node))
            finally:
                mac.release(node)
        a, b = result
        assert abs(a[0] + a[2] - b[0]) <= 1, ('split seam gap', result)
        assert abs(a[1] - b[1]) <= 1 and abs(a[3] - b[3]) <= 1, ('split alignment', result)
        return result

    def move(part=None):
        a, b = bounds()
        if part is None:
            point = (b[0] + b[2] + 30, b[1] + b[3] / 2)
        else:
            x, y, width, height = (a, b)[part]
            point = (x + width / 2, y + height / 2)
        mouse.check_owner(point)
        mouse.send(5, point)

    def paint(case, expected):
        last = None
        for attempt in range(8):
            rects = bounds()
            window = mac.window(TITLE)
            try:
                wx, wy, ww, wh = element_rect(mac, window)
            finally:
                mac.release(window)
            path = directory / f'{case}-{attempt}.png'
            screenshot(mac, path, title=TITLE)
            pixels = read_png(mac, path)
            actual = []
            for x, y, width, height in rects:
                assert wx <= x and wy <= y and x + width <= wx + ww and y + height <= wy + wh
                # Sample a 3x3 patch inside left padding, away from the border,
                # label and focus outline. Every pixel must match the surface.
                actual.append([pixels.rgb(round((x + 8 - wx) * pixels.width / ww) + dx,
                                          round((y + height / 2 - wy) * pixels.height / wh) + dy)
                               for dy in (-1, 0, 1) for dx in (-1, 0, 1)])
            last = actual
            if all(all(all(abs(a - b) <= 6 for a, b in zip(pixel, color))
                       for pixel in patch) for patch, color in zip(actual, expected)):
                samples.append({'case': case, 'expected': expected, 'actual': actual,
                                'bounds': rects, 'capture': path.name})
                print('GALLERY_SPLIT_PAINT_CASE', case, 'PASS', flush=True)
                return
            time.sleep(.06)  # Bounded wait for actual presentation after input.
        raise AssertionError((case, 'split paint did not settle', expected, last))

    def toggle(label):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        reveal_gallery_control(mac, names[1], 'AXButton')

    try:
        for theme, resting, shared, accent in (
            ('Light', (255, 255, 255), (211, 220, 229), (9, 110, 91)),
            ('Dark', (25, 33, 44), (45, 57, 73), (137, 221, 201)),
        ):
            select_gallery_appearance(mac, theme)
            focus_gallery_control(mac, 'Disable split pair', 'AXCheckBox')
            reveal_gallery_control(mac, names[1], 'AXButton')
            move()
            paint(theme + '-rest', (resting, resting))
            move(0)
            paint(theme + '-hover-primary', (accent, shared))
            move(1)
            paint(theme + '-hover-menu', (shared, accent))
            move()
            paint(theme + '-blank-outside', (resting, resting))
            focus_gallery_control(mac, names[1], 'AXButton')
            mac.key(125)
            mac.release(mac.wait_find(TITLE, names[1], 'AXMenu'))
            move()
            paint(theme + '-menu-held', (shared, accent))
            mac.key(53)
            wait_absent(mac, names[1], 'AXMenu')
            paint(theme + '-menu-dismissed', (resting, resting))
            toggle('Disable primary action')
            expect_enabled(mac, names[0], False)
            move(1)
            paint(theme + '-primary-disabled', (resting, accent))
            toggle('Disable primary action')
            expect_enabled(mac, names[0], True)
            toggle('Primary action loading')
            move(1)
            paint(theme + '-primary-loading', (resting, accent))
            toggle('Primary action loading')
            toggle('Disable split pair')
            for name in names:
                expect_enabled(mac, name, False)
            move(1)
            paint(theme + '-pair-disabled', (resting, resting))
            toggle('Disable split pair')
            for name in names:
                expect_enabled(mac, name, True)
            move(1)
            paint(theme + '-recovered', (shared, accent))
        mac.press(TITLE, 'Presentation')
        for name in names:
            wait_absent(mac, name, 'AXButton')
        wait_absent(mac, names[1], 'AXMenu')
        print(f'GALLERY_SPLIT_PAINT_OK: {len(samples)} Light/Dark GPU cases, '
              'native hover and menu-held surfaces, per-part disabled/loading, '
              'pair disable/recovery, joined geometry and page retirement', flush=True)
    finally:
        (directory / 'split-paint-samples.json').write_text(json.dumps(samples, indent=2) + '\n')
        temporary.cleanup()
