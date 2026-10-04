"""Public toggle composition walkthrough; desktop assertions are not headless tests."""
import ctypes as C
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, element_rect, expect_focus, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot
    from gallery_buttons import expect_busy

    mac.press(TITLE, 'Selection & actions')
    reveal_gallery_control(mac, 'Text formatting', 'AXToolbar')
    originals = [mac.wait_find(TITLE, name, 'AXToolbar')
                 for name in ('Text alignment', 'Text formatting')]
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]

    def value(label, expected):
        # AccessKit maps a Button with Toggled to AXCheckBox / AXToggle.
        number = mac.cf.CFNumberGetValue
        number.restype, number.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        number_type = mac.cf.CFNumberGetTypeID
        number_type.restype, number_type.argtypes = C.c_ulong, []
        deadline = time.monotonic() + 5
        actual = None
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, label, 'AXCheckBox')
            raw = mac.attr(node, 'AXValue')
            try:
                result = C.c_longlong()
                if raw and mac.type_id(raw) == number_type() and number(raw, 4, C.byref(result)):
                    actual = result.value
                    if actual == expected:
                        return
            finally:
                if raw:
                    mac.release(raw)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError((label, expected, actual))

    def toggle(label):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    def orientation(vertical, connected=True):
        expected = 'AXVerticalOrientation' if vertical else 'AXHorizontalOrientation'
        for name, original in zip(('Text alignment', 'Text formatting'), originals):
            node = mac.wait_find(TITLE, name, 'AXToolbar')
            try:
                assert equal(original, node), 'Orientation update replaced toolbar identity'
                assert mac.text(node, 'AXOrientation') == expected
            finally:
                mac.release(node)
        nodes = [mac.wait_find(TITLE, name, 'AXCheckBox') for name in ('Bold', 'Italic')]
        try:
            first, second = (element_rect(mac, node) for node in nodes)
            expected_gap = 0 if connected else 6
            if vertical:
                gap = second[1] - first[1] - first[3]
                assert abs(second[0] - first[0]) <= 1, (first, second)
                assert abs(second[2] - first[2]) <= 1, (first, second)
            else:
                gap = second[0] - first[0] - first[2]
                assert abs(second[1] - first[1]) <= 1, (first, second)
                assert abs(second[3] - first[3]) <= 1, (first, second)
            assert abs(gap - expected_gap) <= 1, (gap, expected_gap, first, second)
        finally:
            for node in nodes:
                mac.release(node)

    try:
        orientation(False)
        value('Bold', 1)
        value('Italic', 0)
        value('All formatting options', 2)
        toggle('All formatting options')
        for name in ('Bold', 'Italic', 'Monospace', 'All formatting options'):
            value(name, 1)
        toggle('All formatting options')
        for name in ('Bold', 'Italic', 'Monospace', 'All formatting options'):
            value(name, 0)

        focus_gallery_control(mac, 'Bold', 'AXCheckBox')
        mac.key(49)  # Space
        value('Bold', 1)
        mac.key(48)  # Tab: each native toggle remains a separate Tab stop.
        expect_focus(mac, 'Italic', 'AXCheckBox')
        mac.key(36)  # Return
        value('Italic', 1)
        value('All formatting options', 2)

        toggle('Align center')
        toggle('Align center')  # Mandatory single selection cannot become empty.
        value('Align left', 0)
        value('Align center', 1)
        value('Align right', 0)
        toggle('Align right')
        value('Align center', 0)
        value('Align right', 1)

        # The platform may expose Switch as AXCheckBox; find by its unique name.
        activate(mac, mac.wait_find(TITLE, 'Enable formatting controls'))
        deadline = time.monotonic() + 5
        false = C.c_void_p.in_dll(mac.cf, 'kCFBooleanFalse').value
        while True:
            node = mac.wait_find(TITLE, 'Bold', 'AXCheckBox')
            raw = mac.attr(node, 'AXEnabled')
            try:
                disabled = raw and equal(raw, false)
            finally:
                if raw:
                    mac.release(raw)
            if disabled:
                break
            mac.release(node)
            assert time.monotonic() < deadline, 'Disabled toggle remains enabled'
            time.sleep(.025)
        action = mac.string('AXPress')
        try:
            result = mac.action(node, action)
            assert result in (0, -25206), ('disabled AXPress', result)
        finally:
            mac.release(action)
            mac.release(node)
        time.sleep(.1)
        value('Bold', 1)
        activate(mac, mac.wait_find(TITLE, 'Enable formatting controls'))

        # Child policies do not disable their siblings or replace keyed owners.
        children = [(name, mac.wait_find(TITLE, name, 'AXCheckBox'))
                    for name in ('Bold', 'Italic', 'Monospace')]
        try:
            toggle('Enable Italic option')
            value('Enable Italic option', 0)
            toggle('Bold option busy')
            value('Bold option busy', 1)
            expect_busy(mac, children[0][1], True)
            false = C.c_void_p.in_dll(mac.cf, 'kCFBooleanFalse').value
            for name, node in children:
                current = mac.wait_find(TITLE, name, 'AXCheckBox')
                try:
                    assert equal(current, node), ('Child policy replaced owner', name)
                finally:
                    mac.release(current)
                raw = mac.attr(node, 'AXEnabled')
                try:
                    assert raw and bool(equal(raw, false)) == (name == 'Italic'), name
                finally:
                    if raw:
                        mac.release(raw)
            for name, node in children[:2]:
                action = mac.string('AXPress')
                try:
                    assert mac.action(node, action) in (0, -25206), name
                finally:
                    mac.release(action)
            time.sleep(.1)
            value('Bold', 1)
            value('Italic', 1)
            focus_gallery_control(mac, 'Bold', 'AXCheckBox')
            expect_focus(mac, 'Bold', 'AXCheckBox')
            mac.key(48)
            expect_focus(mac, 'Monospace', 'AXCheckBox')
            toggle('Monospace')
            value('All formatting options', 1)
            toggle('All formatting options')
            value('Bold', 1)
            value('Italic', 1)
            value('Monospace', 0)
            value('All formatting options', 2)
            toggle('Bold option busy')
            expect_busy(mac, children[0][1], False)
            toggle('Enable Italic option')
            value('Enable Italic option', 1)
            for name, previous in children:
                current = mac.wait_find(TITLE, name, 'AXCheckBox')
                try:
                    assert equal(previous, current), ('Child recovery replaced owner', name)
                finally:
                    mac.release(current)
        finally:
            for _, node in children:
                mac.release(node)

        toggle('Vertical formatting toolbars')
        value('Vertical formatting toolbars', 1)
        orientation(True)
        value('Bold', 1)
        value('Align right', 1)
        toggle('Connected formatting buttons')
        value('Connected formatting buttons', 0)
        orientation(True, connected=False)
        toggle('Connected formatting buttons')
        value('Connected formatting buttons', 1)
        orientation(True)
        toggle('Vertical formatting toolbars')
        value('Vertical formatting toolbars', 0)
        orientation(False)
        toggle('Connected formatting buttons')
        value('Connected formatting buttons', 0)
        orientation(False, connected=False)
        toggle('Connected formatting buttons')
        value('Connected formatting buttons', 1)
        orientation(False)
        # Filtering to one child preserves that keyed native owner and its
        # selection; changing orientation must not recreate it either.
        retained = [(name, mac.wait_find(TITLE, name, 'AXCheckBox'))
                    for name in ('Bold', 'Align right')]
        try:
            toggle('Single item groups')
            value('Single item groups', 1)
            for name in ('Italic', 'Monospace', 'Align left', 'Align center'):
                wait_absent(mac, name, 'AXCheckBox')
            for vertical in (False, True, False):
                if vertical:
                    toggle('Vertical formatting toolbars')
                    value('Vertical formatting toolbars', 1)
                elif mac.text(originals[0], 'AXOrientation') == 'AXVerticalOrientation':
                    toggle('Vertical formatting toolbars')
                    value('Vertical formatting toolbars', 0)
                for name, previous in retained:
                    node = mac.wait_find(TITLE, name, 'AXCheckBox')
                    try:
                        assert equal(previous, node), ('Singleton replaced owner', name)
                    finally:
                        mac.release(node)
                    value(name, 1)
                if images:
                    screenshot(mac, images / ('gallery-single-group-vertical.png'
                               if vertical else 'gallery-single-group-horizontal.png'), title=TITLE)
            toggle('Single item groups')
            value('Single item groups', 0)
            orientation(False)
            value('Italic', 1)
        finally:
            for _, node in retained:
                mac.release(node)
        if images:
            screenshot(mac, images / 'gallery-selection.png', title=TITLE)
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, 'Text formatting', 'AXToolbar')
        wait_absent(mac, 'Text alignment', 'AXToolbar')
    finally:
        for node in originals:
            mac.release(node)
    print('GALLERY_SELECTION_OK: toolbar roles/orientation/identity, checked and mixed '
          'values, single/multiple selection, Space/Return/Tab, disabled actions and '
          'connected/separated horizontal/vertical geometry, singleton identity and '
          'independent busy/disabled children, partial bulk updates, page retirement', flush=True)
