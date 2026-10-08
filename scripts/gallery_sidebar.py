"""Real desktop branch policy, retained identity and sidebar collapse checks."""
import ctypes as C
import json
import time


def exercise(mac, images=None):
    from mac_input_source import foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )

    report = {'complete': False, 'cases': [],
              'scope': 'native actions, OS keys/pointer and sampled AX geometry; not VoiceOver or FPS'}
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    retained = None
    original_post = mac.post_key

    def present(label, role='AXLink'):
        mac.release(mac.wait_find(TITLE, label, role))

    def press_link(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXLink'))
        mac.wait_text(TITLE, 'Destination: ' + label)

    def expanded(value):
        present(('Collapse ' if value else 'Expand ') + 'Projects', 'AXButton')
        if value:
            present('Observatory')
        else:
            wait_absent(mac, 'Observatory', 'AXLink')

    def identity():
        current = mac.wait_find(TITLE, 'Projects', 'AXLink')
        try:
            assert equal(retained, current), 'Sidebar link was replaced during a model/style update'
        finally:
            mac.release(current)

    def checked(expected):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, 'Style sidebar labels', 'AXCheckBox')
            value = mac.attr(node, 'AXValue')
            try:
                if value and bool(boolean(value)) == expected:
                    return
            finally:
                if value:
                    mac.release(value)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError(('Sidebar styling state did not settle', expected))

    def current(options):
        for name in options:
            node = mac.find(TITLE, name, 'AXButton')
            if node:
                mac.release(node)
                return name
        raise AssertionError(('Missing shell control', options))

    def scale(wanted):
        names = ('Compact', 'Comfortable', 'Large')
        for _ in range(3):
            value = current(names)
            if value == wanted:
                return
            mac.press(TITLE, value)
            present(names[(names.index(value) + 1) % 3], 'AXButton')
        assert current(names) == wanted

    try:
        mac.press(TITLE, 'Carousels & journeys')
        mac.wait_text(TITLE, 'Destination: Orchard')
        reveal_gallery_control(mac, 'Projects', 'AXLink')
        retained = mac.wait_find(TITLE, 'Projects', 'AXLink')
        expanded(True)
        mac.press(TITLE, 'Collapse Projects')
        expanded(False)
        mac.wait_text(TITLE, 'Destination: Orchard')  # Caret does not navigate.
        press_link('Projects')
        expanded(False)  # Select_only does not open the branch.
        identity()
        mac.press(TITLE, 'Branch selection: navigate')
        press_link('Projects')
        expanded(True)
        press_link('Projects')
        expanded(True)  # Expand is idempotent.
        identity()
        mac.press(TITLE, 'Branch selection: expand')

        # Real foreground Return and Space, rather than AXPress simulations.
        reveal_gallery_control(mac, 'Projects', 'AXLink')
        focus_gallery_control(mac, 'Projects', 'AXLink')
        foreground_keys(mac)
        mac.key(36)
        expanded(False)
        mac.key(49)
        expanded(True)
        identity()
        report['branch_policies'] = ['Select_only', 'Expand', 'Toggle via Return/Space']

        for theme in ('Dark', 'Light'):
            value = current(('Dark', 'Light'))
            if value != theme:
                mac.press(TITLE, value)
                present(theme, 'AXButton')
            for density in ('Comfortable', 'Large', 'Compact'):
                scale(density)
                for styled in (False, True):  # The example initially enables styling.
                    activate(mac, mac.wait_find(TITLE, 'Style sidebar labels', 'AXCheckBox'))
                    checked(styled)
                    reveal_gallery_control(mac, 'Observatory', 'AXLink')
                    identity()
                    nodes = [mac.wait_find(TITLE, name, 'AXLink')
                             for name in ('Projects', 'Orchard', 'Observatory', 'Archive')]
                    try:
                        bounds = [element_rect(mac, node) for node in nodes]
                    finally:
                        for node in nodes:
                            mac.release(node)
                    assert all(w > 20 and h > 10 for _, _, w, h in bounds), bounds
                    assert all(a[1] + a[3] <= b[1] + .5
                               for a, b in zip(bounds, bounds[1:])), ('Overlapping destinations', bounds)
                    report['cases'].append(dict(theme=theme, scale=density,
                                                styled=styled, bounds=bounds))
                    if images and styled:
                        screenshot(mac, images / f'sidebar-{theme}-{density}.png', title=TITLE)

        # The pointer routes through the painted composed link to Bonsai.
        x, y, w, h = reveal_gallery_control(mac, 'Observatory', 'AXLink')
        mouse = GalleryMouse(mac)
        point = (x + w / 2, y + h / 2)
        mouse.check_owner(point)
        for kind in (5, 1, 2):
            mouse.send(kind, point)
        mac.wait_text(TITLE, 'Destination: Observatory')
        mac.press(TITLE, 'Collapse sidebar')
        wait_absent(mac, 'Observatory', 'AXLink')
        identity()  # Compact mode retains the labelled branch destination.
        press_link('Projects')  # Toggle stored expansion while children are hidden.
        wait_absent(mac, 'Observatory', 'AXLink')
        mac.press(TITLE, 'Expand sidebar')
        expanded(False)
        mac.press(TITLE, 'Expand Projects')
        expanded(True)
        mac.wait_text(TITLE, 'Destination: Projects')
        press_link('Observatory')
        mac.press(TITLE, 'Use offcanvas sidebar')
        mac.press(TITLE, 'Collapse sidebar')
        wait_absent(mac, 'Projects', 'AXLink')
        mac.press(TITLE, 'Expand sidebar')
        current_link = mac.wait_find(TITLE, 'Projects', 'AXLink')
        try:
            # AccessKit removes macOS platform objects for excluded subtrees
            # (event.rs node_updated/remove_subtree), even if GPUIO retains
            # the widget. CFEqual is only an oracle while continuously exposed.
            report['same_ax_object_after_offcanvas'] = bool(equal(retained, current_link))
        finally:
            mac.release(current_link)
        expanded(True)
        mac.wait_text(TITLE, 'Destination: Observatory')
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, 'Observatory', 'AXLink')
        mac.press(TITLE, 'Carousels & journeys')
        mac.wait_text(TITLE, 'Destination: Observatory')
        expanded(True)
        reveal_gallery_control(mac, 'Archive', 'AXLink')
        focus_gallery_control(mac, 'Archive', 'AXLink')
        mac.key(36)
        mac.wait_text(TITLE, 'Destination: Archive')
        report['complete'] = True
        print('GALLERY_SIDEBAR_OK', len(report['cases']), 'appearance cases; policies, OS input, collapse and remount', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if retained:
            mac.release(retained)
        if images:
            (images / 'sidebar-report.json').write_text(json.dumps(report, indent=2) + '\n')
