"""Installed color palette: physical hover pixels, retained draft and tab keys."""
import ctypes as C
import json
from pathlib import Path
import tempfile
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_field, expect_focus,
        focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
        wait_absent, within,
    )
    from test_canvas import screenshot
    from window_pixels import read_png

    mac.press(TITLE, 'Presentation')
    mac.press(TITLE, 'Dates & colors')
    for current, following in (('Large', 'Compact'), ('Compact', 'Comfortable')):
        node = mac.find(TITLE, current, 'AXButton')
        if node:
            activate(mac, node)
            mac.release(mac.wait_find(TITLE, following, 'AXButton'))
    mac.release(mac.wait_find(TITLE, 'Comfortable', 'AXButton'))
    group = 'Preview palette'
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-color-preview-')
    directory = Path(images) if images else Path(temporary.name)
    directory.mkdir(parents=True, exist_ok=True)
    samples = []
    mouse = GalleryMouse(mac)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    boolean_type = mac.cf.CFBooleanGetTypeID
    boolean_type.restype, boolean_type.argtypes = C.c_ulong, []

    def rect(label, role):
        node = within(mac, group, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def flag(label, attribute, expected):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = within(mac, group, label, 'AXRadioButton')
            raw = mac.attr(node, attribute)
            try:
                assert raw and mac.type_id(raw) == boolean_type(), (label, attribute)
                if bool(boolean(raw)) == expected:
                    return
            finally:
                if raw:
                    mac.release(raw)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError((label, attribute, expected))

    def move(label=None):
        if label:
            x, y, w, h = rect(label, 'AXRadioButton')
            point = (x + w / 2, y + h / 2)
        else:
            x, y, w, h = rect('Hex color', 'AXTextField')
            point = (x + w + 25, y + h / 2)
        mouse.check_owner(point)
        mouse.send(5, point)

    def unchanged(draft):
        current = within(mac, group, 'Hex color', 'AXTextField')
        owner = mac.wait_find(TITLE, group, 'AXGroup')
        try:
            assert equal(editor, current), 'Color editor replaced'
            assert mac.text(current, 'AXValue') == draft, ('Unexpected draft', draft)
            assert mac.text(owner, 'AXValue') == '#89DDC9', 'Hover changed the color model'
        finally:
            mac.release(owner)
            mac.release(current)
        flag('Mint', 'AXValue', True)
        flag('Iris', 'AXValue', False)
        flag('Coral', 'AXValue', False)

    def paint(case, expected, factor, reference, *, draft='1'):
        last = None
        for attempt in range(8):
            owner = mac.wait_find(TITLE, group, 'AXGroup')
            window = mac.window(TITLE)
            try:
                gx, gy, gw, gh = element_rect(mac, owner)
                wx, wy, ww, wh = element_rect(mac, window)
            finally:
                mac.release(owner)
                mac.release(window)
            hx, hy, hw, hh = rect('Hex color', 'AXTextField')
            # Public gallery padding scales; the native preview itself is 30pt.
            px, py = gx + 10 * factor + 15, hy + hh / 2
            assert wx < px < wx + ww and wy + 130 < py < wy + wh - 20
            path = directory / f'{case}-{attempt}.png'
            screenshot(mac, path, title=TITLE)
            pixels = read_png(mac, path)
            last = [pixels.rgb(round((px - wx) * pixels.width / ww) + dx,
                               round((py - wy) * pixels.height / wh) + dy)
                    for dy in (-1, 0, 1) for dx in (-1, 0, 1)]
            if all(all(abs(a - b) <= 6 for a, b in zip(rgb, expected)) for rgb in last):
                unchanged(draft)
                for label, before in reference.items():
                    after = rect(label, 'AXRadioButton')
                    assert all(abs(a - b) <= .5 for a, b in zip(after, before)), (
                        'Hover shifted palette layout', label, before, after)
                samples.append({'case': case, 'expected': expected, 'actual': last,
                                'point': [px, py], 'capture': path.name})
                print('GALLERY_COLOR_PREVIEW_CASE', case, 'PASS', flush=True)
                return
            time.sleep(.06)
        raise AssertionError((case, 'preview pixels did not settle', expected, last))

    reveal_gallery_control(mac, 'Hex color', 'AXTextField')
    focus_gallery_control(mac, 'Hex color', 'AXTextField')
    editor = within(mac, group, 'Hex color', 'AXTextField')
    try:
        mac.key(0, flags=1 << 20)  # Cmd+A then an actual OS digit key.
        mac.key(18)
        expect_field(mac, TITLE, 'Hex color', '1')
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for scale, next_scale, factor in (
                ('Comfortable', 'Large', 1.), ('Large', 'Compact', 1.2),
                ('Compact', 'Comfortable', .85),
            ):
                # Reveal the swatches as well as the preceding preview/editor;
                # revealing only the editor can leave Favorites below the window.
                reveal_gallery_control(mac, 'Hex color', 'AXTextField')
                reveal_gallery_control(mac, 'Iris', 'AXRadioButton')
                reference = {name: rect(name, 'AXRadioButton')
                             for name in ('Mint', 'Iris', 'Coral', 'Stone shade 1')}
                for name, expected_size in (('Mint', 36 * factor), ('Stone shade 1', 28 * factor)):
                    assert all(abs(size - expected_size) <= 1 for size in reference[name][2:]), (
                        name, scale, reference[name], expected_size)
                move()
                paint(f'{theme}-{scale}-rest', (137, 221, 201), factor, reference)
                move('Iris')
                paint(f'{theme}-{scale}-iris', (163, 181, 255), factor, reference)
                move('Coral')
                paint(f'{theme}-{scale}-coral', (246, 168, 157), factor, reference)
                move()
                paint(f'{theme}-{scale}-leave', (137, 221, 201), factor, reference)
                # The passive hover caption must not become a selected value or
                # an independently exposed text node.
                wait_absent(mac, '#A3B5FF', 'AXStaticText')
                mac.press(TITLE, scale)
                mac.release(mac.wait_find(TITLE, next_scale, 'AXButton'))

        # Hover and presentation changes must preserve real native undo/redo.
        expect_focus(mac, 'Hex color', 'AXTextField')
        # This compound field exposes its draft but not AXSelectedTextRange.
        # Verify the retained insertion point through actual keyboard editing.
        mac.key(19)
        expect_field(mac, TITLE, 'Hex color', '12')
        mac.key(6, flags=1 << 20)
        expect_field(mac, TITLE, 'Hex color', '1')
        mac.key(6, flags=1 << 20)
        expect_field(mac, TITLE, 'Hex color', '#89DDC9')
        mac.key(6, flags=(1 << 20) | (1 << 17))
        expect_field(mac, TITLE, 'Hex color', '1')
        # Read-only cancels an active edit (including its discarded history), as
        # specified by the color-input contract. Hover alone above does not.
        reveal_gallery_control(mac, 'Hex color', 'AXTextField')
        reveal_gallery_control(mac, 'Iris', 'AXRadioButton')
        reference = {name: rect(name, 'AXRadioButton') for name in ('Mint', 'Iris')}
        move('Iris')
        paint('Dark-read-only-before', (163, 181, 255), 1., reference)
        activate(mac, mac.wait_find(TITLE, 'Read-only pickers', 'AXCheckBox'))
        paint('Dark-read-only-clears', (137, 221, 201), 1., reference, draft='#89DDC9')
        move('Coral')
        paint('Dark-read-only-rejects', (137, 221, 201), 1., reference, draft='#89DDC9')
        activate(mac, mac.wait_find(TITLE, 'Read-only pickers', 'AXCheckBox'))
        move()
        focus_gallery_control(mac, 'Hex color', 'AXTextField')
        mac.key(6, flags=1 << 20)
        expect_field(mac, TITLE, 'Hex color', '#89DDC9')

        # One roving tab stop: hex -> active tab -> Shift+Tab back to hex.
        mac.key(48)
        flag('Palette', 'AXFocused', True)
        mac.key(124)
        flag('HSLA', 'AXFocused', True)
        flag('HSLA', 'AXValue', True)
        mac.release(within(mac, group, 'Hue', 'AXSlider'))
        mac.key(48, flags=1 << 17)
        expect_focus(mac, 'Hex color', 'AXTextField')
        mac.key(48)
        flag('HSLA', 'AXFocused', True)
        for key, tab in ((115, 'Palette'), (119, 'HSLA'), (115, 'Palette'), (123, 'HSLA')):
            mac.key(key)
            flag(tab, 'AXFocused', True)
            flag(tab, 'AXValue', True)
        mac.key(115)
        flag('Palette', 'AXFocused', True)
        current = within(mac, group, 'Hex color', 'AXTextField')
        try:
            assert equal(editor, current), 'Panel keys replaced the hex editor'
            assert mac.text(current, 'AXValue') == '#89DDC9'
        finally:
            mac.release(current)
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, group, 'AXGroup')
        mac.press(TITLE, 'Dates & colors')
        current = within(mac, group, 'Hex color', 'AXTextField')
        try:
            assert not equal(editor, current), 'Retired palette owner reused'
            assert mac.text(current, 'AXValue') == '#89DDC9'
        finally:
            mac.release(current)
        print(f'GALLERY_COLOR_PREVIEW_OK: {len(samples)} actual GPU hover/policy cases, '
              'two themes/three scales, invalid draft/caret/history and selected color '
              'retention, native roving tab keys and owner retirement/remount', flush=True)
    finally:
        mac.release(editor)
        (directory / 'color-preview-samples.json').write_text(json.dumps(samples, indent=2) + '\n')
        temporary.cleanup()
