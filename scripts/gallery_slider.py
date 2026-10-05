"""Public retained sliders: actual macOS keyboard/pointer/AX and GPU samples."""
import ctypes as C
import json
import math
from pathlib import Path
import tempfile
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_enabled, expect_focus,
        focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
        wait_absent,
    )
    from test_canvas import screenshot
    from test_numeric import Numeric
    from window_pixels import read_png

    single, lower, upper = 'Preview level', 'Preview interval lower', 'Preview interval upper'
    labels = (single, lower, upper)
    mouse = GalleryMouse(mac)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-slider-paint-')
    directory = Path(images) if images else Path(temporary.name)
    directory.mkdir(parents=True, exist_ok=True)
    samples = []

    def node(label, role='AXSlider'):
        return mac.wait_find(TITLE, label, role)

    def rect(label, role='AXSlider'):
        owner = node(label, role)
        try:
            return element_rect(mac, owner)
        finally:
            mac.release(owner)

    def value(label):
        owner = node(label)
        try:
            return Numeric.number(mac, owner)
        finally:
            mac.release(owner)

    def expect_value(label, expected, bounds=None):
        deadline, actual = time.monotonic() + 5, None
        while time.monotonic() < deadline:
            actual = value(label)
            if actual == expected:
                if bounds:
                    owner = node(label)
                    try:
                        assert (Numeric.number(mac, owner, 'AXMinValue'),
                                Numeric.number(mac, owner, 'AXMaxValue')) == bounds
                    finally:
                        mac.release(owner)
                return
            time.sleep(.03)
        raise AssertionError((label, expected, actual))

    def set_value(label, number):
        owner = node(label)
        create = mac.cf.CFNumberCreate
        create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, C.c_void_p]
        value = C.c_double(number)
        encoded = create(None, 13, C.byref(value))
        assert encoded
        try:
            mac.set(owner, 'AXValue', encoded)
        finally:
            mac.release(encoded)
            mac.release(owner)

    def setting(label, expected):
        # Config changes use AX actions even outside the viewport. Physical
        # pointer/keyboard/paint checks reveal their actual slider separately.
        owner = node(label, 'AXCheckBox')
        raw = mac.attr(owner, 'AXValue')
        try:
            assert raw
            if bool(boolean(raw)) != expected:
                activate(mac, mac.retain(owner))
        finally:
            if raw:
                mac.release(raw)
            mac.release(owner)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            owner = node(label, 'AXCheckBox')
            raw = mac.attr(owner, 'AXValue')
            try:
                if raw and bool(boolean(raw)) == expected:
                    return
            finally:
                if raw:
                    mac.release(raw)
                mac.release(owner)
            time.sleep(.025)
        raise AssertionError(('slider setting', label, expected))

    def identities(originals, retained=True):
        for label, original in zip(labels, originals):
            owner = node(label)
            try:
                assert bool(equal(original, owner)) == retained, ('slider identity', label, retained)
            finally:
                mac.release(owner)

    def focus(label):
        group = single if label == single else 'Preview interval'
        reveal_gallery_control(mac, group, 'AXGroup')
        focus_gallery_control(mac, label, 'AXSlider')

    def drag(destination, *, cancel=False, vertical=False, logarithmic=False):
        reveal_gallery_control(mac, single, 'AXGroup')
        x, y, width, height = rect(single)
        start = (x + width / 2, y + height / 2)
        gx, gy, gw, gh = rect(single, 'AXGroup')
        committed = value(single)
        fraction = math.log10(destination) / 3 if logarithmic else destination / 100
        current = math.log10(committed) / 3 if logarithmic else committed / 100
        # AX thumb bounds can be rounded. A drag keeps the initial grab offset;
        # translate the actual start rather than assuming its center is exact.
        end = ((start[0], start[1] - (gh - height) * (fraction - current))
               if vertical else (start[0] + (gw - width) * (fraction - current), start[1]))
        mouse.check_owner(start)
        mouse.check_owner(end)
        mouse.send(5, start)
        mouse.send(1, start)
        try:
            mouse.send(6, end)  # Left-dragged event while capture is held.
            expect_value(single, destination)
            mac.wait_text(TITLE, f'Preview: {destination} · committed: {committed:g}')
            if cancel:
                mac.key(53)
                expect_value(single, committed)
        finally:
            mouse.send(2, end)
        final = committed if cancel else destination
        expect_value(single, final)
        mac.wait_text(TITLE, f'Value: {final:g} · committed: {final:g}')

    def paint(case, group, thumbs, vertical, large, remaining, track, fill, thumb_color):
        reveal_gallery_control(mac, group, 'AXGroup')
        gx, gy, gw, gh = rect(group, 'AXGroup')
        # Remove hover without stealing focus, and sample away from ring paint.
        outside = (gx + gw + 15, gy + gh / 2)
        mouse.check_owner(outside)
        mouse.send(5, outside)
        target = rect(thumbs[0])[2]
        assert abs(target - (40 if large else 20)) <= 1, ('thumb target size', case, target)
        positions = []
        for fraction in (.1, .5, .9):
            point = ((gx + gw / 2, gy + gh - target / 2 - (gh - target) * fraction)
                     if vertical else (gx + target / 2 + (gw - target) * fraction, gy + gh / 2))
            selected = (.2 < fraction < .8 if len(thumbs) == 2
                        else fraction > .35 if remaining else fraction < .35)
            positions.append((point, fill if selected else track))
        for label in thumbs:
            x, y, w, h = rect(label)
            positions.append(((x + w / 2, y + h / 2), thumb_color))
        last = None
        for attempt in range(8):
            window = mac.window(TITLE)
            try:
                wx, wy, ww, wh = element_rect(mac, window)
            finally:
                mac.release(window)
            path = directory / f'{case}-{attempt}.png'
            screenshot(mac, path, title=TITLE)
            pixels = read_png(mac, path)
            actual = [pixels.rgb(round((x - wx) * pixels.width / ww),
                                 round((y - wy) * pixels.height / wh))
                      for (x, y), _ in positions]
            last = actual
            if all(all(abs(a - b) <= 6 for a, b in zip(rgb, color))
                   for rgb, (_, color) in zip(actual, positions)):
                samples.append({'case': case, 'samples': positions, 'actual': actual,
                                'group_bounds': (gx, gy, gw, gh), 'capture': path.name})
                print('GALLERY_SLIDER_PAINT_CASE', case, 'PASS', flush=True)
                return
            time.sleep(.05)
        raise AssertionError((case, positions, last))

    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Numbers & codes')
    for label in ('Read-only numeric inputs', 'Vertical sliders', 'Logarithmic scale',
                  'Disable sliders', 'Fill the remaining amount (single value)',
                  'Separate track and thumb colors', 'Larger slider thumbs'):
        setting(label, False)
    originals = [node(label) for label in labels]
    try:
        expect_value(single, 35, (0, 100))
        expect_value(lower, 20, (0, 80))
        expect_value(upper, 80, (20, 100))
        focus(lower)
        mac.key(124)
        expect_value(lower, 21)
        mac.key(48)
        expect_focus(mac, upper, 'AXSlider')
        mac.key(123)
        expect_value(upper, 79, (21, 100))
        mac.key(48, flags=1 << 17)
        expect_focus(mac, lower, 'AXSlider')
        set_value(lower, 90)
        expect_value(lower, 79, (0, 79))
        set_value(lower, 20)
        set_value(upper, 80)
        focus(single)
        for code, expected in ((115, 0), (119, 100), (121, 90), (116, 100)):
            mac.key(code)
            expect_value(single, expected)
        set_value(single, 50)
        drag(75)
        drag(25, cancel=True)
        setting('Read-only numeric inputs', True)
        focus(single)
        mac.key(123)
        set_value(single, 10)
        expect_value(single, 75)
        expect_enabled(mac, single, True, role='AXSlider')
        setting('Read-only numeric inputs', False)
        setting('Disable sliders', True)
        for label in labels:
            expect_enabled(mac, label, False, role='AXSlider')
        set_value(single, 10)
        expect_value(single, 75)
        setting('Disable sliders', False)
        identities(originals)
        setting('Vertical sliders', True)
        setting('Logarithmic scale', True)
        focus(single)
        mac.key(119)
        expect_value(single, 1000, (1, 1000))
        mac.key(121)
        expect_value(single, 990)
        drag(10, vertical=True, logarithmic=True)
        drag(100, cancel=True, vertical=True, logarithmic=True)
        set_value(single, 1000)
        expect_value(single, 1000)
        setting('Logarithmic scale', False)
        expect_value(single, 100, (0, 100))
        set_value(single, 35)
        set_value(lower, 20)
        set_value(upper, 80)
        setting('Separate track and thumb colors', True)
        for theme, track, fill, thumb_color in (
            ('Light', (211, 220, 229), (9, 110, 91), (27, 41, 57)),
            ('Dark', (45, 57, 73), (137, 221, 201), (234, 240, 247)),
        ):
            select_gallery_appearance(mac, theme)
            for vertical in (False, True):
                setting('Vertical sliders', vertical)
                for large in (False, True):
                    setting('Larger slider thumbs', large)
                    for remaining in (False, True):
                        setting('Fill the remaining amount (single value)', remaining)
                        identities(originals)
                        case = f'{theme}-{"vertical" if vertical else "horizontal"}-{"large" if large else "small"}-{"remaining" if remaining else "selected"}'
                        paint(case + '-single', single, (single,), vertical, large, remaining, track, fill, thumb_color)
                        paint(case + '-range', 'Preview interval', (lower, upper), vertical, large, remaining, track, fill, thumb_color)
        for label in ('Vertical sliders', 'Larger slider thumbs',
                      'Fill the remaining amount (single value)', 'Separate track and thumb colors'):
            setting(label, False)
        identities(originals)
        set_value(single, 65)
        mac.press(TITLE, 'Runtime & windows')
        for label in labels:
            wait_absent(mac, label, 'AXSlider')
        mac.press(TITLE, 'Numbers & codes')
        identities(originals, retained=False)
        expect_value(single, 35)
        expect_value(lower, 20)
        expect_value(upper, 80)
        focus(single)
        mac.key(124)
        expect_value(single, 36)
        print(f'GALLERY_SLIDER_OK: {len(samples)} GPU cases; range Tab/limits, '
              'OS keys, pointer preview/commit/Escape, read-only/disabled fencing, '
              'axis/log domain updates, retained owners, page retirement/remount', flush=True)
    finally:
        for original in originals:
            mac.release(original)
        (directory / 'slider-paint-samples.json').write_text(json.dumps(samples, indent=2) + '\n')
        temporary.cleanup()
