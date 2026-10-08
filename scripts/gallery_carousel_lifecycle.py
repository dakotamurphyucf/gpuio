"""Cancel real carousel drag previews across window, geometry and page changes."""
import ctypes as C
import json
import time


def exercise(mac, images=None):
    from mac_input_source import foreground_keys, Sources
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_field,
        focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
        wait_absent,
    )

    report = {'complete': False, 'cases': [],
              'scope': 'native preview cancellation and page-scoped editor lifetime; not GPU/resource timing'}
    label = 'Measured idea cards'
    mouse = GalleryMouse(mac)
    original_post = mac.post_key
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    sources = Sources(mac)
    try:
        source = sources.selected()
    finally:
        sources.close()
    assert source in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC'), source
    report['input_source'] = source

    def rect(name=label, role='AXGroup'):
        node = mac.wait_find(TITLE, name, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def compact(expected):
        node = mac.wait_find(TITLE, 'Compact viewport', 'AXCheckBox')
        value = mac.attr(node, 'AXValue')
        try:
            assert value
            actual = bool(boolean(value))
        finally:
            if value:
                mac.release(value)
            mac.release(node)
        if actual != expected:
            activate(mac, mac.wait_find(TITLE, 'Compact viewport', 'AXCheckBox'))

    def draft():
        focus_gallery_control(mac, 'Card draft', 'AXTextField')
        mac.key(0, flags=1 << 20)
        mac.key(0)
        expect_field(mac, TITLE, 'Card draft', 'a')

    def titles():
        windows = mac.children(mac.app, 'AXWindows')
        try:
            return {mac.text(window, 'AXTitle') for window in windows}
        finally:
            for window in windows:
                mac.release(window)

    def selected():
        mac.wait_text(TITLE, 'Selected card: Capture')

    def interrupt(axis, theme, action):
        compact(True)
        reveal_gallery_control(mac, label, 'AXGroup', scroll_in_left_gutter=True)
        focus_gallery_control(mac, label, 'AXGroup')
        mac.key(115)
        selected()
        time.sleep(.4)
        x, y, w, h = rect()
        start = (x+w-8, y+h-8)
        finish = (start[0]-200, start[1]) if axis == 'horizontal' else (start[0], start[1]-200)
        mouse.check_owner(start)
        mouse.check_owner(finish)
        case = dict(axis=axis, theme=theme, action=action, viewport=(x, y, w, h))
        report['cases'].append(case)
        second = None
        try:
            mouse.send(5, start)
            mouse.send(1, start)
            time.sleep(.05)
            for step in range(1, 13):
                mouse.send(6, tuple(a+(b-a)*step/12 for a,b in zip(start, finish)))
                time.sleep(.02)
            selected()
            bx, by, bw, bh = rect('Select Explore', 'AXButton')
            assert x < bx+bw/2 < x+w and y < by+bh/2 < y+h, 'No candidate preview before interruption'
            case['preview_button'] = (bx, by, bw, bh)
            if action == 'inactive-window':
                previous = titles()
                mac.press(TITLE, 'New window')
                deadline = time.monotonic()+10
                while time.monotonic() < deadline:
                    created = titles()-previous
                    if created:
                        assert len(created) == 1
                        second = created.pop()
                        focused = mac.attr(mac.app, 'AXFocusedWindow')
                        try:
                            if focused and mac.text(focused, 'AXTitle') == second:
                                case['focused_window'] = second
                                break
                        finally:
                            if focused:
                                mac.release(focused)
                    time.sleep(.04)
                assert 'focused_window' in case, 'Second window did not become key'
            elif action == 'resize':
                compact(False)
                deadline = time.monotonic()+5
                while time.monotonic() < deadline:
                    after = rect()
                    if abs(after[2]-w) > 1 or abs(after[3]-h) > 1:
                        break
                    time.sleep(.04)
                assert abs(after[2]-w) > 1 or abs(after[3]-h) > 1, 'Viewport did not resize'
                case['resized_viewport'] = after
            elif action == 'page-unmount':
                mac.press(TITLE, 'Presentation')
                wait_absent(mac, label, 'AXGroup')
                wait_absent(mac, 'Card draft', 'AXTextField')
            else:
                raise AssertionError(action)
            time.sleep(.15)
        finally:
            mouse.send(2, finish)
            if second:
                mac.close(second)
        if action == 'page-unmount':
            mac.press(TITLE, 'Carousels & journeys')
            # Returning recreates the page scroller. Clipped track cards are
            # intentionally absent from AX until the outer page reveals them.
            case['remount_before_reveal'] = rect()
            case['remount_after_reveal'] = reveal_gallery_control(
                mac, label, 'AXGroup', scroll_in_left_gutter=True)
            expect_field(mac, TITLE, 'Card draft', 'Make something worth sharing.')
            case['draft'] = 'fresh page-scoped editor'
        else:
            case['draft'] = 'retained'
        compact(True)
        reveal_gallery_control(mac, label, 'AXGroup', scroll_in_left_gutter=True)
        focus_gallery_control(mac, label, 'AXGroup')
        # No Home/reset here: a late commit must remain observable.
        for _ in range(8):
            selected()
            time.sleep(.05)
        if action == 'page-unmount':
            draft()
        else:
            expect_field(mac, TITLE, 'Card draft', 'a')
        case['complete'] = True
        print('CAROUSEL_LIFECYCLE_CASE', case, flush=True)

    try:
        foreground_keys(mac)
        mac.press(TITLE, 'Carousels & journeys')
        compact(True)
        reveal_gallery_control(mac, label, 'AXGroup', scroll_in_left_gutter=True)
        draft()
        for theme in ('Dark', 'Light'):
            select_gallery_appearance(mac, theme)
            for axis in ('horizontal', 'vertical'):
                if axis == 'vertical':
                    mac.press(TITLE, 'Use vertical track')
                for action in ('inactive-window', 'resize', 'page-unmount'):
                    interrupt(axis, theme, action)
                if axis == 'vertical':
                    mac.press(TITLE, 'Use horizontal track')
        report['complete'] = True
        print('GALLERY_CAROUSEL_LIFECYCLE_OK', len(report['cases']), 'interrupted previews', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'carousel-lifecycle-report.json').write_text(json.dumps(report, indent=2)+'\n')
