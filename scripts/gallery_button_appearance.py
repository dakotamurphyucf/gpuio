"""Public button style/tooltip walkthrough. Requires a real macOS desktop."""
import ctypes as C
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_enabled, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent,
    )
    from test_canvas import screenshot
    from gallery_buttons import expect_busy

    mac.press(TITLE, 'Selection & actions')
    primary = 'Primary appearance action'
    reveal_gallery_control(mac, primary, 'AXButton')
    original = mac.wait_find(TITLE, primary, 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    boolean_type = mac.cf.CFBooleanGetTypeID
    boolean_type.restype, boolean_type.argtypes = C.c_ulong, []

    def toggle(label, expected):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, label, 'AXCheckBox')
            raw = mac.attr(node, 'AXValue')
            try:
                if (raw and mac.type_id(raw) == boolean_type()
                        and int(boolean(raw)) == expected):
                    return
            finally:
                if raw:
                    mac.release(raw)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError(('Appearance setting did not settle', label, expected))

    def bounds():
        node = mac.wait_find(TITLE, primary, 'AXButton')
        try:
            assert equal(original, node), 'Restyling replaced the action owner'
            return element_rect(mac, node)
        finally:
            mac.release(node)

    variants = ('Default', 'Primary', 'Secondary', 'Danger', 'Info', 'Success',
                'Warning', 'Ghost', 'Link', 'Text', 'Custom')
    try:
        mac.wait_text(TITLE, 'Style requests: 0 · Last: None')
        x, y, width, height = bounds()
        mouse = GalleryMouse(mac)
        point = (x + width / 2, y + height / 2)
        mouse.check_owner(point)
        mouse.send(5, point)
        mac.wait_text(TITLE, 'Hovered action: Primary')
        mac.key(48)
        mac.wait_text(TITLE, 'Hovered action: None')
        for count, variant in enumerate(variants, 1):
            label = variant + ' appearance action'
            role = 'AXLink' if variant == 'Link' else 'AXButton'
            reveal_gallery_control(mac, label, role)
            activate(mac, mac.wait_find(TITLE, label, role))
            mac.wait_text(TITLE, f'Style requests: {count} · Last: {variant}')

        focus_gallery_control(mac, primary, 'AXButton')
        mac.wait_text(TITLE, 'A clear next step')
        mac.wait_text(TITLE, 'Use Enter or Space when this action is focused.')
        mac.key(53)  # Escape closes the native tooltip without invoking the action.
        wait_absent(mac, 'A clear next step', None)
        mac.key(36)
        mac.wait_text(TITLE, 'Style requests: 12 · Last: Primary')
        focus_gallery_control(mac, 'Text appearance action', 'AXButton')
        mac.key(49)
        mac.wait_text(TITLE, 'Style requests: 13 · Last: Text')

        initial = bounds()
        toggle('Compact action spacing', 1)
        compact = bounds()
        assert compact[2] < initial[2] - 1, ('Compact width', initial, compact)
        toggle('Large action sizes', 1)
        large = bounds()
        assert large[3] > compact[3] + 1, ('Large height', compact, large)
        for label in ('Outline action styles', 'Rounded action corners',
                      'Selected action appearance'):
            toggle(label, 1)
            bounds()  # Still AXButton, not an invented toggle role.
        toggle('Disable appearance actions', 1)
        expect_enabled(mac, primary, False)
        expect_enabled(mac, 'Link appearance action', False, role='AXLink')
        action = mac.string('AXPress')
        try:
            assert mac.action(original, action) in (0, -25206)
        finally:
            mac.release(action)
        toggle('Disable appearance actions', 0)
        expect_enabled(mac, primary, True)
        activate(mac, mac.wait_find(TITLE, primary, 'AXButton'))
        mac.wait_text(TITLE, 'Style requests: 14 · Last: Primary')
        link_label = 'Link appearance action'
        link = mac.wait_find(TITLE, link_label, 'AXLink')
        try:
            expect_busy(mac, link, False)
            resting_width = element_rect(mac, link)[2]
            toggle('Load link preview', 1)
            expect_busy(mac, link, True)
            # Link.Config.label owns its semantic name; passive text is not a
            # second AX leaf. Check changed layout and capture the visible text.
            reveal_gallery_control(mac, link_label, 'AXLink')
            deadline = time.monotonic() + 5
            while element_rect(mac, link)[2] <= resting_width + 10:
                assert time.monotonic() < deadline, 'Loading link content did not expand'
                time.sleep(.025)
            if images:
                screenshot(mac, images / 'gallery-link-loading.png', title=TITLE)
            expect_enabled(mac, link_label, True, role='AXLink')
            focus_gallery_control(mac, link_label, 'AXLink')
            mac.key(36)
            mac.key(49)
            action = mac.string('AXPress')
            try:
                assert mac.action(link, action) in (0, -25206)
            finally:
                mac.release(action)
            mac.wait_text(TITLE, 'Style requests: 14 · Last: Primary')
            toggle('Disable appearance actions', 1)
            expect_enabled(mac, link_label, False, role='AXLink')
            toggle('Load link preview', 0)
            expect_busy(mac, link, False)
            deadline = time.monotonic() + 5
            while abs(element_rect(mac, link)[2] - resting_width) > 1:
                assert time.monotonic() < deadline, 'Link content width did not recover'
                time.sleep(.025)
            expect_enabled(mac, link_label, False, role='AXLink')
            toggle('Disable appearance actions', 0)
            current = mac.wait_find(TITLE, link_label, 'AXLink')
            try:
                assert equal(link, current), 'Loading replaced the native link owner'
            finally:
                mac.release(current)
            focus_gallery_control(mac, link_label, 'AXLink')
            mac.key(36)
            mac.wait_text(TITLE, 'Style requests: 15 · Last: Link')
        finally:
            mac.release(link)
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            reveal_gallery_control(mac, primary, 'AXButton')
            bounds()
            if images:
                screenshot(mac, images / f'gallery-button-styles-{theme.lower()}.png', title=TITLE)
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, primary, 'AXButton')
        wait_absent(mac, 'Link appearance action', 'AXLink')
    finally:
        mac.release(original)
    print('GALLERY_BUTTON_APPEARANCE_OK: variant actions/link semantics, rich focus '
          'tooltip/Escape, Return/Space, sizing, restyle identity, disabled action '
          'rejection, Link loading/focus/recovery, themes and page retirement; '
          'pixel styling still requires review', flush=True)
    print('GALLERY_BUTTON_APPEARANCE_OK: eleven variants, hover, tooltip focus/Escape, '
          'Return/Space, dimensions, disabled/loading policies, retained owners, '
          'loading-content geometry and both themes; page retirement', flush=True)
