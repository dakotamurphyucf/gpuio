"""Installed date-picker draft presets and read-only policy walkthrough."""
import ctypes as C
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, expect_enabled, expect_focus, expect_popup_expanded, open_picker,
        within,
    )
    from test_canvas import screenshot

    trigger = 'Choose appointment'
    cancel = 'Cancel appointment'
    apply = 'Apply appointment'
    group = 'Preview appointment'
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    boolean_type = mac.cf.CFBooleanGetTypeID
    boolean_type.restype, boolean_type.argtypes = C.c_ulong, []

    def focus_trace(stage):
        node = mac.attr(mac.app, 'AXFocusedUIElement')
        try:
            print('DATE_PRESET_FOCUS', stage,
                  (mac.text(node, 'AXRole'), mac.text(node, 'AXTitle')) if node else None,
                  flush=True)
        finally:
            if node:
                mac.release(node)

    def selected(day, expected):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            node = within(mac, group, day, 'AXCheckBox')
            raw = mac.attr(node, 'AXValue')
            try:
                assert raw and mac.type_id(raw) == boolean_type()
                if bool(boolean(raw)) == expected:
                    return
            finally:
                if raw:
                    mac.release(raw)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError((day, 'selected', expected))

    def saved(value):
        mac.wait_text(TITLE, 'Appointment: ' + value)

    def choose(preset, day, *, checked=True):
        expect_enabled(mac, preset, True)
        focus_trace('before ' + preset)
        mac.press(TITLE, preset)
        selected(day, checked)
        expect_enabled(mac, apply, True)
        expect_popup_expanded(mac, trigger, True)
        focus_trace('after ' + preset)

    def close(label):
        mac.press(TITLE, label)
        focus_trace('after ' + label)
        expect_focus(mac, trigger)
        expect_popup_expanded(mac, trigger, False)

    today = 'September 14, 2026, Today'
    later = 'September 21, 2026'
    open_picker(mac, trigger, cancel)
    choose('One week later', later)
    selected(today, False)
    saved('2026-09-14')
    if images:
        screenshot(mac, images / 'gallery-date-preset-draft.png', title=TITLE)
    close(cancel)
    saved('2026-09-14')

    open_picker(mac, trigger, cancel)
    selected(today, True)
    selected(later, False)
    choose('One week later', later)
    close(apply)
    saved('2026-09-21')

    open_picker(mac, trigger, cancel)
    choose('Clear date', later, checked=False)
    saved('2026-09-21')
    mac.key(53)
    expect_focus(mac, trigger)
    expect_popup_expanded(mac, trigger, False)
    saved('2026-09-21')

    open_picker(mac, trigger, cancel)
    selected(later, True)
    choose('Clear date', later, checked=False)
    close(apply)
    saved('No date selected')

    open_picker(mac, trigger, cancel)
    choose('Demo day', today)
    close(apply)
    saved('2026-09-14')

    # Read-only still permits opening and inspection, while both preset commands
    # and Apply are fenced. Exercise an AX action too, not just disabled paint.
    activate(mac, mac.wait_find(TITLE, 'Read-only pickers', 'AXCheckBox'))
    expect_enabled(mac, 'Clear appointment', False)
    expect_enabled(mac, 'Clear accent', False)
    open_picker(mac, trigger, cancel)
    for preset in ('Demo day', 'One week later', 'Clear date'):
        expect_enabled(mac, preset, False)
        mac.press(TITLE, preset)
        selected(today, True)
        selected(later, False)
    expect_enabled(mac, apply, False)
    mac.press(TITLE, apply)
    expect_popup_expanded(mac, trigger, True)
    saved('2026-09-14')
    if images:
        screenshot(mac, images / 'gallery-date-preset-read-only.png', title=TITLE)
    close(cancel)
    activate(mac, mac.wait_find(TITLE, 'Read-only pickers', 'AXCheckBox'))
    expect_enabled(mac, 'Clear appointment', True)
    expect_enabled(mac, 'Clear accent', True)
    print('GALLERY_DATE_PRESETS_OK: native drafts, explicit Apply, Cancel/Escape, '
          'clear/reopen, read-only command fences and focus return', flush=True)
