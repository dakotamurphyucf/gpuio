"""Authored macOS calendar-content check; requires a real accessible desktop."""
import ctypes as C
import time


def exercise(mac):
    from test_gallery import TITLE, activate, reveal_gallery_control, within

    day = 'September 14, 2026, Today'
    group = 'Preview date range'
    reveal_gallery_control(mac, 'Update events', 'AXButton')
    original = within(mac, group, day, 'AXCheckBox')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    selection = mac.text(original, 'AXValue')

    def check(description):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            current = within(mac, group, day, 'AXCheckBox')
            try:
                assert equal(original, current), 'Content update replaced calendar target'
                assert mac.text(current, 'AXValue') == selection, 'Content changed selection'
                if (mac.text(current, 'AXHelp') or '') == description:
                    return
            finally:
                mac.release(current)
            time.sleep(.03)
        raise RuntimeError(f'Calendar content help did not settle to {description!r}')

    try:
        check('2 scheduled events')
        mac.press(TITLE, 'Update events')
        check('3 scheduled events')
        mac.press(TITLE, 'Event badges')
        check('')
        mac.press(TITLE, 'Event badges')
        check('3 scheduled events')
        # Native range action stays on the date target, even with custom artwork.
        activate(mac, within(mac, group, day, 'AXCheckBox'))
        mac.wait_text(TITLE, '2026-09-14 → choose an end date')
    finally:
        mac.release(original)
