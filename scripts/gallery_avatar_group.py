"""Public avatar-group geometry, image observations and lifecycle on macOS."""
import ctypes as C
from pathlib import Path
import tempfile
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, element_rect, focus_gallery_control, reveal_gallery_control,
        select_gallery_appearance, wait_absent,
    )
    from test_canvas import screenshot
    from window_pixels import read_png

    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A familiar face. A team in a little space.')
    names = ['Ada', 'Grace', 'Yuki', 'Sam', 'Morgan']
    limit, size_index, overlap_index = 3, 2, 1
    sizes = [('XS', 16), ('S', 24), ('M', 48), ('L', 80), ('56', 56)]
    overlaps = [0, .3, .6]
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    retained = {}
    cases = 0

    def palette_pixels(theme):
        # Public stable-key fixture: "ada" maps to palette index 7. Expected
        # bytes are fixed in avatar_palette_test.ml, independent of GPU output.
        colors = ((0xDEFCFF, 0x007790, 0xB5E4ED) if theme == 'Light'
                  else (0x03343C, 0x60D8EB, 0x05454E))
        reveal_gallery_control(mac, 'Team avatars', 'AXGroup')
        node, window = member('Ada'), mac.window(TITLE)
        try:
            x, y, width, height = element_rect(mac, node)
            wx, wy, ww, wh = element_rect(mac, window)
        finally:
            mac.release(node)
            mac.release(window)
        assert wx <= x and wy <= y and x + width <= wx + ww and y + height <= wy + wh
        with tempfile.TemporaryDirectory(prefix='gpuio-avatar-palette-') as temporary:
            directory = Path(images) if images else Path(temporary)
            path = directory / f'gallery-avatar-palette-{theme.lower()}.png'
            screenshot(mac, path, title=TITLE)
            pixels = read_png(mac, path)
            sx, sy = pixels.width / ww, pixels.height / wh
            # Inspect only the left half: the following avatar overlaps the right.
            samples = [pixels.rgb(px, py)
                       for py in range(round((y - wy) * sy), round((y + height - wy) * sy))
                       for px in range(round((x - wx) * sx), round((x + width / 2 - wx) * sx))]
            counts = []
            for color in colors:
                expected = tuple((color >> shift) & 255 for shift in (16, 8, 0))
                count = sum(all(abs(a - b) <= 6 for a, b in zip(sample, expected))
                            for sample in samples)
                assert count >= 3, ('avatar palette pixels absent', theme, hex(color), count)
                counts.append(count)
            print('GALLERY_AVATAR_PALETTE_PIXELS_OK', theme, 'background/foreground/border', counts, flush=True)

    def member(name):
        return mac.wait_find(TITLE, 'Team member ' + name, 'AXImage')

    def rect(node):
        return element_rect(mac, node)

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.08)

    def change_limit():
        nonlocal limit
        mac.press(TITLE, f'Avatar limit: {limit}')
        limit = (limit + 1) % 6
        mac.release(mac.wait_find(TITLE, f'Avatar limit: {limit}', 'AXButton'))

    def geometry(order, preserve=True):
        nonlocal cases
        group_bounds = reveal_gallery_control(mac, 'Team avatars', 'AXGroup')
        size = sizes[size_index][1]
        stride = size * (1 - overlaps[overlap_index])
        visible = order[:limit]
        previous = None
        for name in visible:
            node = member(name)
            try:
                bounds = rect(node)
                assert abs(bounds[2] - size) < 1.1 and abs(bounds[3] - size) < 1.1, ('avatar dimensions', name, bounds, size)
                if previous:
                    assert abs(bounds[0] - previous[0] - stride) < 1.1, ('avatar overlap', previous, bounds, stride)
                    assert abs(bounds[1] - previous[1]) < 1.1, ('avatar alignment', previous, bounds)
                previous = bounds
                if preserve and name in retained:
                    assert equal(retained[name], node), ('surviving avatar replaced', name)
            finally:
                mac.release(node)
        for name in order[limit:]:
            absent = mac.find(TITLE, 'Team member ' + name, 'AXImage')
            if absent:
                mac.release(absent)
                raise AssertionError(('omitted avatar still exposed', name))
        omitted = len(order) - len(visible)
        if omitted:
            overflow = mac.wait_find(TITLE, f'{omitted} more teammates', 'AXImage')
            try:
                bounds = rect(overflow)
                assert abs(bounds[2] - size) < 1.1, ('overflow size', bounds, size)
                if previous:
                    assert abs(bounds[0] - previous[0] - previous[2] - 4) < 1.1, ('overflow gap', group_bounds, previous, bounds)
            finally:
                mac.release(overflow)
        cases += 1

    try:
        reveal_gallery_control(mac, 'Team avatars', 'AXGroup')
        for name in names[:limit]:
            retained[name] = member(name)
        # Shared size and overlap update the native leaves, retaining image identity.
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            geometry(names)
            palette_pixels(theme)
            for _ in range(5):
                for _ in range(3):
                    geometry(names)
                    mac.press(TITLE, f'Avatar overlap: {round(overlaps[overlap_index] * 100)}%')
                    overlap_index = (overlap_index + 1) % 3
                    mac.release(mac.wait_find(TITLE, f'Avatar overlap: {round(overlaps[overlap_index] * 100)}%', 'AXButton'))
                mac.press(TITLE, 'Avatar size: ' + sizes[size_index][0])
                size_index = (size_index + 1) % 5
                mac.release(mac.wait_find(TITLE, 'Avatar size: ' + sizes[size_index][0], 'AXButton'))
        # Source changes must not replace Ada's native avatar.
        mac.press(TITLE, 'Avatar source: Initials')
        mac.wait_text(TITLE, 'Avatar image: Ready · Team opens: 0')
        geometry(names)
        mac.press(TITLE, 'Avatar source: Image')
        mac.wait_text(TITLE, 'Avatar image: Failed: Invalid_data · Team opens: 0')
        geometry(names)
        mac.press(TITLE, 'Avatar source: Invalid image')
        geometry(names)
        # Custom native fallback uses the same avatar semantic identity across
        # successful/failed/no primary source; it is not an OCaml sibling swap.
        toggle('Custom avatar fallback')
        geometry(names)
        mac.press(TITLE, 'Avatar source: Custom fallback')
        mac.wait_text(TITLE, 'Avatar image: Ready · Team opens: 0')
        geometry(names)
        mac.press(TITLE, 'Avatar source: Image')
        mac.wait_text(TITLE, 'Avatar image: Failed: Invalid_data · Team opens: 0')
        geometry(names)
        mac.press(TITLE, 'Avatar source: Invalid image')
        geometry(names)
        if images:
            reveal_gallery_control(mac, 'Team avatars', 'AXGroup')
            screenshot(mac, images / 'gallery-avatar-rich-fallback.png', title=TITLE)
        toggle('Custom avatar fallback')
        toggle('Rounded-square avatars')
        geometry(names)
        toggle('Rounded-square avatars')
        toggle('Avatar identity colors')
        geometry(names)
        toggle('Avatar identity colors')
        geometry(names)
        # Show every member before reversing so reorder itself does not unmount any.
        while limit != 5:
            change_limit()
        geometry(names)
        for name in names[3:]:
            retained[name] = member(name)
        toggle('Reverse avatar order')
        geometry(list(reversed(names)))
        toggle('Reverse avatar order')
        geometry(names)
        # Limit zero retires all members; restored keys have fresh native objects.
        change_limit()
        geometry(names)
        while limit != 3:
            change_limit()
            geometry(names, preserve=False)
        for name in names[:3]:
            current = member(name)
            try:
                assert not equal(current, retained[name]), ('limited-out avatar was not remounted', name)
            finally:
                mac.release(current)
        toggle('Actionable avatar overflow')
        focus_gallery_control(mac, 'Show 2 more teammates', 'AXButton')
        mac.key(49)
        mac.wait_text(TITLE, 'Avatar image: No image source · Team opens: 1')
        toggle('Show avatar overflow')
        absent = mac.find(TITLE, 'Show 2 more teammates', 'AXButton')
        if absent:
            mac.release(absent)
            raise AssertionError('Removed overflow action remained accessible')
        toggle('Show avatar overflow')
        toggle('Actionable avatar overflow')
        if images:
            reveal_gallery_control(mac, 'Team avatars', 'AXGroup')
            screenshot(mac, images / 'gallery-avatar-group.png', title=TITLE)
        before_page_exit = {name: member(name) for name in names[:limit]}
        try:
            mac.press(TITLE, 'Selection & actions')
            wait_absent(mac, 'Team avatars', 'AXGroup')
            mac.press(TITLE, 'Presentation')
            # Bonsai retains this branch's model; the native subtree is retired.
            mac.wait_text(TITLE, 'Avatar image: No image source · Team opens: 1')
            geometry(names, preserve=False)
            for name, previous in before_page_exit.items():
                current = member(name)
                try:
                    assert not equal(current, previous), ('page avatar was not remounted', name)
                finally:
                    mac.release(current)
        finally:
            for node in before_page_exit.values():
                mac.release(node)
    finally:
        for node in retained.values():
            mac.release(node)
    print(f'GALLERY_AVATAR_GROUP_OK geometry_cases={cases} source_transitions=6 rich_fallback_identity=pass native_overflow_action=pass retirement=pass', flush=True)
