"""macOS calendar-content check; requires a real accessible desktop."""
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
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    boolean_type = mac.cf.CFBooleanGetTypeID
    boolean_type.restype, boolean_type.argtypes = C.c_ulong, []

    def selected(node):
        raw = mac.attr(node, 'AXValue')
        try:
            assert raw and mac.type_id(raw) == boolean_type(), 'calendar checked value is not Boolean'
            return bool(boolean(raw))
        finally:
            if raw:
                mac.release(raw)

    selection = selected(original)

    def check(description):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            current = within(mac, group, day, 'AXCheckBox')
            try:
                assert equal(original, current), 'Content update replaced calendar target'
                assert selected(current) == selection, 'Content changed selection'
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
