"""Public native-menu observation walkthrough; real macOS desktop required."""
import ctypes as C


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, expect_enabled, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot

    mac.press(TITLE, 'Selection & actions')
    label = 'Preview menu'
    reveal_gallery_control(mac, label, 'AXButton')
    mac.wait_text(TITLE, 'Menu visibility: closed')
    original = mac.wait_find(TITLE, label, 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]

    def identity():
        current = mac.wait_find(TITLE, label, 'AXButton')
        try:
            assert equal(original, current), 'Menu setting replaced its owner'
        finally:
            mac.release(current)

    def open_menu(observed=True):
        focus_gallery_control(mac, label, 'AXButton')
        mac.key(125)  # Down opens the native menu.
        mac.release(mac.wait_find(TITLE, label, 'AXMenu'))
        if observed:
            mac.wait_text(TITLE, 'Menu visibility: open')
        identity()

    def close_menu(observed=True):
        mac.key(53)
        wait_absent(mac, label, 'AXMenu')
        if observed:
            mac.wait_text(TITLE, 'Menu visibility: closed')
        identity()

    def toggle(label):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    try:
        open_menu()
        mac.key(119)  # End selects the submenu.
        mac.key(124)  # Right enters it; root visibility stays open.
        mac.release(mac.wait_find(TITLE, 'More previews', 'AXMenu'))
        mac.wait_text(TITLE, 'Menu visibility: open')
        mac.key(115)  # Home, Return invokes the submenu command.
        mac.key(36)
        mac.wait_text(TITLE, 'Menu visibility: closed')
        mac.wait_text(TITLE, 'Menu preview requests: 1')
        for setting in ('Prefer menu on right', 'Align menu to trailing edge'):
            toggle(setting)
            open_menu()
            if images:
                suffix = 'right' if setting == 'Prefer menu on right' else 'right-end'
                screenshot(mac, images / f'gallery-menu-{suffix}.png', title=TITLE)
            close_menu()
        toggle('Observe menu visibility')
        mac.wait_text(TITLE, 'Menu visibility: not observed')
        open_menu(observed=False)
        close_menu(observed=False)
        toggle('Observe menu visibility')
        mac.wait_text(TITLE, 'Menu visibility: closed')
        toggle('Disable preview menu')
        expect_enabled(mac, label, False)
        action = mac.string('AXPress')
        try:
            assert mac.action(original, action) in (0, -25206)
        finally:
            mac.release(action)
        wait_absent(mac, label, 'AXMenu')
        mac.wait_text(TITLE, 'Menu visibility: closed')
        toggle('Disable preview menu')
        expect_enabled(mac, label, True)
        open_menu()
        close_menu()
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, label, 'AXButton')
        wait_absent(mac, label, 'AXMenu')
    finally:
        mac.release(original)
    print('GALLERY_MENU_OBSERVATION_OK: root visibility, submenu keyboard/action, '
          'Escape, observer detach/reattach, disabled requests, retained owner and '
          'retirement; placement pixels require separate review', flush=True)
