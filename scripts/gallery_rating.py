"""Public Rating geometry, policies and real macOS input. Execution is native-only."""
import ctypes as C
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot

    mac.press(TITLE, 'Numeric inputs')
    name = 'Preview rating'
    reveal_gallery_control(mac, name, 'AXSlider')
    original = mac.wait_find(TITLE, name, 'AXSlider')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    get_bool = mac.cf.CFBooleanGetValue
    get_bool.restype, get_bool.argtypes = C.c_bool, [C.c_void_p]
    mouse = GalleryMouse(mac)
    maximum, size, cases = 5, 24, 0

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.08)

    def geometry():
        nonlocal cases
        reveal_gallery_control(mac, name, 'AXSlider')
        node = mac.wait_find(TITLE, name, 'AXSlider')
        try:
            assert equal(original, node), 'Appearance/configuration replaced the rating owner'
            bounds = element_rect(mac, node)
            assert bounds[2] >= maximum * size, ('star row clipped', bounds, maximum, size)
            assert abs(bounds[3] - (size + 2)) < 1.1, ('logical star size', bounds, size)
            cases += 1
            return bounds
        finally:
            mac.release(node)

    def click_star(index):
        bounds = geometry()
        point = (bounds[0] + 1 + (index - .5) * size, bounds[1] + bounds[3] / 2)
        mouse.check_owner(point)
        mouse.send(5, point)
        mouse.send(1, point)
        mouse.send(2, point)

    try:
        for theme in ('Light', 'Dark'):
            mac.press(TITLE, theme)
            for _ in range(3):
                for _ in range(3):
                    geometry()
                    toggle('Separate star colors')
                    geometry()
                    toggle('Separate star colors')
                    mac.press(TITLE, f'Maximum: {maximum} stars')
                    maximum = {5: 10, 10: 1, 1: 5}[maximum]
                    mac.release(mac.wait_find(TITLE, f'Maximum: {maximum} stars', 'AXButton'))
                mac.press(TITLE, f'Star size: {size} px')
                size = {24: 36, 36: 16, 16: 24}[size]
                mac.release(mac.wait_find(TITLE, f'Star size: {size} px', 'AXButton'))
        focus_gallery_control(mac, name, 'AXSlider')
        mac.key(115)  # Home.
        mac.wait_text(TITLE, 'Rating: 0 of 5')
        for _ in range(7):
            mac.key(124)  # Ordered Right requests saturate in the current reducer.
        mac.wait_text(TITLE, 'Rating: 5 of 5')
        click_star(4)
        mac.wait_text(TITLE, 'Rating: 4 of 5')
        click_star(4)
        mac.wait_text(TITLE, 'Rating: 0 of 5')
        toggle('Step down filled stars on click')
        click_star(4)
        mac.wait_text(TITLE, 'Rating: 4 of 5')
        click_star(2)
        mac.wait_text(TITLE, 'Rating: 1 of 5')
        toggle('Read-only numeric inputs')
        focus_gallery_control(mac, name, 'AXSlider')
        mac.key(124)
        time.sleep(.1)
        mac.wait_text(TITLE, 'Rating: 1 of 5')
        toggle('Read-only numeric inputs')
        toggle('Disable rating')
        node = mac.wait_find(TITLE, name, 'AXSlider')
        try:
            enabled = mac.attr(node, 'AXEnabled')
            try:
                assert enabled and not get_bool(enabled), 'Disabled rating remains enabled in AX'
            finally:
                if enabled:
                    mac.release(enabled)
        finally:
            mac.release(node)
        toggle('Disable rating')
        toggle('Separate star colors')
        geometry()
        if images:
            screenshot(mac, images / 'gallery-rating-colors.png', title=TITLE)
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, name, 'AXSlider')
    finally:
        mac.release(original)
    print(f'GALLERY_RATING_OK: {cases} geometry/identity cases, themes, size/maximum/colors, '
          'real keyboard/pointer reducers, read-only/disabled semantics and page retirement', flush=True)
