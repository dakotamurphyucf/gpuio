"""Public checkable appearance acceptance. Requires actual macOS GPU/AX/input."""
import ctypes as C
from pathlib import Path
import tempfile
import time


def color_bounds(pixels, control, window, color):
    """Find a distinctive part color inside one fully visible semantic control.

    Return logical, control-relative bounds. Root focus borders must be absent
    when sampling: their color may legitimately match the indicator.
    """
    x, y, w, h = control
    wx, wy, ww, wh = window
    assert ww > 0 and wh > 0 and w > 0 and h > 0
    assert wx <= x and wy <= y and x + w <= wx + ww and y + h <= wy + wh
    sx, sy = pixels.width / ww, pixels.height / wh
    x0, y0 = round((x - wx) * sx), round((y - wy) * sy)
    x1, y1 = round((x + w - wx) * sx), round((y + h - wy) * sy)
    left, top, right, bottom = x1, y1, x0, y0
    count = 0
    for py in range(y0, y1):
        for px in range(x0, x1):
            if all(abs(a - b) <= 6 for a, b in zip(pixels.rgb(px, py), color)):
                left, top = min(left, px), min(top, py)
                right, bottom = max(right, px + 1), max(bottom, py + 1)
                count += 1
    assert count, ('indicator color absent', color, control)
    return (left - x0) / sx, (top - y0) / sy, (right - left) / sx, (bottom - top) / sy


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_focus, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent, within,
    )
    from test_canvas import screenshot
    from window_pixels import read_png

    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Selection & actions')
    names = ('Include context', 'Stream responses', 'Indicator response mode')
    roles = ('AXCheckBox', 'AXCheckBox', 'AXRadioGroup')
    reveal_gallery_control(mac, names[-1], roles[-1])
    originals = [mac.wait_find(TITLE, name, role) for name, role in zip(names, roles)]
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    number = mac.cf.CFNumberGetValue
    number.restype, number.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
    number_type = mac.cf.CFNumberGetTypeID
    number_type.restype, number_type.argtypes = C.c_ulong, []
    get_bool = mac.cf.CFBooleanGetValue
    get_bool.restype, get_bool.argtypes = C.c_bool, [C.c_void_p]
    bool_type = mac.cf.CFBooleanGetTypeID
    bool_type.restype, bool_type.argtypes = C.c_ulong, []
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-control-captures-')
    directory = Path(images) if images is not None else Path(temporary.name)
    directory.mkdir(parents=True, exist_ok=True)
    captures, cases = 0, 0

    def checked(label, expected):
        deadline, actual = time.monotonic() + 5, None
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, label, 'AXCheckBox')
            raw = mac.attr(node, 'AXValue')
            try:
                value = C.c_longlong()
                if raw and mac.type_id(raw) == bool_type():
                    actual = int(get_bool(raw))
                elif raw and mac.type_id(raw) == number_type() and number(raw, 4, C.byref(value)):
                    actual = value.value
                if actual == expected:
                    return
            finally:
                if raw:
                    mac.release(raw)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError((label, expected, actual))

    def toggle(label, expected):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        checked(label, expected)

    def identities():
        for name, role, original in zip(names, roles, originals):
            node = mac.wait_find(TITLE, name, role)
            try:
                assert equal(original, node), ('appearance replaced native control', name)
            finally:
                mac.release(node)

    def painted(name, role, color, width, height):
        nonlocal captures, cases
        reveal_gallery_control(mac, name, role)
        deadline, last_error = time.monotonic() + 5, None
        for _ in range(12):
            node, window = mac.wait_find(TITLE, name, role), mac.window(TITLE)
            try:
                bounds, window_bounds = element_rect(mac, node), element_rect(mac, window)
            finally:
                mac.release(node)
                mac.release(window)
            path = directory / f'gallery-control-appearance-{captures:03d}.png'
            screenshot(mac, path, title=TITLE)
            captures += 1
            try:
                result = color_bounds(read_png(mac, path), bounds, window_bounds, color)
                assert abs(result[2] - width) <= 2 and abs(result[3] - height) <= 2, (name, result, width, height)
            except AssertionError as error:
                # AX publication can precede physical presentation. Retry a
                # bounded number of captures, retaining the failing evidence.
                last_error = error
            else:
                cases += 1
                return result
            if time.monotonic() >= deadline:
                break
            time.sleep(.04)
        raise AssertionError((name, 'indicator pixels did not settle', last_error))

    def rejected(node, *, retired=False):
        action = mac.string('AXPress')
        try:
            status = mac.action(node, action)
            allowed = (0, -25206, -25202) if retired else (0, -25206)
            assert status in allowed, ('disabled/stale AX action', status)
        finally:
            mac.release(action)
        time.sleep(.1)
        mac.wait_text(TITLE, 'Indicator value: on')

    try:
        checked('Include context', 1)
        checked('Custom indicators', 1)
        mac.wait_text(TITLE, 'Indicator mode: balanced')
        for theme, accent in [('Light', (9, 110, 91)), ('Dark', (137, 221, 201))]:
            select_gallery_appearance(mac, theme)
            for large, size in [(False, 18), (True, 32)]:
                if large:
                    toggle('Large indicators', 1)
                positions = []
                for before in (False, True):
                    if before:
                        toggle('Labels first', 1)
                    identities()
                    positions.append(painted(names[0], roles[0], accent, size, size)[0])
                    painted(names[1], roles[1], accent, size * 2, size)
                    painted(names[2], roles[2], accent, size, size)
                assert positions[1] > positions[0] + 20, ('label order', positions)
                toggle('Labels first', 0)
            toggle('Large indicators', 0)
        toggle('Custom indicators', 0)
        identities()
        checked('Include context', 1)
        toggle('Custom indicators', 1)
        identities()
        toggle('Mixed checkbox example', 1)
        checked('Include context', 2)
        toggle('Mixed checkbox example', 0)
        checked('Include context', 1)
        focus_gallery_control(mac, 'Include context', 'AXCheckBox')
        mac.key(49)
        mac.wait_text(TITLE, 'Indicator value: off')
        checked('Include context', 0)
        checked('Stream responses', 0)
        expect_focus(mac, 'Include context', 'AXCheckBox')
        mac.key(36)
        mac.wait_text(TITLE, 'Indicator value: on')
        expect_focus(mac, 'Include context', 'AXCheckBox')
        reveal_gallery_control(mac, names[2], roles[2])
        activate(mac, within(mac, names[2], 'Fast', 'AXRadioButton'))
        mac.wait_text(TITLE, 'Indicator mode: fast')
        unavailable = within(mac, names[2], 'Unavailable', 'AXRadioButton')
        try:
            raw = mac.attr(unavailable, 'AXEnabled')
            try:
                assert raw and not get_bool(raw), 'disabled radio option remains enabled'
            finally:
                if raw:
                    mac.release(raw)
            rejected(unavailable)
            mac.wait_text(TITLE, 'Indicator mode: fast')
        finally:
            mac.release(unavailable)
        # Preserve native owners while replacing strings with rich subtrees.
        plain_heights = [element_rect(mac, node)[3] for node in originals[:2]]
        toggle('Rich control labels', 1)
        identities()
        for name, role, old_height in zip(names[:2], roles[:2], plain_heights):
            x, y, width, height = reveal_gallery_control(mac, name, role)
            assert height >= old_height + 8, ('multiline label geometry', name, height, old_height)
        for description in ('Attach the active conversation to your next request.',
                            'Read each response as it arrives.',
                            'Depth and responsiveness', 'Requires an additional model'):
            wait_absent(mac, description, 'AXStaticText')
        # Click the lower descriptive line, away from the indicator. The shared
        # Boolean changes exactly once; duplicate activation would leave it on.
        x, y, width, height = reveal_gallery_control(mac, names[0], roles[0])
        mouse = GalleryMouse(mac)
        point = (x + min(60, width / 2), y + height - 8)
        mouse.check_owner(point)
        for event in (5, 1, 2):
            mouse.send(event, point)
        mac.wait_text(TITLE, 'Indicator value: off')
        checked('Include context', 0)
        checked('Stream responses', 0)
        expect_focus(mac, 'Include context', 'AXCheckBox')
        mac.key(36)
        mac.wait_text(TITLE, 'Indicator value: on')
        reveal_gallery_control(mac, names[2], roles[2])
        activate(mac, within(mac, names[2], 'Balanced', 'AXRadioButton'))
        mac.wait_text(TITLE, 'Indicator mode: balanced')
        # The omitted Fast override still supplies its ordinary native label.
        activate(mac, within(mac, names[2], 'Fast', 'AXRadioButton'))
        mac.wait_text(TITLE, 'Indicator mode: fast')
        if images is not None:
            screenshot(mac, directory / 'gallery-rich-control-labels.png', title=TITLE)
        toggle('Enable indicator examples', 0)
        for node in originals:
            raw = mac.attr(node, 'AXEnabled')
            try:
                assert raw and not get_bool(raw), 'inherited disabled state absent from AX'
            finally:
                if raw:
                    mac.release(raw)
        rejected(originals[0])
        toggle('Enable indicator examples', 1)
        identities()
        toggle('Make indicator examples inert', 1)
        for name, role in zip(names, roles):
            wait_absent(mac, name, role)
        rejected(originals[0], retired=True)
        toggle('Make indicator examples inert', 0)
        # Removal from the AX tree retires AppKit wrappers. After exposure is
        # restored, check current state and resume identity checks with new refs.
        for index, (name, role) in enumerate(zip(names, roles)):
            node = mac.wait_find(TITLE, name, role)
            mac.release(originals[index])
            originals[index] = node
        identities()
        checked('Include context', 1)
        mac.wait_text(TITLE, 'Indicator mode: fast')
        toggle('Rich control labels', 0)
        identities()
        mac.press(TITLE, 'Runtime & windows')
        for name, role in zip(names, roles):
            wait_absent(mac, name, role)
    finally:
        for node in originals:
            mac.release(node)
        temporary.cleanup()
    print(f'GALLERY_CONTROL_APPEARANCE_OK: {cases} GPU part-bound cases, theme/size/label '
          'order and reset identity, mixed values, real Space/Return focus, radio AX '
          'activation, rich-label pointer/semantic ownership, disabled/inert fences and page retirement', flush=True)
