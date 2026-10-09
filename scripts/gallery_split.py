"""Split action/menu public-API walkthrough; requires a real macOS desktop.

This checks ownership, input and mode changes. Shared-hover pixels need separate
capture/review; compiling this driver does not establish runtime acceptance.
"""
import ctypes as C


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, expect_enabled, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot

    mac.press(TITLE, 'Selection & actions')
    primary, trigger = 'Run split action', 'More split actions'
    reveal_gallery_control(mac, trigger, 'AXButton')
    original = mac.wait_find(TITLE, primary, 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]

    def identity(label, previous):
        current = mac.wait_find(TITLE, label, 'AXButton')
        try:
            assert equal(previous, current), ('Split mode replaced surviving owner', label)
        finally:
            mac.release(current)

    def toggle(label):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    def invoke_menu(count):
        focus_gallery_control(mac, trigger, 'AXButton')
        mac.key(125)
        mac.release(mac.wait_find(TITLE, trigger, 'AXMenu'))
        mac.key(36)
        wait_absent(mac, trigger, 'AXMenu')
        mac.wait_text(TITLE, f'Split action requests: {count}')

    try:
        activate(mac, mac.wait_find(TITLE, primary, 'AXButton'))
        mac.wait_text(TITLE, 'Split action requests: 1')
        invoke_menu(2)
        toggle('Disable primary action')
        expect_enabled(mac, primary, False)
        expect_enabled(mac, trigger, True)
        invoke_menu(3)
        toggle('Disable primary action')
        toggle('Primary action loading')
        expect_enabled(mac, primary, True)
        reveal_gallery_control(mac, primary, 'AXButton')
        activate(mac, mac.wait_find(TITLE, primary, 'AXButton'))
        mac.wait_text(TITLE, 'Split action requests: 3')
        invoke_menu(4)
        toggle('Primary action loading')
        toggle('Disable split pair')
        expect_enabled(mac, primary, False)
        expect_enabled(mac, trigger, False)
        toggle('Disable split pair')
        for mode in ('both', 'action only'):
            label = f'Split mode: {mode}'
            reveal_gallery_control(mac, label, 'AXButton')
            activate(mac, mac.wait_find(TITLE, label, 'AXButton'))
            if mode == 'both':
                wait_absent(mac, trigger, 'AXButton')
                identity(primary, original)
            else:
                wait_absent(mac, primary, 'AXButton')
        reveal_gallery_control(mac, trigger, 'AXButton')
        menu_only = mac.wait_find(TITLE, trigger, 'AXButton')
        try:
            label = 'Split mode: menu only'
            reveal_gallery_control(mac, label, 'AXButton')
            activate(mac, mac.wait_find(TITLE, label, 'AXButton'))
            identity(trigger, menu_only)
        finally:
            mac.release(menu_only)
        invoke_menu(5)
        if images:
            screenshot(mac, images / 'gallery-split-restored.png', title=TITLE)
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, trigger, 'AXButton')
        wait_absent(mac, trigger, 'AXMenu')
    finally:
        mac.release(original)
    print('GALLERY_SPLIT_OK: independent action/menu commands, disabled/loading '
          'policy, native menu keyboard, mode retention and retirement; '
          'shared-hover pixels require separate validation', flush=True)
