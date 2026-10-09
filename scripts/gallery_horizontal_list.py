"""Physical variable-size list geometry, local state and OS wheel routing."""
import ctypes as C
import json
import re
import time


def exercise(mac, images=None):
    from mac_input_source import foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, GalleryMouse, element_rect, focus_gallery_control, reveal_gallery_control,
        select_gallery_appearance, wait_absent,
    )

    report = {'complete': False, 'cases': []}
    original_post = mac.post_key
    mouse = GalleryMouse(mac)
    create = mac.cg.CGEventCreateScrollWheelEvent2
    create.restype, create.argtypes = C.c_void_p, [
        C.c_void_p, C.c_uint, C.c_uint, C.c_int, C.c_int, C.c_int]
    locate = mac.cg.CGEventSetLocation
    locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]

    def press(name):
        mac.press(TITLE, name)

    def snapshot():
        root = mac.wait_find(TITLE, 'Research cards', 'AXList')
        children = mac.children(root)
        rows = []
        try:
            bounds = list(element_rect(mac, root))
            for child in children:
                nodes = mac.children(child)
                try:
                    label = next((mac.text(n, 'AXTitle') for n in nodes
                                  if mac.text(n, 'AXTitle').startswith('FIELD NOTE / ')), None)
                    if not label:
                        continue
                    button = next((n for n in nodes if mac.text(n, 'AXRole') == 'AXButton'), None)
                    assert button, ('missing row reaction', label)
                    rows.append({'key': int(label.split('/')[-1]),
                                 'bounds': list(element_rect(mac, child)),
                                 'button': list(element_rect(mac, button)),
                                 'reaction': mac.text(button, 'AXTitle')})
                finally:
                    for node in nodes:
                        mac.release(node)
        finally:
            for child in children:
                mac.release(child)
            mac.release(root)
        return {'bounds': bounds, 'rows': rows}

    def first():
        press('First card')
        reveal_gallery_control(mac, 'Research cards', 'AXList', scroll_in_left_gutter=True)
        mac.wait_text(TITLE, 'FIELD NOTE / 00000')
        time.sleep(.15)
        value = snapshot()
        assert value['rows'][0]['key'] == 0, value
        return value

    def contained(outer, inner):
        x, y, w, h = outer
        a, b, c, d = inner
        return x-1 <= a and y-1 <= b and a+c <= x+w+1 and b+d <= y+h+1

    def check(value, axis):
        rows = value['rows']
        assert 0 < len(rows) <= 16, ('visible row budget', len(rows))
        for row in rows:
            assert contained(row['bounds'], row['button']), ('clipped reaction', row)
        offset, extent = (0, 2) if axis == 'horizontal' else (1, 3)
        for a, b in zip(rows, rows[1:]):
            end = a['bounds'][offset] + a['bounds'][extent]
            assert abs(end-b['bounds'][offset]) <= 1, ('row overlap/gap', axis, a, b)

    def wheel(axis, value):
        x, y, w, h = value['bounds']
        point = (x+w-10, y+min(h/2, 100))
        mouse.check_owner(point)
        mouse.send(5, point)
        event = create(None, 1, 2, -2 if axis == 'vertical' else 0,
                       -2 if axis == 'horizontal' else 0, 0)
        assert event
        try:
            locate(event, GalleryMouse.Point(*point))
            mouse.post(0, event)
        finally:
            mac.release(event)
        time.sleep(.15)
        return snapshot()

    try:
        foreground_keys(mac)
        press('Presentation')
        for current, following in (('Large', 'Compact'), ('Compact', 'Comfortable')):
            node = mac.find(TITLE, current, 'AXButton')
            if node:
                mac.release(node)
                press(current)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for scale, following in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                press('Lists, trees & tables')
                press('Horizontal cards')
                case = {'theme': theme, 'scale': scale, 'axes': []}
                report['cases'].append(case)
                for axis in ('horizontal', 'vertical'):
                    before = first()
                    check(before, axis)
                    row = before['rows'][0]
                    assert contained(before['bounds'], row['button']), ('first action outside viewport', before)
                    x, y, w, h = row['button']
                    count = int(row['reaction'].split('·')[-1])
                    point = (x+w/2, y+h/2)
                    if axis == 'horizontal':
                        mouse.check_owner(point)
                        mouse.send(1, point)
                        mouse.send(2, point)
                    else:
                        focus_gallery_control(mac, row['reaction'], 'AXButton')
                        mac.key(49)
                    mac.wait_text(TITLE, f'Local reactions · {count+1}')
                    # The row remains mounted across growth and an axis update.
                    if axis == 'horizontal':
                        press('Grow visible card')
                        time.sleep(.15)
                    grown = snapshot()
                    check(grown, axis)
                    assert grown['rows'][0]['key'] == 0
                    assert grown['rows'][0]['reaction'] == f'Local reactions · {count+1}'
                    dimension = 2 if axis == 'horizontal' else 3
                    if axis == 'horizontal':
                        assert grown['rows'][0]['bounds'][dimension] > row['bounds'][dimension]+20
                    samples = [grown]
                    for _ in range(8):
                        samples.append(wheel(axis, samples[-1]))
                        check(samples[-1], axis)
                    initial = {r['key']: r for r in samples[0]['rows']}
                    after = {r['key']: r for r in samples[1]['rows']}
                    common = set(initial) & set(after)
                    coordinate = 0 if axis == 'horizontal' else 1
                    assert common and all(after[k]['bounds'][coordinate] < initial[k]['bounds'][coordinate]-1 for k in common), 'wheel did not move rows'
                    case['axes'].append({'axis': axis, 'before': before, 'grown': grown, 'wheel': samples[1:]})
                    first()
                    # Rows evicted by scrolling may have reset their transient model.
                    retained = snapshot()['rows'][0]['reaction']
                    press('Use vertical axis' if axis == 'horizontal' else 'Use horizontal axis')
                    time.sleep(.2)
                    changed = snapshot()
                    assert changed['rows'][0]['key'] == 0 and changed['rows'][0]['reaction'] == retained, ('axis lost surviving row state', changed)
                    case['axes'][-1]['after_axis_change'] = changed
                if images:
                    screenshot(mac, images / f'cards-{theme}-{scale}.png', title=TITLE)
                # Observe actual Bonsai active-row count independently of exposed AX rows.
                node = mac.wait_find(TITLE, ' / 16 mounted', 'AXStaticText', contains=True)
                try:
                    caption = mac.text(node, 'AXTitle')
                finally:
                    mac.release(node)
                match = re.search(r'10000 cards · (\d+) / 16 mounted', caption)
                assert match and 0 < int(match[1]) <= 16, caption
                case['caption'] = caption
                case['complete'] = True
                print('GALLERY_HORIZONTAL_CASE', theme, scale, 'PASS', flush=True)
                press('Presentation')
                press(scale)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        press('Lists, trees & tables')
        press('Horizontal cards')
        press('Middle')
        reveal_gallery_control(mac, 'Research cards', 'AXList', scroll_in_left_gutter=True)
        mac.wait_text(TITLE, 'FIELD NOTE / 05000')
        time.sleep(.2)
        anchored = snapshot()
        anchor = anchored['rows'][0]
        navigation = {'before': anchored, 'updates': []}
        report['navigation'] = navigation
        for action in ('Reverse order', 'Prepend', 'Append'):
            press(action)
            time.sleep(.2)
            changed = snapshot()
            check(changed, 'horizontal')
            retained = next((r for r in changed['rows'] if r['key'] == anchor['key']), None)
            assert retained and abs(retained['bounds'][0]-anchor['bounds'][0]) <= 1, ('anchor moved on edit', action, anchored, changed)
            navigation['updates'].append({'action': action, 'snapshot': changed})
        press('Follow latest')
        mac.wait_text(TITLE, 'FIELD NOTE / 10000')
        press('Append')
        mac.wait_text(TITLE, 'FIELD NOTE / 10001')
        navigation['tail'] = snapshot()
        check(navigation['tail'], 'horizontal')
        press('Presentation')
        wait_absent(mac, 'Research cards', 'AXList')
        navigation['complete'] = True
        report['complete'] = True
        print('GALLERY_HORIZONTAL_LIST_OK 6 cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'horizontal-report.json').write_text(json.dumps(report, indent=2)+'\n')
