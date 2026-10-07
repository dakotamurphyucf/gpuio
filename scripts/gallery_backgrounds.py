"""Public data-owned backgrounds: source publication, pixels and stable identity."""
from pathlib import Path
import tempfile
import time

from test_gallery import (TITLE, activate, element_rect, focus_gallery_control,
                          reveal_gallery_control)
from test_canvas import screenshot
from window_pixels import read_png


def exercise(mac, images):
    label = 'Chart preview: Bar backgrounds'

    def press(name):
        # Branch navigation sits above the card, at window-relative y=159
        # in the comfortable layout. Preserve the card inset for its controls.
        margin = 150 if name in ('Chart families', 'Bar backgrounds') else 170
        reveal_gallery_control(mac, name, 'AXButton', top_margin=margin)
        mac.press(TITLE, name)

    def toggle(name):
        reveal_gallery_control(mac, name, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))

    def ready(direction='Vertical', policy='Exact'):
        reveal_gallery_control(mac, label, 'AXGroup')
        mac.wait_text(TITLE, f'Ready: Bar backgrounds · 24 source values · {direction} · {policy}')

    def select(expected, end=False):
        focus_gallery_control(mac, label, 'AXGroup')
        mac.key(119 if end else 115)
        mac.key(36)
        mac.wait_text(TITLE, 'Selected: Evaluations · ' + expected)

    def palette_pixels(name, present, absent=None):
        # Inspect only the actual chart bounds, excluding palette buttons and
        # surrounding page text. Source colors must survive a view-theme change
        # until explicit republishing; afterward the old accent must disappear.
        reveal_gallery_control(mac, label, 'AXGroup')
        node, window = mac.wait_find(TITLE, label, 'AXGroup'), mac.window(TITLE)
        try:
            x, y, width, height = element_rect(mac, node)
            wx, wy, ww, wh = element_rect(mac, window)
        finally:
            mac.release(node)
            mac.release(window)
        assert wx <= x and wy <= y and x + width <= wx + ww and y + height <= wy + wh
        with tempfile.TemporaryDirectory(prefix='gpuio-background-pixels-') as temporary:
            directory = Path(images) if images else Path(temporary)
            colors = [present] + ([absent] if absent is not None else [])
            for attempt in range(8):
                path = directory / f'gallery-backgrounds-{name}-{attempt}.png'
                screenshot(mac, path, title=TITLE)
                pixels = read_png(mac, path)
                sx, sy = pixels.width / ww, pixels.height / wh
                samples = [pixels.rgb(px, py)
                           for py in range(round((y - wy) * sy), round((y + height - wy) * sy), 2)
                           for px in range(round((x - wx) * sx), round((x + width - wx) * sx), 2)]
                counts = []
                for color in colors:
                    rgb = tuple((color >> shift) & 255 for shift in (16, 8, 0))
                    counts.append(sum(all(abs(a - b) <= 6 for a, b in zip(sample, rgb)) for sample in samples))
                if counts[0] > 100 and (len(counts) == 1 or counts[1] < 20):
                    print('GALLERY_BACKGROUND_PIXELS_OK', name, [hex(c) for c in colors], counts, flush=True)
                    return
                time.sleep(.05)
            raise AssertionError(('source palette did not settle', name, colors, counts))

    press('Bar backgrounds')
    ready()
    select('Batch 01 (category 101) · value 30')
    palette_pixels('initial', 0x386ac8)
    press('Update batch values')
    ready()
    mac.wait_text(TITLE, 'Sample step: 1 · Original order · Solid fills · Color applications: 0')
    select('Batch 01 (category 101) · value 33')
    press('Reorder batches')
    ready()
    mac.wait_text(TITLE, 'Sample step: 1 · Reordered')
    select('Batch 24 (category 124) · value 51')
    select('Batch 01 (category 101) · value 33', end=True)
    toggle('Highlight Batch 01')
    ready()
    palette_pixels('sparse-after-reorder', 0xf59e0b)
    select('Batch 01 (category 101) · value 33', end=True)
    toggle('Highlight Batch 01')
    press('Reorder batches')
    ready()

    theme = mac.find(TITLE, 'Dark', 'AXButton')
    initial = 'Dark' if theme else 'Light'
    if theme:
        mac.release(theme)
    alternate = 'Light' if initial == 'Dark' else 'Dark'
    previous = 0x386ac8
    for current, next_label in [(initial, alternate), (alternate, initial)]:
        mac.press(TITLE, current)
        mac.release(mac.wait_find(TITLE, next_label, 'AXButton'))
        ready()
        palette_pixels('before-apply-' + next_label.lower(), previous)
        press('Apply preview colors')
        ready()
        next_color = 0x096e5b if next_label == 'Light' else 0x89ddc9
        palette_pixels('after-apply-' + next_label.lower(), next_color, previous)
        previous = next_color
        select('Batch 01 (category 101) · value 33')

    press('Pattern backgrounds')
    ready()
    mac.wait_text(TITLE, 'Sample step: 1 · Original order · Patterns · Color applications: 2')
    for control, direction in [
            ('Horizontal background bars', 'Horizontal'),
            ('Reverse background axis', 'Horizontal reversed'),
            ('Horizontal background bars', 'Vertical reversed'),
            ('Reverse background axis', 'Vertical')]:
        toggle(control)
        ready(direction)
        select('Batch 01 (category 101) · value 33')
        if images:
            screenshot(mac, images / ('gallery-backgrounds-pattern-' + direction.lower().replace(' ', '-') + '.png'), title=TITLE)
    toggle('Mean background bars')
    ready(policy='Mean')
    select('4 categories selected')
    toggle('Uniform background agreement')
    ready(policy='Mean')
    select('4 categories selected')
    toggle('Mean background bars')
    ready()
    select('Batch 01 (category 101) · value 33')
    press('View data')
    mac.release(mac.wait_find(TITLE, label + ' · original data', 'AXTable'))
    mac.key(119)
    mac.release(mac.wait_find(TITLE, 'Row 24:', 'AXRow', contains=True, search_files=True))
    press('Back to chart')
    select('Batch 01 (category 101) · value 33')

    # Switching branches retires the source. Bonsai fixture choices persist,
    # while reacquisition clears the committed selection and republishes data.
    press('Chart families')
    press('Line')
    reveal_gallery_control(mac, 'Chart preview: Line', 'AXGroup')
    mac.wait_text(TITLE, 'Ready: Line · 48 source values')
    press('Bar backgrounds')
    ready()
    mac.wait_text(TITLE, 'Sample step: 1 · Original order · Patterns · Color applications: 2')
    mac.wait_text(TITLE, 'Select a batch to inspect its original value.')
    select('Batch 01 (category 101) · value 33')
    print('GALLERY_BACKGROUND_SOURCE_OK: update/reorder/themes/patterns/directions/aggregation/original-data/reacquisition', flush=True)
