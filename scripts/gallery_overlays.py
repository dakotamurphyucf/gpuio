"""Native overlay focus, live sheet geometry and point-placement qualification."""
import ctypes as C
import json
import time


def exercise(mac, images=None):
    from mac_input_source import foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, activate, element_rect, exercise_overlays, expect_focus,
        focus_gallery_control, open_picker, reveal_gallery_control,
        select_gallery_appearance, wait_absent,
    )
    from test_gallery_editor_geometry import content_origin

    report = {'complete': False, 'cases': []}
    original_post = mac.post_key
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]

    def press(label):
        mac.press(TITLE, label)

    def rectangle(label):
        node = mac.wait_find(TITLE, label, 'AXWindow')
        try:
            return list(element_rect(mac, node))
        finally:
            mac.release(node)

    def wait_rectangle(label, expected, *, tolerance=1.):
        deadline = time.monotonic() + 5
        while True:
            actual = rectangle(label)
            if all(abs(a-b) <= tolerance for a, b in zip(actual, expected)):
                return actual
            assert time.monotonic() < deadline, (label, expected, actual)
            time.sleep(.03)

    def checked(label):
        node = mac.wait_find(TITLE, label, 'AXCheckBox')
        value = mac.attr(node, 'AXValue')
        try:
            assert value
            return bool(boolean(value))
        finally:
            if value:
                mac.release(value)
            mac.release(node)

    def setting(label, value):
        if checked(label) != value:
            activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        deadline = time.monotonic() + 5
        while checked(label) != value:
            assert time.monotonic() < deadline, ('checkbox did not settle', label)
            time.sleep(.025)

    def is_modal(node, expected):
        value = mac.attr(node, 'AXModal')
        try:
            assert value and bool(boolean(value)) == expected, ('AXModal', expected)
        finally:
            if value:
                mac.release(value)

    try:
        foreground_keys(mac)
        exercise_overlays(mac, images)  # Existing dialog, confirmation, popover/help cases.
        press('Presentation')
        for current, following in (('Large', 'Compact'), ('Compact', 'Comfortable')):
            node = mac.find(TITLE, current, 'AXButton')
            if node:
                activate(mac, node)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for scale, following in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                policy = 'Reduced' if scale == 'Compact' else 'Full'
                press('Motion & rhythm')
                press('Use reduced motion' if policy == 'Reduced' else 'Use full motion')
                mac.wait_text(TITLE, 'Motion preference: ' + policy)
                press('Overlays & help')
                case = {'theme': theme, 'scale': scale, 'motion': policy, 'sheets': [], 'placements': []}
                report['cases'].append(case)
                open_picker(mac, 'Open dialog', 'Close dialog')
                dialog = mac.wait_find(TITLE, 'Preview dialog', 'AXWindow')
                try:
                    is_modal(dialog, True)
                    focus_gallery_control(mac, 'Change backdrop', 'AXButton')
                    mac.key(48)
                    expect_focus(mac, 'Close dialog')
                    mac.key(48)
                    expect_focus(mac, 'Change backdrop')
                    mac.key(48, flags=1 << 17)
                    expect_focus(mac, 'Close dialog')
                    press('Change backdrop')
                    current = mac.wait_find(TITLE, 'Preview dialog', 'AXWindow')
                    try:
                        assert equal(dialog, current), 'backdrop replaced modal frame'
                    finally:
                        mac.release(current)
                    press('Change backdrop')
                    mac.key(53)
                    wait_absent(mac, 'Preview dialog', 'AXWindow')
                    expect_focus(mac, 'Open dialog')
                finally:
                    mac.release(dialog)

                open_picker(mac, 'Open drawer', 'Close drawer')
                drawer = mac.wait_find(TITLE, 'Preview drawer', 'AXWindow')
                try:
                    is_modal(drawer, True)
                    # Full entry can still be settling after AX appears. The
                    # explicit source extent is380; allow entry to reach its end.
                    time.sleep(.3)
                    for edge in ('Right', 'Bottom', 'Left', 'Top'):
                        setting('Reserve space for app chrome', False)
                        time.sleep(.05)
                        base = rectangle('Preview drawer')
                        x, y, w, h = base
                        expected = {
                            'Right': [x-16, y+56, w, h-72],
                            'Bottom': [x+16, y-16, w-32, h],
                            'Left': [x+16, y+56, w, h-72],
                            'Top': [x+16, y+56, w-32, h],
                        }[edge]
                        setting('Reserve space for app chrome', True)
                        inset = wait_rectangle('Preview drawer', expected)
                        current = mac.wait_find(TITLE, 'Preview drawer', 'AXWindow')
                        try:
                            assert equal(drawer, current), 'edge/insets replaced drawer frame'
                        finally:
                            mac.release(current)
                        case['sheets'].append({'edge': edge, 'base': base, 'inset': inset})
                        if images and edge == 'Right':
                            screenshot(mac, images / f'drawer-{theme}-{scale}.png', title=TITLE)
                        setting('Reserve space for app chrome', False)
                        wait_rectangle('Preview drawer', base)
                        press('Move drawer to next edge')
                        time.sleep(.06)
                    focus_gallery_control(mac, 'Close drawer', 'AXButton')
                    mac.key(48)
                    expect_focus(mac, 'Reserve space for app chrome', 'AXCheckBox')
                    mac.key(53)
                    wait_absent(mac, 'Preview drawer', 'AXWindow')
                    expect_focus(mac, 'Open drawer')
                finally:
                    mac.release(drawer)

                reveal_gallery_control(mac, 'Contributor', 'AXButton')
                focus_gallery_control(mac, 'Contributor', 'AXButton')
                mac.wait_text(TITLE, 'Building thoughtful native interfaces.')
                card = mac.wait_find(TITLE, 'Contributor profile', 'AXWindow')
                try:
                    is_modal(card, False)
                finally:
                    mac.release(card)
                focus_gallery_control(mac, 'View profile', 'AXButton')
                mac.key(36)
                mac.wait_text(TITLE, 'Contributor profile selected')
                mac.key(53)
                wait_absent(mac, 'Contributor profile', 'AXWindow')
                expect_focus(mac, 'Contributor')

                reveal_gallery_control(mac, 'Open placement preview', 'AXButton')
                open_picker(mac, 'Open placement preview', 'Close placement preview')
                placement = mac.wait_find(TITLE, 'Placement preview', 'AXWindow')
                try:
                    setting('Place at window point', True)
                    setting('Roomier window margin', False)
                    ox, oy = content_origin(mac)
                    for corner in ('Top left', 'Top right', 'Bottom left', 'Bottom right'):
                        press(corner)
                        time.sleep(.06)
                        x, y, w, h = rectangle('Placement preview')
                        # This reference point leaves the comfortable panel
                        # inside the viewport. General clamp cases stay in native tests.
                        target = [ox+560-(w if 'right' in corner else 0),
                                  oy+400-(h if 'Bottom' in corner else 0), w, h]
                        actual = wait_rectangle('Placement preview', target)
                        current = mac.wait_find(TITLE, 'Placement preview', 'AXWindow')
                        try:
                            assert equal(placement, current), 'live placement replaced panel'
                        finally:
                            mac.release(current)
                        case['placements'].append({'corner': corner, 'bounds': actual})
                    press('Top left')
                    press('Close placement preview')
                    expect_focus(mac, 'Open placement preview')
                finally:
                    mac.release(placement)
                # Nonmodal overlays permit page navigation; deactivation must
                # retire the old popup instead of leaking it into another page.
                open_picker(mac, 'Show details', 'Done with details')
                press('Presentation')
                wait_absent(mac, 'Details popover', 'AXWindow')
                press('Overlays & help')
                wait_absent(mac, 'Details popover', 'AXWindow')
                case['complete'] = True
                print('GALLERY_OVERLAY_CASE', theme, scale, policy, 'PASS', flush=True)
                press('Presentation')
                press(scale)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        report['complete'] = True
        print('GALLERY_OVERLAY_MATRIX_OK', len(report['cases']), 'cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'overlay-report.json').write_text(json.dumps(report, indent=2) + '\n')
