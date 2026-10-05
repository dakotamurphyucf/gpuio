"""Measure a public native menu against its live anchor on the macOS desktop."""
import ctypes as C
import json
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_focus,
        focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
        wait_absent,
    )
    from test_canvas import screenshot

    label = 'Preview menu'
    mac.press(TITLE, 'Selection & actions')
    reveal_gallery_control(mac, label, 'AXButton')
    mouse = GalleryMouse(mac)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    original = mac.wait_find(TITLE, label, 'AXButton')
    samples = []

    def rect(role):
        node = mac.wait_find(TITLE, label, role)
        try:
            if role == 'AXButton':
                assert equal(original, node), 'placement replaced menu owner'
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def window_rect():
        node = mac.window(TITLE)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def set_checked(name, expected):
        reveal_gallery_control(mac, name, 'AXCheckBox')
        node = mac.wait_find(TITLE, name, 'AXCheckBox')
        raw = mac.attr(node, 'AXValue')
        try:
            assert raw
            if bool(boolean(raw)) != expected:
                activate(mac, mac.retain(node))
        finally:
            if raw:
                mac.release(raw)
            mac.release(node)

    def scroll(amount):
        wx, wy, ww, wh = window_rect()
        point = (wx + ww * .88, wy + wh * .57)
        mouse.check_owner(point)
        mouse.send(5, point)
        create = mac.cg.CGEventCreateScrollWheelEvent
        create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
        locate = mac.cg.CGEventSetLocation
        locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]
        event = create(None, 0, 1, C.c_int(-round(amount)))
        assert event
        try:
            locate(event, GalleryMouse.Point(*point))
            mouse.post(0, event)
        finally:
            mac.release(event)
        time.sleep(.15)

    def centered_anchor():
        reveal_gallery_control(mac, label, 'AXButton')
        _, wy, _, wh = window_rect()
        for _ in range(4):
            _, y, _, height = rect('AXButton')
            distance = y + height / 2 - (wy + wh * .55)
            if abs(distance) < 10:
                return
            scroll(distance)
        raise AssertionError(('could not center menu anchor', rect('AXButton'), window_rect()))

    def geometry(case, right, trailing):
        deadline, observed = time.monotonic() + 5, None
        while time.monotonic() < deadline:
            ax, ay, aw, ah = anchor = rect('AXButton')
            mx, my, mw, mh = popup = rect('AXMenu')
            if right:
                expected = (ax + aw + 8, ay + ah - mh if trailing else ay)
            else:
                expected = (ax + aw - mw if trailing else ax, ay + ah + 8)
            observed = (anchor, popup, expected)
            # AX panel bounds exclude the one-point border. Allow two logical
            # points for the pair of borders and device-pixel rounding.
            if abs(mx - expected[0]) <= 2 and abs(my - expected[1]) <= 2:
                samples.append({'case': case, 'anchor': anchor, 'popup': popup,
                                'expected_origin': expected})
                print('GALLERY_MENU_PLACEMENT_CASE', case, observed, flush=True)
                return anchor, popup
            time.sleep(.03)
        raise AssertionError((case, 'menu placement', observed))

    try:
        set_checked('Observe menu visibility', True)
        set_checked('Disable preview menu', False)
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for right, trailing in ((False, False), (True, False), (True, True), (False, True)):
                set_checked('Prefer menu on right', right)
                set_checked('Align menu to trailing edge', trailing)
                centered_anchor()
                focus_gallery_control(mac, label, 'AXButton')
                mac.key(125)
                mac.wait_text(TITLE, 'Menu visibility: open')
                case = f'{theme}-{"right" if right else "bottom"}-{"end" if trailing else "start"}'
                before, _ = geometry(case, right, trailing)
                if images:
                    screenshot(mac, images / (case + '.png'), title=TITLE)
                # A wheel outside the popup moves the actual retained anchor.
                # The native menu should reposition without losing open state.
                scroll(45)
                mac.wait_text(TITLE, 'Menu visibility: open')
                after, _ = geometry(case + '-scrolled', right, trailing)
                assert before[1] - after[1] > 10, ('wheel did not move anchor', before, after)
                mac.key(53)
                wait_absent(mac, label, 'AXMenu')
                mac.wait_text(TITLE, 'Menu visibility: closed')
                expect_focus(mac, label)
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, label, 'AXButton')
        wait_absent(mac, label, 'AXMenu')
        print(f'GALLERY_MENU_PLACEMENT_OK: {len(samples)} native geometry samples; '
              'Bottom/Right × Start/End × Light/Dark, live root scroll, '
              'retained anchor, Escape focus return and page retirement', flush=True)
    finally:
        if images:
            (images / 'menu-placement-samples.json').write_text(json.dumps(samples, indent=2) + '\n')
        mac.release(original)
