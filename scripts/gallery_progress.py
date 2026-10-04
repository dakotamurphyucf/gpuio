"""Public progress/editor lifecycle and ring pixels; requires a macOS desktop.

Import/compile checks do not execute or qualify these native assertions.
"""
import ctypes as C
import math
from pathlib import Path
import tempfile
import time


def ring_samples(pixels, root, window):
    """Sample the ring only, excluding the editable center and blinking caret."""
    x, y, width, height = root
    wx, wy, ww, wh = window
    assert ww > 0 and wh > 0
    assert wx <= x and wy <= y and x + width <= wx + ww and y + height <= wy + wh
    outer = min(width, height) / 2
    thickness = min(min(width, height) * .15, 5)
    radius = outer - thickness / 2
    assert radius > 0
    return tuple(pixels.rgb(
        (x - wx + width / 2 + radius * math.cos(i * math.tau / 64)) * pixels.width / ww,
        (y - wy + height / 2 + radius * math.sin(i * math.tau / 64)) * pixels.height / wh,
    ) for i in range(64))


def changed(before, after):
    assert len(before) == len(after)
    return sum(max(abs(a - b) for a, b in zip(p, q)) > 5
               for p, q in zip(before, after))


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, element_rect, expect_field, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot
    from window_pixels import read_png

    mac.press(TITLE, 'Motion & rhythm')
    mac.press(TITLE, 'Use full motion')
    mac.wait_text(TITLE, 'Motion preference: Full')
    mac.press(TITLE, 'Commands & feedback')
    label, role = 'Circular transfer progress', 'AXProgressIndicator'
    draft, field_role = 'Progress center draft', 'AXTextField'
    reveal_gallery_control(mac, label, role)
    original = mac.wait_find(TITLE, label, role)
    editor = None
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-progress-captures-')
    directory = Path(images) if images is not None else Path(temporary.name)
    directory.mkdir(parents=True, exist_ok=True)
    capture_index = 0

    def identity():
        root = mac.wait_find(TITLE, label, role)
        field = mac.wait_find(TITLE, draft, field_role)
        try:
            assert equal(original, root), 'Progress value/style update replaced the AX root'
            assert equal(editor, field), 'Progress update replaced the center editor'
            bounds, child = element_rect(mac, root), element_rect(mac, field)
            x, y, w, h = bounds
            assert abs(w - 160) < 1.1 and abs(h - 160) < 1.1, bounds
            assert abs(child[0] + child[2] / 2 - (x + w / 2)) < 1.1, (bounds, child)
            assert x <= child[0] and child[0] + child[2] <= x + w + 1
            assert y <= child[1] and child[1] + child[3] <= y + h + 1
            return bounds
        finally:
            mac.release(root)
            mac.release(field)

    def numeric(name, expected):
        get = mac.cf.CFNumberGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        number_type = mac.cf.CFNumberGetTypeID
        number_type.restype, number_type.argtypes = C.c_ulong, []
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, name, role)
            value = mac.attr(node, 'AXValue')
            try:
                if expected is None:
                    if not value:
                        return
                elif value and mac.type_id(value) == number_type():
                    result = C.c_double()
                    assert get(value, 6, C.byref(result))  # kCFNumberFloat64Type
                    if abs(result.value - expected) < .001:
                        return
            finally:
                if value:
                    mac.release(value)
                mac.release(node)
            time.sleep(.03)
        raise AssertionError(f'{name}: AXValue did not become {expected}')

    def capture():
        nonlocal capture_index
        reveal_gallery_control(mac, label, role)
        root = identity()
        window = mac.window(TITLE)
        try:
            bounds = element_rect(mac, window)
        finally:
            mac.release(window)
        path = directory / f'gallery-progress-{capture_index:03d}.png'
        capture_index += 1
        screenshot(mac, path, title=TITLE)
        return ring_samples(read_png(mac, path), root, bounds)

    def static():
        time.sleep(.8)  # The public preview's value tween lasts 600ms.
        before = capture()
        time.sleep(.2)
        assert changed(before, capture()) == 0, 'Settled progress ring kept changing'
        return before

    def target(button, percentage):
        mac.press(TITLE, button)
        numeric(label, percentage)
        numeric('Rounded transfer progress', percentage)
        identity()
        expect_field(mac, TITLE, draft, 'a')

    try:
        editor = mac.wait_find(TITLE, draft, field_role)
        focus_gallery_control(mac, draft, field_role)
        mac.key(0, flags=1 << 20)  # Real OS select-all and typing, not AXValue assignment.
        mac.key(0)
        expect_field(mac, TITLE, draft, 'a')
        for _ in range(2):
            target('Empty', 0)
            empty = static()
            target('Complete', 100)
            full = static()
            assert changed(empty, full) >= 48, 'Determinate value did not fill the ring'
            for button, value in [('Tiny', 1), ('Quarter', 25), ('Nearly there', 85)]:
                target(button, value)
                static()
            # The shell names its current theme; pressing it toggles to the other.
            theme = mac.find(TITLE, 'Light', 'AXButton')
            if not theme:
                theme = mac.wait_find(TITLE, 'Dark', 'AXButton')
            activate(mac, theme)
            identity()
        target('Unknown', None)
        before = capture()
        deadline = time.monotonic() + 5
        while changed(before, capture()) < 8:
            assert time.monotonic() < deadline, 'Indeterminate ring did not visibly animate'
            time.sleep(.12)
        activate(mac, mac.wait_find(TITLE, 'Inert progress preview', 'AXCheckBox'))
        wait_absent(mac, label, role)
        wait_absent(mac, draft, field_role)
        activate(mac, mac.wait_find(TITLE, 'Inert progress preview', 'AXCheckBox'))
        identity()
        expect_field(mac, TITLE, draft, 'a')
        target('Quarter', 25)
        focus_gallery_control(mac, 'Animate value changes', 'AXCheckBox')
        mac.key(49)  # Real Space changes the public transition policy.
        target('Complete', 100)
        static()
        focus_gallery_control(mac, draft, field_role)
        mac.key(0, flags=1 << 20)
        mac.key(11)  # The retained editor still accepts keyboard input.
        expect_field(mac, TITLE, draft, 'b')
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, label, role)
        wait_absent(mac, draft, field_role)
        mac.press(TITLE, 'Commands & feedback')
        reveal_gallery_control(mac, label, role)
        for name, kind, old in [(label, role, original), (draft, field_role, editor)]:
            node = mac.wait_find(TITLE, name, kind)
            try:
                assert not equal(old, node), f'Retired AX object reused for {name}'
            finally:
                mac.release(node)
        focus_gallery_control(mac, draft, field_role)
        mac.key(0, flags=1 << 20)
        mac.key(8)
        expect_field(mac, TITLE, draft, 'c')
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, label, role)
        wait_absent(mac, draft, field_role)
    finally:
        if editor:
            mac.release(editor)
        mac.release(original)
        temporary.cleanup()
    print('GALLERY_PROGRESS_OK: native values, circle geometry/pixels, editor identity '
          'and keyboard edits, themes, indeterminate motion, inert AX retirement, '
          'transition policy and page remount', flush=True)
