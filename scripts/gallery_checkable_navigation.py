"""Standalone radio contracts through real macOS input and the installed gallery.

Ordinary AX inspection only; this does not launch or qualify VoiceOver.
"""
import ctypes as C
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, raise_gallery,
        reveal_gallery_control, select_gallery_appearance, wait_absent, within,
    )
    from test_canvas import screenshot

    group = 'Response depth'
    labels = ('Balanced', 'Fast', 'Deep')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    boolean_type = mac.cf.CFBooleanGetTypeID
    boolean_type.restype, boolean_type.argtypes = C.c_ulong, []

    def radio(label):
        # Other cards also contain Fast/Balanced radios. Never focus or assert
        # their state through an unscoped label lookup.
        return within(mac, group, label, 'AXRadioButton')

    def flag(node, name):
        raw = mac.attr(node, name)
        try:
            assert raw and mac.type_id(raw) == boolean_type(), (name, 'not Boolean')
            return bool(boolean(raw))
        finally:
            if raw:
                mac.release(raw)

    def expect_flag(label, name, expected, *, checkbox=False):
        deadline, actual = time.monotonic() + 5, None
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, label, 'AXCheckBox') if checkbox else radio(label)
            try:
                actual = flag(node, name)
                if actual == expected:
                    return
            finally:
                mac.release(node)
            time.sleep(.03)
        raise AssertionError((label, name, expected, actual))

    def selection(label, *, enabled=True):
        mac.wait_text(TITLE, 'Response depth: ' + label.lower())
        for candidate in labels:
            expect_flag(candidate, 'AXValue', candidate == label)
            # The pinned macOS adapter gates AXSelected on is_selectable(),
            # which excludes disabled nodes. AXValue still exposes checked
            # state. Record that platform mapping, not a cleared model value.
            expect_flag(candidate, 'AXSelected', enabled and candidate == label)

    def toggle(label, expected):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        expect_flag(label, 'AXValue', expected, checkbox=True)
        reveal_gallery_control(mac, group, 'AXRadioGroup')

    def focus(label):
        reveal_gallery_control(mac, group, 'AXRadioGroup')
        raise_gallery(mac)
        node = radio(label)
        try:
            mac.set(node, 'AXFocused', mac.true)
        finally:
            mac.release(node)
        expect_flag(label, 'AXFocused', True)

    def identities(originals, *, retained=True):
        for label, original in zip(labels, originals):
            node = radio(label)
            try:
                assert bool(equal(original, node)) == retained, ('radio identity', label, retained)
            finally:
                mac.release(node)

    def press_retained(node):
        action = mac.string('AXPress')
        try:
            status = mac.action(node, action)
            # AppKit may reject the action synchronously or enqueue it for the
            # native generation/config fence. The committed state is checked.
            assert status in (0, -25206, -25202), ('fenced radio action', status)
        finally:
            mac.release(action)
        time.sleep(.1)

    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Selection & actions')
    reveal_gallery_control(mac, group, 'AXRadioGroup')
    root = mac.wait_find(TITLE, group, 'AXRadioGroup')
    try:
        assert mac.text(root, 'AXOrientation') == 'AXVerticalOrientation'
    finally:
        mac.release(root)
    originals = [radio(label) for label in labels]
    try:
        selection('Balanced')
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            focus('Balanced')
            mac.key(48)  # Tab follows tree order for equal indices.
            expect_flag('Fast', 'AXFocused', True)
            mac.key(49)
            selection('Fast')
            mac.key(48)
            expect_flag('Deep', 'AXFocused', True)
            mac.key(36)
            selection('Deep')
            mac.key(48, flags=1 << 17)
            expect_flag('Fast', 'AXFocused', True)
            mac.key(124)  # Independent radios do not invent Arrow navigation.
            expect_flag('Fast', 'AXFocused', True)
            selection('Deep')
            mac.key(49)
            selection('Fast')
            mac.key(49)  # Committed selection stays checked; no callback-count claim.
            selection('Fast')
            identities(originals)
            if images:
                screenshot(mac, images / f'gallery-standalone-radio-{theme.lower()}.png', title=TITLE)

        toggle('Reverse choice Tab order', True)
        focus('Deep')
        mac.key(48)
        expect_flag('Fast', 'AXFocused', True)
        mac.key(48, flags=1 << 17)
        expect_flag('Deep', 'AXFocused', True)
        # Indices are global to the native focus scope: other index-zero
        # controls intervene before Balanced. Do not imply a trapped group.
        identities(originals)
        toggle('Reverse choice Tab order', False)
        toggle('Skip Fast with Tab', True)
        focus('Balanced')
        mac.key(48)
        expect_flag('Deep', 'AXFocused', True)
        mac.key(48, flags=1 << 17)
        expect_flag('Balanced', 'AXFocused', True)
        mac.key(49)
        selection('Balanced')
        # The lower rich-label line is part of the single interactive radio.
        reveal_gallery_control(mac, group, 'AXRadioGroup')
        node = radio('Fast')
        try:
            x, y, width, height = element_rect(mac, node)
            rich_height = height
        finally:
            mac.release(node)
        mouse = GalleryMouse(mac)
        point = (x + min(60, width / 2), y + height - 8)
        mouse.check_owner(point)
        for event in (5, 1, 2):
            mouse.send(event, point)
        selection('Fast')
        expect_flag('Fast', 'AXFocused', True)
        for description in ('Time to think, room to explore', 'A concise answer with less waiting',
                            'Work through the details together'):
            wait_absent(mac, description, 'AXStaticText')
        toggle('Detailed choice labels', False)
        identities(originals)
        selection('Fast')
        node = radio('Fast')
        try:
            assert element_rect(mac, node)[3] < rich_height, 'plain label did not shrink'
        finally:
            mac.release(node)
        toggle('Disable standalone choices', True)
        for label in labels:
            expect_flag(label, 'AXEnabled', False)
        identities(originals)
        press_retained(originals[0])
        selection('Fast', enabled=False)
        toggle('Disable standalone choices', False)
        toggle('Skip Fast with Tab', False)
        toggle('Detailed choice labels', True)
        identities(originals)
        for label in labels:
            expect_flag(label, 'AXEnabled', True)
        focus('Balanced')
        mac.key(48)
        expect_flag('Fast', 'AXFocused', True)
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, group, 'AXRadioGroup')
        press_retained(originals[0])
        mac.press(TITLE, 'Selection & actions')
        reveal_gallery_control(mac, group, 'AXRadioGroup')
        identities(originals, retained=False)
        selection('Fast')  # Application model survives native page retirement.
        press_retained(originals[0])
        selection('Fast')
        focus('Deep')
        mac.key(36)
        selection('Deep')
        print('GALLERY_CHECKABLE_NAVIGATION_OK: scoped radio roles/values/selection, '
              'Light/Dark, OS Tab/Shift-Tab/Space/Return/Arrow, signed order, skipped stop '
              'with rich-label pointer selection, plain/rich and disabled identity, '
              'page retirement/stale actions/remount; VoiceOver untouched', flush=True)
    finally:
        for node in originals:
            mac.release(node)
