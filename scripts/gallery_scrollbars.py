"""Native scrollbar input and presentation updates on an ordinary viewport."""
import ctypes as C
import json
import time


def exercise(mac, images=None, *, managed=False):
    from mac_input_source import foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_focus, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent,
    )

    report = {'complete': False, 'managed': managed, 'cases': []}
    mouse = GalleryMouse(mac)
    original_post = mac.post_key
    prefix = 'Collection preview — '
    get = mac.cf.CFNumberGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]

    def press(name):
        if name in ('Custom scrollbars', 'Gradient thumbs', 'Animate scrollbars'):
            activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))
        else:
            mac.press(TITLE, name)
        # AX actions enqueue native/Bonsai updates; reveal must use the resulting
        # layout rather than the old toolbar/viewport coordinates.
        time.sleep(.2)

    def number(node, name):
        raw = mac.attr(node, name)
        value = C.c_double()
        try:
            assert raw and get(raw, 13, C.byref(value)), name
            return value.value
        finally:
            if raw:
                mac.release(raw)

    def state(axis):
        node = mac.wait_find(TITLE, prefix+axis, 'AXScrollBar', search_files=managed)
        try:
            return {'value': number(node, 'AXValue'),
                    'maximum': number(node, 'AXMaxValue'),
                    'minimum': number(node, 'AXMinValue'),
                    'bounds': list(element_rect(mac, node))}
        finally:
            mac.release(node)

    def wait_value(axis, predicate):
        deadline = time.monotonic()+5
        while True:
            result = state(axis)
            if predicate(result):
                return result
            assert time.monotonic() < deadline, (axis, result)
            time.sleep(.025)

    def same(axis, value):
        return wait_value(axis, lambda s: abs(s['value']-value) <= .05)

    def focus(axis):
        reveal_gallery_control(mac, prefix+axis, 'AXScrollBar', scroll_fraction=.24, search_files=managed)
        focus_gallery_control(mac, prefix+axis, 'AXScrollBar', search_files=managed)
        expect_focus(mac, prefix+axis, 'AXScrollBar', search_files=managed)

    def reveal_viewport():
        return reveal_gallery_control(
            mac, 'Changing the appearance keeps your position. Axis settings choose the bars; '
            'the content can still scroll in both directions.',
            'AXStaticText', scroll_fraction=.24)

    def home(axis):
        focus(axis)
        mac.key(115)
        return same(axis, 0)

    def action(axis, name):
        node = mac.wait_find(TITLE, prefix+axis, 'AXScrollBar', search_files=managed)
        try:
            mac.perform(node, name)
        finally:
            mac.release(node)

    def click(point):
        mouse.check_owner(point)
        mouse.send(5, point)
        mouse.send(1, point)
        mouse.send(2, point)

    try:
        foreground_keys(mac)
        if managed:
            owners = (
                ('Message list', 'Preview conversation', None, 'First entry'),
                ('Outline tree', 'Preview outline', 'AXOutline', 'Reveal observatory'),
                ('Result table', 'Preview results', 'AXTable', 'First result'),
            )
            for theme in ('Light', 'Dark'):
                select_gallery_appearance(mac, theme)
                for scale, next_scale in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                    press('Lists, trees & tables')
                    press('Always visible')
                    for mode, label, role, reset in owners:
                        case = {'theme': theme, 'scale': scale, 'owner': mode}
                        report['cases'].append(case)
                        press(mode)
                        press(reset)
                        reveal_gallery_control(mac, label, role, scroll_in_left_gutter=True)
                        if mode == 'Outline tree':
                            # Reveal opens Research/Sketches; expanding Archive
                            # produces seven 36px rows in the 235px viewport.
                            focus_gallery_control(mac, 'Archive', 'AXRow', search_files=True)
                            mac.key(124)
                            time.sleep(.15)
                        home('vertical')
                        case['start'] = state('vertical')
                        assert case['start']['maximum'] > 0, case
                        mac.key(125)
                        case['step'] = wait_value('vertical', lambda s: s['value'] > 0)
                        mac.key(126)
                        same('vertical', 0)
                        action('vertical', 'AXIncrement')
                        wait_value('vertical', lambda s: s['value'] > 0)
                        action('vertical', 'AXDecrement')
                        same('vertical', 0)
                        focus('vertical')
                        mac.key(119)
                        case['end'] = wait_value('vertical', lambda s: abs(s['value']-s['maximum']) < .05)
                        assert case['end']['value'] > 0, case
                        if mode == 'Message list':
                            mac.wait_text(TITLE, 'Entry 0999')
                        elif mode == 'Outline tree':
                            mac.release(mac.wait_find(TITLE, 'Release checklist', 'AXRow', search_files=True))
                        else:
                            mac.release(mac.wait_find(TITLE, '0999', 'AXCell', search_files=True))
                        home('vertical')
                        mac.key(125)
                        offset = wait_value('vertical', lambda s: s['value'] > 0)['value']
                        for control in ('Gradient thumbs', 'Animate scrollbars'):
                            press(control)
                            reveal_gallery_control(mac, label, role, scroll_in_left_gutter=True)
                            same('vertical', offset)
                            press(control)
                            reveal_gallery_control(mac, label, role, scroll_in_left_gutter=True)
                            same('vertical', offset)
                        press('Custom scrollbars')
                        reveal_gallery_control(mac, label, role, scroll_in_left_gutter=True)
                        wait_absent(mac, prefix+'vertical', 'AXScrollBar', search_files=managed)
                        press('Custom scrollbars')
                        reveal_gallery_control(mac, label, role, scroll_in_left_gutter=True)
                        same('vertical', offset)
                        if images:
                            screenshot(mac, images/f'managed-{mode}-{theme}-{scale}.png', title=TITLE)
                        case['retained_offset'] = offset
                        case['complete'] = True
                        print('GALLERY_MANAGED_SCROLLBARS_CASE', mode, theme, scale, 'PASS', flush=True)
                    press('Presentation')
                    wait_absent(mac, prefix+'vertical', 'AXScrollBar', search_files=managed)
                    press(scale)
                    mac.release(mac.wait_find(TITLE, next_scale, 'AXButton'))
            report['complete'] = True
            print('GALLERY_MANAGED_SCROLLBARS_OK 18 cases', flush=True)
            return
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for scale, next_scale in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                press('Lists, trees & tables')
                press('Scrollbars')
                press('Always visible')
                case = {'theme': theme, 'scale': scale}
                report['cases'].append(case)
                # The horizontal range can be clipped completely until the
                # outer page reveals the bottom of this ordinary viewport.
                reveal_viewport()
                home('vertical')
                home('horizontal')
                case['start'] = {a: state(a) for a in ('horizontal', 'vertical')}
                for axis, forward, backward in (('horizontal', 124, 123), ('vertical', 125, 126)):
                    home(axis)
                    mac.key(forward)
                    moved = wait_value(axis, lambda s: s['value'] > 0)
                    mac.key(backward)
                    same(axis, 0)
                    action(axis, 'AXIncrement')
                    wait_value(axis, lambda s: s['value'] > 0)
                    action(axis, 'AXDecrement')
                    same(axis, 0)
                    mac.key(119)
                    end = wait_value(axis, lambda s: abs(s['value']-s['maximum']) < .05)
                    assert end['maximum'] > moved['value'], (axis, end)
                    mac.key(115)
                    same(axis, 0)
                    case[axis+'_keys'] = {'step': moved['value'], 'end': end['value']}
                focus('vertical')
                mac.key(121)  # Page Down, one viewport.
                page = wait_value('vertical', lambda s: s['value'] > 0)
                mac.key(116)
                same('vertical', 0)
                case['page'] = page

                # Track click and captured thumb motion must not steal focus.
                focus_gallery_control(mac, 'Add six rows', 'AXButton')
                x, y, w, h = state('vertical')['bounds']
                click((x+w/2, y+h*.8))
                case['track_click'] = wait_value('vertical', lambda s: s['value'] > 0)
                expect_focus(mac, 'Add six rows', 'AXButton')
                home('vertical')
                focus_gallery_control(mac, 'Add six rows', 'AXButton')
                x, y, w, h = state('vertical')['bounds']
                start = (x+w/2, y+12)
                finish = (start[0], start[1]+60)
                mouse.check_owner(start)
                mouse.check_owner(finish)
                mouse.send(5, start)
                time.sleep(.15)
                mouse.send(1, start)
                try:
                    mouse.send(6, finish)
                    dragged = wait_value('vertical', lambda s: s['value'] > 0)
                    expect_focus(mac, 'Add six rows', 'AXButton')
                    mac.key(53)  # Escape cancels capture, not the offset already applied.
                    time.sleep(.1)
                    mouse.send(6, (finish[0], finish[1]+35))
                    time.sleep(.1)
                    same('vertical', dragged['value'])
                finally:
                    mouse.send(2, finish)
                case['drag_cancel'] = dragged

                # Keep both nonzero offsets while changing presentation metadata.
                focus('horizontal')
                mac.key(124)
                horizontal = wait_value('horizontal', lambda s: s['value'] > 0)['value']
                vertical = state('vertical')['value']
                for label in ('Gradient thumbs', 'Animate scrollbars', 'While scrolling', 'On hover', 'Always visible'):
                    case['presentation_action'] = label
                    press(label)
                    # AX activation may reveal the toolbar at Large size.
                    # Restore the outer viewport before querying clipped bars.
                    case['presentation_caption_bounds'] = reveal_viewport()
                    same('horizontal', horizontal)
                    same('vertical', vertical)
                press('Gradient thumbs')
                press('Animate scrollbars')
                reveal_viewport()
                press('Both axes')
                reveal_viewport()
                wait_absent(mac, prefix+'vertical', 'AXScrollBar', search_files=managed)
                same('horizontal', horizontal)
                press('Horizontal only')
                reveal_viewport()
                wait_absent(mac, prefix+'horizontal', 'AXScrollBar')
                same('vertical', vertical)
                press('Vertical only')
                reveal_viewport()
                same('horizontal', horizontal)
                same('vertical', vertical)
                press('Custom scrollbars')
                reveal_viewport()
                wait_absent(mac, prefix+'horizontal', 'AXScrollBar')
                wait_absent(mac, prefix+'vertical', 'AXScrollBar', search_files=managed)
                press('Custom scrollbars')
                reveal_viewport()
                same('horizontal', horizontal)
                same('vertical', vertical)
                before = state('vertical')
                press('Add six rows')
                reveal_viewport()
                after = wait_value('vertical', lambda s: s['maximum'] > before['maximum'])
                same('vertical', vertical)
                same('horizontal', horizontal)
                case['growth'] = {'before': before, 'after': after}
                if images:
                    screenshot(mac, images/f'scrollbar-{theme}-{scale}.png', title=TITLE)
                press('Reset rows')
                reveal_viewport()
                same('vertical', vertical)
                press('Presentation')
                wait_absent(mac, prefix+'vertical', 'AXScrollBar', search_files=managed)
                wait_absent(mac, prefix+'horizontal', 'AXScrollBar')
                case['complete'] = True
                print('GALLERY_SCROLLBARS_CASE', theme, scale, 'PASS', flush=True)
                press(scale)
                mac.release(mac.wait_find(TITLE, next_scale, 'AXButton'))
        report['complete'] = True
        print('GALLERY_SCROLLBARS_OK 6 cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        if images:
            screenshot(mac, images/'failure.png', title=TITLE)
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images/('managed-scrollbars-report.json' if managed else 'scrollbars-report.json')).write_text(json.dumps(report, indent=2)+'\n')
