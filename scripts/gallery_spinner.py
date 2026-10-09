"""Public custom-spinner lifecycle and native pixels. Requires a macOS desktop."""
import ctypes as C
from pathlib import Path
import tempfile
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, element_rect, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent,
    )
    from test_canvas import screenshot
    from window_pixels import read_png

    mac.press(TITLE, 'Motion & rhythm')
    mac.press(TITLE, 'Use full motion')
    mac.wait_text(TITLE, 'Motion preference: Full')
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'Icon ready')
    label, role = 'Spinner preview', 'AXProgressIndicator'
    reveal_gallery_control(mac, label, role)
    original = mac.wait_find(TITLE, label, role)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-spinner-captures-')
    directory = images or Path(temporary.name)
    capture_index = 0

    def identity():
        node = mac.wait_find(TITLE, label, role)
        try:
            assert equal(original, node), 'Spinner update replaced the native owner'
            bounds = element_rect(mac, node)
            assert abs(bounds[2] - 40) < 1.1 and abs(bounds[3] - 40) < 1.1, bounds
            return bounds
        finally:
            mac.release(node)

    def capture():
        nonlocal capture_index
        reveal_gallery_control(mac, label, role)
        x, y, w, h = identity()
        window = mac.window(TITLE)
        try:
            wx, wy, ww, wh = element_rect(mac, window)
        finally:
            mac.release(window)
        assert wx <= x and wy < y and x+w <= wx+ww and y+h < wy+wh
        path = directory / f'gallery-spinner-{capture_index:03d}.png'
        capture_index += 1
        screenshot(mac, path, title=TITLE)
        pixels = read_png(mac, path)
        return tuple(pixels.rgb((x-wx+(ix+.5)*w/24)*pixels.width/ww,
                                (y-wy+(iy+.5)*h/24)*pixels.height/wh)
                     for iy in range(24) for ix in range(24))

    def changed(before, after):
        return sum(max(abs(a-b) for a, b in zip(p, q)) > 5
                   for p, q in zip(before, after))

    def static():
        time.sleep(.15)
        before = capture()
        time.sleep(.25)
        assert changed(before, capture()) == 0, 'Static spinner pixels changed'

    def animated():
        before = capture()
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            time.sleep(.12)
            if changed(before, capture()) >= 6:
                return
        raise AssertionError('Enabled spinner did not visibly animate')

    try:
        # Default static custom SVG. All source choices retain the same native
        # semantic node; the optional image callback reports real decode state.
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            static()
            mac.press(TITLE, 'Circular arrow')
            mac.wait_text(TITLE, 'Icon unavailable · showing fallback')
            static()
            mac.press(TITLE, 'Unavailable icon')
            mac.wait_text(TITLE, 'Built-in indicator')
            static()
            mac.press(TITLE, 'Built-in strokes')
            mac.wait_text(TITLE, 'Icon ready')
            static()
        focus_gallery_control(mac, 'Animate custom spinner', 'AXCheckBox')
        mac.key(49)  # Real OS Space -> public Bonsai effect.
        animated()
        mac.press(TITLE, 'Cycle: 800 ms')
        mac.release(mac.wait_find(TITLE, 'Cycle: 2 seconds', 'AXButton'))
        animated()
        mac.press(TITLE, 'Ease in and out')
        mac.release(mac.wait_find(TITLE, 'Linear easing', 'AXButton'))
        animated()
        activate(mac, mac.wait_find(TITLE, 'Animate custom spinner', 'AXCheckBox'))
        static()
        # Departure removes the native owner and scoped sources. Bonsai retains
        # the preview's model while the page scope acquires fresh native sources.
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, label, role)
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'Icon ready')
        node = mac.wait_find(TITLE, label, role)
        try:
            assert not equal(original, node), 'Unmounted spinner reused its AX object'
        finally:
            mac.release(node)
        mac.release(mac.wait_find(TITLE, 'Cycle: 2 seconds', 'AXButton'))
        mac.release(mac.wait_find(TITLE, 'Linear easing', 'AXButton'))
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, label, role)
    finally:
        mac.release(original)
        temporary.cleanup()
    print('GALLERY_SPINNER_OK: source failure/recovery, theme/geometry/identity, '
          'static and animated pixels, keyboard toggle, timing/easing and page retirement',
          flush=True)
