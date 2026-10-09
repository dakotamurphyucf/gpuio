"""Public rich/loading/focus button walkthrough; requires a real macOS desktop.

No headless acceptance is implied by importing or compiling this driver.
Busy checks require both blocked activation and the native AXElementBusy Boolean.
Local execution evidence is recorded in docs/evidence/installed-actions-och41.md.
"""
import ctypes as C
import time



def expect_busy(mac, node, expected):
    boolean_type = mac.cf.CFBooleanGetTypeID
    boolean_type.restype, boolean_type.argtypes = C.c_ulong, []
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        value = mac.attr(node, 'AXElementBusy')
        try:
            if value:
                assert mac.type_id(value) == boolean_type(), 'AXElementBusy must be a Boolean'
                if bool(boolean(value)) == expected:
                    return
        finally:
            if value:
                mac.release(value)
        time.sleep(.025)
    raise AssertionError(('AXElementBusy did not settle', expected))


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_focus,
        raise_gallery, reveal_gallery_control, within,
    )
    from test_canvas import screenshot

    mac.press(TITLE, 'Selection & actions')
    primary_group, secondary_group = 'Primary publish action', 'Secondary publish action'
    reveal_gallery_control(mac, secondary_group, 'AXGroup')
    original = within(mac, primary_group, 'Publish draft', 'AXButton')
    peer = within(mac, secondary_group, 'Publish draft', 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    boolean_type = mac.cf.CFBooleanGetTypeID
    boolean_type.restype, boolean_type.argtypes = C.c_ulong, []

    def focused(node, wanted=True):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            raw = mac.attr(node, 'AXFocused')
            try:
                if raw and bool(boolean(raw)) == wanted:
                    return
            finally:
                if raw:
                    mac.release(raw)
            time.sleep(.025)
        raise AssertionError(('Button focus did not settle', wanted))

    def focus(node):
        raise_gallery(mac)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            mac.set(node, 'AXFocused', mac.true)
            time.sleep(.04)
            raw = mac.attr(node, 'AXFocused')
            try:
                if raw and boolean(raw):
                    return
            finally:
                if raw:
                    mac.release(raw)
        raise AssertionError('Initial button focus did not settle')

    def identity():
        for group, previous in [(primary_group, original), (secondary_group, peer)]:
            node = within(mac, group, 'Publish draft', 'AXButton')
            try:
                assert equal(previous, node), ('Button owner changed', group)
                raw = mac.attr(node, 'AXEnabled')
                try:
                    assert raw and boolean(raw), 'Loading must not become disabled'
                finally:
                    if raw:
                        mac.release(raw)
            finally:
                mac.release(node)

    def toggle(label, expected):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, label, 'AXCheckBox')
            raw = mac.attr(node, 'AXValue')
            try:
                if (raw and mac.type_id(raw) == boolean_type()
                        and int(boolean(raw)) == expected):
                    identity()
                    return
            finally:
                if raw:
                    mac.release(raw)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError(('Checkbox update did not settle', label, expected))

    def click_primary():
        reveal_gallery_control(mac, primary_group, 'AXGroup')
        x, y, w, h = element_rect(mac, original)
        assert w > 0 and h > 0
        point = (x + w / 2, y + h / 2)
        mouse = GalleryMouse(mac)
        mouse.check_owner(point)
        mouse.send(5, point)
        mouse.send(1, point)
        mouse.send(2, point)

    try:
        expect_busy(mac, original, False)
        expect_busy(mac, peer, False)
        mac.wait_text(TITLE, 'Publish requests: 0')
        mac.perform(original, 'AXPress')
        mac.wait_text(TITLE, 'Publish requests: 1')
        focus(original)
        mac.key(36)
        mac.wait_text(TITLE, 'Publish requests: 2')
        toggle('Primary button busy', 1)
        expect_busy(mac, original, True)
        expect_busy(mac, peer, False)
        focused(original)
        mac.perform(original, 'AXPress')
        mac.key(49)
        # The sibling acts as an event-order barrier: blocked primary actions
        # must not add requests before this shared-command invocation.
        mac.perform(peer, 'AXPress')
        mac.wait_text(TITLE, 'Publish requests: 3')
        focused(original)
        if images:
            screenshot(mac, images / 'gallery-button-busy.png', title=TITLE)
        toggle('Primary button busy', 0)
        expect_busy(mac, original, False)
        toggle('Detailed button content', 0)
        focused(original)
        mac.key(36)
        mac.wait_text(TITLE, 'Publish requests: 4')
        toggle('Detailed button content', 1)
        toggle('Skip primary with Tab', 1)
        focus(peer)
        mac.key(48, 1 << 17)  # Shift-Tab skips the primary owner.
        expect_focus(mac, 'Skip primary with Tab', 'AXCheckBox')
        click_primary()
        mac.wait_text(TITLE, 'Publish requests: 5')
        focused(original)
        toggle('Skip primary with Tab', 0)
        toggle('Preserve existing focus', 1)
        focus(peer)
        click_primary()
        mac.wait_text(TITLE, 'Publish requests: 6')
        focused(peer)
        focused(original, False)
        mac.key(36)
        mac.wait_text(TITLE, 'Publish requests: 7')
        toggle('Preserve existing focus', 0)
        identity()
        mac.press(TITLE, 'Presentation')  # Unmount the card and its native clocks.
    finally:
        mac.release(original)
        mac.release(peer)
    print('GALLERY_BUTTONS_OK: rich/plain identity, shared-command loading, real '
          'Return/Space/pointer, skipped Tab and preserved sibling focus; '
          'external AXElementBusy checked; VoiceOver remains unqualified', flush=True)
