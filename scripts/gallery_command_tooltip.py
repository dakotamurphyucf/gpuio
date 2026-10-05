"""Live command tooltip walkthrough; requires actual macOS AX and keyboard input."""
import ctypes as C
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, expect_enabled, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot

    mac.press(TITLE, 'Selection & actions')
    label = 'Run hinted action'
    reveal_gallery_control(mac, label, 'AXButton')
    original = mac.wait_find(TITLE, label, 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]

    def toggle(name):
        # AXPress leaves focus on the hinted action. After Escape, focusing that
        # already-focused action does not re-enter it or reopen its tooltip.
        focus_gallery_control(mac, name, 'AXCheckBox')
        mac.key(49)

    def hint(description, chord=None):
        focus_gallery_control(mac, label, 'AXButton')
        mac.wait_text(TITLE, 'Run a local preview action')
        mac.wait_text(TITLE, description)
        if chord:
            # Kbd exposes its accessible label, independently of painted glyphs.
            mac.wait_text(TITLE, chord)
        current = mac.wait_find(TITLE, label, 'AXButton')
        try:
            assert equal(original, current), 'Hint update replaced command owner'
        finally:
            mac.release(current)

    try:
        hint('Assigned shortcut', 'Shift + Command + H')
        mac.wait_text(TITLE, 'Hinted action requests: 0')
        mac.key(53)
        wait_absent(mac, 'Run a local preview action', None)
        mac.key(4, (1 << 20) | (1 << 17))  # Command-Shift-H.
        mac.wait_text(TITLE, 'Hinted action requests: 1')
        toggle('Alternate hinted shortcut')
        hint('Assigned shortcut', 'Shift + Command + J')
        mac.key(4, (1 << 20) | (1 << 17))
        time.sleep(.1)
        mac.wait_text(TITLE, 'Hinted action requests: 1')
        mac.key(38, (1 << 20) | (1 << 17))  # Command-Shift-J.
        mac.wait_text(TITLE, 'Hinted action requests: 2')
        toggle('Linux hint labels')
        hint('Assigned shortcut', 'Control + Shift + J')
        toggle('Assign hinted shortcut')
        hint('No shortcut assigned')
        wait_absent(mac, 'Control + Shift + J', None)
        mac.key(38, (1 << 20) | (1 << 17))
        time.sleep(.1)
        mac.wait_text(TITLE, 'Hinted action requests: 2')
        # A command without a shortcut is still an ordinary activatable action.
        activate(mac, mac.wait_find(TITLE, label, 'AXButton'))
        mac.wait_text(TITLE, 'Hinted action requests: 3')
        toggle('Enable hinted action')
        expect_enabled(mac, label, False)
        action = mac.string('AXPress')
        try:
            assert mac.action(original, action) in (0, -25206)
        finally:
            mac.release(action)
        time.sleep(.1)
        mac.wait_text(TITLE, 'Hinted action requests: 3')
        toggle('Assign hinted shortcut')
        toggle('Enable hinted action')
        hint('Assigned shortcut', 'Control + Shift + J')
        if images:
            screenshot(mac, images / 'gallery-command-tooltip.png', title=TITLE)
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, label, 'AXButton')
        wait_absent(mac, 'Run a local preview action', None)
    finally:
        mac.release(original)
    print('GALLERY_COMMAND_TOOLTIP_OK: observed shortcut replacement/removal, '
          'display platforms, action identity, invocation, disabling, Escape and '
          'page retirement', flush=True)
