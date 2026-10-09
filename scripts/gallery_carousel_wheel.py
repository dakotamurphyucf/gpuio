"""Actual macOS line-wheel routing through the public measured carousel."""
import ctypes as C
import json
import time


def exercise(mac, images=None):
    from mac_input_source import foreground_keys
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance,
    )

    label = 'Measured idea cards'
    report = {'complete': False, 'cases': [],
              'scope': 'CG line-wheel delivery through AppKit; not hardware trackpad or frame timing'}
    mouse = GalleryMouse(mac)
    original_post = mac.post_key
    create = mac.cg.CGEventCreateScrollWheelEvent2
    create.restype, create.argtypes = C.c_void_p, [
        C.c_void_p, C.c_uint, C.c_uint, C.c_int, C.c_int, C.c_int]
    integer = mac.cg.CGEventGetIntegerValueField
    integer.restype, integer.argtypes = C.c_longlong, [C.c_void_p, C.c_int]
    locate = mac.cg.CGEventSetLocation
    locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]

    def rect():
        node = mac.wait_find(TITLE, label, 'AXGroup')
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def selected(name):
        mac.wait_text(TITLE, 'Selected card: ' + name)

    def toggle(name):
        activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))

    def reset(key=115, name='Capture'):
        reveal_gallery_control(mac, label, 'AXGroup', scroll_in_left_gutter=True)
        focus_gallery_control(mac, label, 'AXGroup')
        mac.key(key)
        selected(name)
        time.sleep(.4)

    def wheel(dx, dy):
        before = rect()
        x, y, w, h = before
        point = (x + w - 8, y + h - 8)  # noninteractive card padding
        mouse.check_owner(point)
        mouse.send(5, point)
        event = create(None, 1, 2, dy, dx, 0)  # kCGScrollEventUnitLine
        assert event, 'Cannot create line-wheel event'
        try:
            assert integer(event, 11) == dy and integer(event, 12) == dx
            assert integer(event, 88) == 0, 'Expected a discrete, not precise, wheel'
            locate(event, GalleryMouse.Point(*point))
            mouse.post(0, event)
        finally:
            mac.release(event)
        time.sleep(.2)  # Separate bursts; quiet coalescing is tested natively.
        return before, rect()

    def stable(before, after):
        assert all(abs(a-b) < 1 for a, b in zip(before, after)), (
            'Consumed carousel wheel moved the enclosing page', before, after)

    try:
        foreground_keys(mac)
        mac.press(TITLE, 'Carousels & journeys')
        toggle('Compact viewport')
        # Immediate movement isolates wheel routing from animation sampling.
        toggle('Animate card movement')
        for theme in ('Dark', 'Light'):
            select_gallery_appearance(mac, theme)
            for axis in ('horizontal', 'vertical'):
                if axis == 'vertical':
                    mac.press(TITLE, 'Use vertical track')
                reset()
                case = {'axis': axis, 'theme': theme, 'steps': []}
                report['cases'].append(case)
                for amount, expected in [(-3, 'Explore'), (-3, 'Refine'),
                                         (-3, 'Publish'), (3, 'Refine')]:
                    before, after = wheel(amount if axis == 'horizontal' else 0,
                                          amount if axis == 'vertical' else 0)
                    selected(expected)
                    stable(before, after)
                    case['steps'].append(dict(delta=amount, selected=expected,
                                              before=before, after=after))
                # Beginning-of-track vertical wheels belong to the outer page;
                # horizontal wheels at an endpoint remain pinned to the track.
                reset()
                before, after = wheel(3 if axis == 'horizontal' else 0,
                                      3 if axis == 'vertical' else 0)
                selected('Capture')
                if axis == 'horizontal':
                    stable(before, after)
                else:
                    assert after[1] > before[1] + 1, (
                        'Vertical start edge failed to hand upward wheel to page', before, after)
                case['start_edge'] = dict(before=before, after=after)
                reset(119, 'Publish')  # End
                before, after = wheel(-3 if axis == 'horizontal' else 0,
                                      -3 if axis == 'vertical' else 0)
                selected('Publish')
                if axis == 'horizontal':
                    stable(before, after)
                else:
                    assert after[1] < before[1] - 1, (
                        'Vertical end edge failed to hand downward wheel to page', before, after)
                case['end_edge'] = dict(before=before, after=after)
                if axis == 'horizontal':
                    reset()
                    before, after = wheel(0, 3)
                    selected('Capture')
                    assert after[1] > before[1] + 1, (
                        'Cross-axis vertical wheel did not reach page', before, after)
                    case['cross_axis'] = dict(before=before, after=after)
                else:
                    mac.press(TITLE, 'Use horizontal track')
                print('CAROUSEL_WHEEL_CASE', case, flush=True)
        report['complete'] = True
        print('GALLERY_CAROUSEL_WHEEL_OK', len(report['cases']), 'axis/theme cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'carousel-wheel-report.json').write_text(json.dumps(report, indent=2) + '\n')
