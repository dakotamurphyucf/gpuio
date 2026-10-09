"""Foreground pointer ownership and cancellation in the public carousel."""
import json
import time


def exercise(mac, images=None):
    from mac_input_source import foreground_keys
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance,
    )

    report = {'complete': False, 'cases': [],
              'scope': 'native pointer routing/selection; not hardware trackpad or frame timing'}
    mouse = GalleryMouse(mac)
    original_post = mac.post_key
    label = 'Measured idea cards'

    def selected(name):
        mac.wait_text(TITLE, 'Selected card: ' + name)

    def rect(name, role):
        node = mac.wait_find(TITLE, name, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def toggle(name):
        activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))

    def reset():
        reveal_gallery_control(mac, label, 'AXGroup')
        focus_gallery_control(mac, label, 'AXGroup')
        mac.key(115)  # Home
        selected('Capture')
        # No presentation-latency claim: let the existing animation settle.
        time.sleep(.4)

    def drag(axis, theme, *, cancel=False, cross=False, tiny=False, disabled=False):
        if disabled:
            # Disabled viewports intentionally reject focus. The preceding
            # below-threshold case leaves Capture selected before disabling.
            reveal_gallery_control(mac, label, 'AXGroup')
            selected('Capture')
        else:
            reset()
        x, y, w, h = rect(label, 'AXGroup')
        # Card padding contains no interactive descendant. All points stay in
        # the viewport so this does not accidentally resize/drag the OS window.
        start = (x + w - 8, y + h - 8)
        amount = 5 if tiny else 200
        horizontal = (axis == 'horizontal') != cross
        finish = (start[0] - amount, start[1]) if horizontal else (start[0], start[1] - amount)
        mouse.check_owner(start)
        mouse.check_owner(finish)
        case = dict(axis=axis, theme=theme, cancel=cancel, cross=cross, tiny=tiny,
                    disabled=disabled, start=start, finish=finish)
        try:
            mouse.send(5, start)
            mouse.send(1, start)
            time.sleep(.05)
            for step in range(1, 13):
                point = tuple(a + (b-a)*step/12 for a,b in zip(start, finish))
                mouse.send(6, point)
                time.sleep(.02)
            selected('Capture')  # preview must not commit before pointer release
            if not (cross or tiny or disabled):
                # Prove the candidate actually moved into view before testing
                # cancellation; an undelivered drag must not pass as a cancel.
                bx, by, bw, bh = rect('Select Explore', 'AXButton')
                assert x < bx + bw/2 < x+w and y < by + bh/2 < y+h, (
                    'Drag did not reveal the candidate button', (x, y, w, h), (bx, by, bw, bh))
                case['preview_button'] = (bx, by, bw, bh)
            if cancel:
                mac.key(53)
                time.sleep(.1)
        finally:
            mouse.send(2, finish)
        expected = 'Capture' if cancel or cross or tiny or disabled else 'Explore'
        selected(expected)
        # Catch delayed commits after the up event/cancel settles.
        for _ in range(8):
            selected(expected)
            time.sleep(.05)
        case['selected'] = expected
        report['cases'].append(case)
        print('CAROUSEL_DRAG_CASE', case, flush=True)

    try:
        foreground_keys(mac)
        mac.press(TITLE, 'Carousels & journeys')
        toggle('Compact viewport')
        for theme in ('Dark', 'Light'):
            select_gallery_appearance(mac, theme)
            for axis in ('horizontal', 'vertical'):
                if axis == 'vertical':
                    mac.press(TITLE, 'Use vertical track')
                for options in ({}, {'cancel': True}, {'cross': True}, {'tiny': True}):
                    drag(axis, theme, **options)
                toggle('Disable track navigation')
                drag(axis, theme, disabled=True)
                toggle('Disable track navigation')
                if axis == 'vertical':
                    mac.press(TITLE, 'Use horizontal track')
        reset()
        focus_gallery_control(mac, 'Card draft', 'AXTextField')
        x, y, w, h = rect('Card draft', 'AXTextField')
        start, finish = (x + 12, y + h/2), (x + min(w-12, 180), y + h/2)
        mouse.check_owner(start)
        mouse.check_owner(finish)
        try:
            mouse.send(5, start)
            mouse.send(1, start)
            time.sleep(.05)
            for step in range(1, 13):
                mouse.send(6, (start[0] + (finish[0]-start[0])*step/12, start[1]))
                time.sleep(.02)
        finally:
            mouse.send(2, finish)
        selected('Capture')
        editor = mac.wait_find(TITLE, 'Card draft', 'AXTextField')
        try:
            text = mac.text(editor, 'AXSelectedText')
            assert text, 'Editor drag did not select native text'
            report['editor_selected_text'] = text
        finally:
            mac.release(editor)
        report['complete'] = True
        print('GALLERY_CAROUSEL_DRAG_OK', len(report['cases']), 'cases and child text selection', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'carousel-drag-report.json').write_text(json.dumps(report, indent=2) + '\n')
