"""Real Tab traversal through visible carousel cards across preview scales."""
import ctypes as C
import json
import time


def exercise(mac, images=None):
    from mac_input_source import Sources, foreground_keys
    from test_gallery import (
        TITLE, activate, element_rect, expect_field, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent,
    )

    label = 'Measured idea cards'
    report = {'complete': False, 'cases': [],
              'scope': 'foreground Tab and AX clipping/retention; not VoiceOver or presentation timing'}
    original_post = mac.post_key
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    sources = Sources(mac)
    try:
        source = sources.selected()
    finally:
        sources.close()
    assert source in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC'), source
    report['input_source'] = source

    def toggle(name):
        activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))

    def bounds():
        node = mac.wait_find(TITLE, label, 'AXGroup')
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def viewport():
        reveal_gallery_control(mac, label, 'AXGroup', scroll_in_left_gutter=True)
        focus_gallery_control(mac, label, 'AXGroup')

    def selected(name):
        mac.wait_text(TITLE, 'Selected card: '+name)

    def current_focus():
        node = mac.attr(mac.app, 'AXFocusedUIElement')
        try:
            return [mac.text(node, 'AXRole'), mac.text(node, 'AXTitle')] if node else None
        finally:
            if node:
                mac.release(node)

    def controls():
        viewport_node = mac.wait_find(TITLE, label, 'AXGroup')
        group = mac.attr(viewport_node, 'AXParent')
        found = {}
        def visit(node):
            title, role = mac.text(node, 'AXTitle'), mac.text(node, 'AXRole')
            if role == 'AXButton' and title in ('Previous', 'Next'):
                assert title not in found, 'Ambiguous carousel controls'
                found[title] = mac.retain(node)
            children = mac.children(node)
            try:
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        try:
            assert group
            visit(group)
            assert set(found) == {'Previous', 'Next'}, found
            return found
        except BaseException:
            for node in found.values():
                mac.release(node)
            raise
        finally:
            if group:
                mac.release(group)
            mac.release(viewport_node)

    def traverse(names, control):
        targets = []
        buttons = controls()
        try:
            for name in names:
                role = 'AXTextField' if name == 'Card draft' else 'AXButton'
                targets.append(mac.wait_find(TITLE, name, role))
            targets.append(mac.retain(buttons[control]))
            observed = []
            # A full forward pass ends at a scoped carousel control; hidden cards
            # would create an extra stop and fail the exact identity sequence.
            for node in targets:
                mac.key(48)
                deadline = time.monotonic()+3
                while time.monotonic() < deadline:
                    actual = mac.attr(mac.app, 'AXFocusedUIElement')
                    try:
                        if actual and equal(actual, node):
                            break
                    finally:
                        if actual:
                            mac.release(actual)
                    time.sleep(.025)
                else:
                    value = mac.attr(node, 'AXFocused')
                    boolean = mac.cf.CFBooleanGetValue
                    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
                    try:
                        focused = bool(boolean(value)) if value else None
                    finally:
                        if value:
                            mac.release(value)
                    raise AssertionError(('Unexpected Tab target', mac.text(node, 'AXTitle'), current_focus(), 'target AXFocused', focused))
                observed.append(current_focus())
            for node in reversed(targets[:-1]):
                mac.key(48, flags=1 << 17)
                deadline = time.monotonic()+3
                while time.monotonic() < deadline:
                    actual = mac.attr(mac.app, 'AXFocusedUIElement')
                    try:
                        if actual and equal(actual, node):
                            break
                    finally:
                        if actual:
                            mac.release(actual)
                    time.sleep(.025)
                else:
                    raise AssertionError(('Unexpected Shift-Tab target', mac.text(node, 'AXTitle'), current_focus()))
            return observed
        finally:
            for node in targets:
                mac.release(node)
            for node in buttons.values():
                mac.release(node)

    try:
        foreground_keys(mac)
        mac.press(TITLE, 'Carousels & journeys')
        toggle('Compact viewport')
        toggle('Animate card movement')
        viewport()
        focus_gallery_control(mac, 'Card draft', 'AXTextField')
        mac.key(0, flags=1 << 20)
        mac.key(0)
        expect_field(mac, TITLE, 'Card draft', 'a')
        for theme in ('Dark', 'Light'):
            select_gallery_appearance(mac, theme)
            for scale in ('Comfortable', 'Large', 'Compact'):
                mac.release(mac.wait_find(TITLE, scale, 'AXButton'))
                for axis in ('horizontal', 'vertical'):
                    if axis == 'vertical':
                        mac.press(TITLE, 'Use vertical track')
                    viewport()
                    mac.key(115)
                    selected('Capture')
                    wait_absent(mac, 'Select Refine', 'AXButton')
                    wait_absent(mac, 'Select Publish', 'AXButton')
                    names = ['Card draft', 'Select Explore'] if axis == 'horizontal' else ['Card draft']
                    if axis == 'vertical':
                        wait_absent(mac, 'Select Explore', 'AXButton')
                    case = dict(theme=theme, scale=scale, axis=axis, compact_bounds=bounds())
                    report['cases'].append(case)
                    case['start_tabs'] = traverse(names, 'Next')
                    viewport()
                    mac.key(119)
                    selected('Publish')
                    wait_absent(mac, 'Card draft', 'AXTextField')
                    wait_absent(mac, 'Select Explore', 'AXButton')
                    names = ['Select Refine', 'Select Publish'] if axis == 'horizontal' else ['Select Publish']
                    if axis == 'vertical':
                        wait_absent(mac, 'Select Refine', 'AXButton')
                    case['end_tabs'] = traverse(names, 'Previous')
                    viewport()
                    mac.key(115)
                    selected('Capture')
                    expect_field(mac, TITLE, 'Card draft', 'a')
                    toggle('Compact viewport')
                    viewport()
                    case['expanded_bounds'] = bounds()
                    assert case['expanded_bounds'][2] > case['compact_bounds'][2]+1
                    selected('Capture')
                    expect_field(mac, TITLE, 'Card draft', 'a')
                    toggle('Compact viewport')
                    case['complete'] = True
                    print('CAROUSEL_FOCUS_CASE', case, flush=True)
                    if axis == 'vertical':
                        mac.press(TITLE, 'Use horizontal track')
                mac.press(TITLE, scale)
        report['complete'] = True
        print('GALLERY_CAROUSEL_FOCUS_OK', len(report['cases']), 'axis/theme/scale cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'carousel-focus-report.json').write_text(json.dumps(report, indent=2)+'\n')
