"""Native message-follow composition: input, anchor retention and bounded geometry."""
import ctypes as C
import json
import re
import time


def exercise(mac, images=None):
    from mac_input_source import foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, GalleryMouse, element_rect, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent,
    )

    label = 'Preview conversation'
    report = {'complete': False, 'cases': []}
    mouse = GalleryMouse(mac)
    original_post = mac.post_key
    create = mac.cg.CGEventCreateScrollWheelEvent2
    create.restype, create.argtypes = C.c_void_p, [
        C.c_void_p, C.c_uint, C.c_uint, C.c_int, C.c_int, C.c_int]
    locate = mac.cg.CGEventSetLocation
    locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]

    def press(name):
        mac.press(TITLE, name)

    def rect(name, role=None):
        node = mac.wait_find(TITLE, name, role)
        try:
            return list(element_rect(mac, node))
        finally:
            mac.release(node)

    def snapshot():
        root = mac.wait_find(TITLE, label, None)
        rows = []
        def visit(node):
            values, children = mac.node_values(node)
            try:
                if values[0] == 'AXStaticText' and values[1].startswith('Entry '):
                    rows.append({'text': values[1], 'bounds': list(element_rect(mac, node))})
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        try:
            bounds = list(element_rect(mac, root))
            visit(root)
        finally:
            mac.release(root)
        assert rows, 'no exposed transcript rows'
        return {'bounds': bounds, 'rows': rows}

    def settled():
        time.sleep(.3)
        return snapshot()

    def stable(before, after):
        assert all(abs(a-b) <= 1 for a, b in zip(before['bounds'], after['bounds'])), ('overlay changed list geometry', before, after)
        anchor = before['rows'][0]
        found = next((r for r in after['rows'] if r['text'] == anchor['text']), None)
        assert found and all(abs(a-b) <= 1 for a, b in zip(anchor['bounds'], found['bounds'])), ('reading anchor moved', anchor, after)

    def following():
        mac.wait_text(TITLE, 'Following new messages.')
        wait_absent(mac, 'Follow latest', 'AXButton')

    def history():
        mac.wait_text(TITLE, 'Reading history.')
        mac.release(mac.wait_find(TITLE, 'Follow latest', 'AXButton'))

    def upward_wheel(bounds):
        x, y, w, h = bounds
        # This is inside the former bottom-center jump slot after it becomes inert.
        point = (x+w/2, y+h-30)
        mouse.check_owner(point)
        mouse.send(5, point)
        event = create(None, 1, 2, 3, 0, 0)
        assert event
        try:
            locate(event, GalleryMouse.Point(*point))
            mouse.post(0, event)
        finally:
            mac.release(event)

    try:
        foreground_keys(mac)
        press('Presentation')
        for current, following_scale in (('Large', 'Compact'), ('Compact', 'Comfortable')):
            node = mac.find(TITLE, current, 'AXButton')
            if node:
                mac.release(node)
                press(current)
                mac.release(mac.wait_find(TITLE, following_scale, 'AXButton'))
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for scale, following_scale in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                policy = 'Reduced' if scale == 'Compact' else 'Full'
                press('Motion & rhythm')
                press('Use reduced motion' if policy == 'Reduced' else 'Use full motion')
                mac.wait_text(TITLE, 'Motion preference: '+policy)
                press('Lists, trees & tables')
                press('Message list')
                press('Reset latest response')
                press('First entry')
                reveal_gallery_control(mac, label, None, scroll_in_left_gutter=True)
                history()
                before = settled()
                button = rect('Follow latest', 'AXButton')
                case = {'theme': theme, 'scale': scale, 'motion': policy, 'before': before, 'button': button}
                report['cases'].append(case)
                if images:
                    screenshot(mac, images/f'follow-{theme}-{scale}.png', title=TITLE)
                x, y, w, h = before['bounds']
                a, b, c, d = button
                assert d <= 48.5, ('jump button exceeds documented 48px slot', button)
                assert x <= a and a+c <= x+w and y <= b and b+d <= y+h, ('jump outside viewport', before, button)
                press('Show follow button')
                wait_absent(mac, 'Follow latest', 'AXButton')
                stable(before, settled())
                press('Fade transcript edge')
                stable(before, settled())
                press('Animate follow controls')
                stable(before, settled())
                press('Show follow button')
                history()
                stable(before, settled())
                press('Fade transcript edge')
                press('Animate follow controls')
                focus_gallery_control(mac, 'Follow latest', 'AXButton')
                mac.key(49)
                following()
                press('Grow latest response')
                mac.wait_text(TITLE, 'Another useful detail.')
                following()
                tail = settled()
                upward_wheel(tail['bounds'])
                history()
                case['wheel_before'] = tail
                case['wheel_after'] = settled()
                assert case['wheel_before']['rows'] != case['wheel_after']['rows'], 'wheel did not move transcript'
                press('First entry')
                history()
                anchored = settled()
                for action in ('Earlier history', 'New message', 'Grow latest response'):
                    press(action)
                    history()
                    stable(anchored, settled())
                case['after_edits'] = snapshot()
                a, b, c, d = rect('Follow latest', 'AXButton')
                point = (a+c/2, b+d/2)
                mouse.check_owner(point)
                mouse.send(1, point)
                mouse.send(2, point)
                following()
                node = mac.wait_find(TITLE, 'Mounted list rows:', 'AXStaticText', contains=True)
                try:
                    caption = mac.text(node, 'AXTitle')
                finally:
                    mac.release(node)
                match = re.fullmatch(r'Mounted list rows: (\d+) / 24', caption)
                assert match and 0 < int(match[1]) <= 24, caption
                case['mounted'] = caption
                press('Presentation')
                wait_absent(mac, label, None)
                wait_absent(mac, 'Follow latest', 'AXButton')
                case['complete'] = True
                print('GALLERY_MESSAGE_FOLLOW_CASE', theme, scale, 'PASS', flush=True)
                press(scale)
                mac.release(mac.wait_find(TITLE, following_scale, 'AXButton'))
        report['complete'] = True
        print('GALLERY_MESSAGE_FOLLOW_OK 6 cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images/'message-follow-report.json').write_text(json.dumps(report, indent=2)+'\n')
