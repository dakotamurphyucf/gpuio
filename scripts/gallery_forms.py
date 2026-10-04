"""Public Form collection geometry, native editing and lifetime checks on macOS."""
import ctypes as C
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, element_rect, expect_field, focus_gallery_control,
        reveal_gallery_control,
    )
    from test_canvas import screenshot

    mac.press(TITLE, 'Text editing')
    mac.wait_text(TITLE, 'A workspace form that adapts')
    name = 'Form workspace name'
    focus_gallery_control(mac, name, 'AXTextField')
    mac.key(0, flags=1 << 20)
    mac.key(0)
    expect_field(mac, TITLE, name, 'a')
    editor = mac.wait_find(TITLE, name, 'AXTextField')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    columns, horizontal, mixed = 2, False, False
    size = 'M'
    cases = 0

    def rect(label, role='AXGroup'):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.08)

    def identity():
        current = mac.wait_find(TITLE, name, 'AXTextField')
        try:
            assert equal(editor, current), 'Form update replaced native editor'
        finally:
            mac.release(current)
        expect_field(mac, TITLE, name, 'a')

    def geometry():
        nonlocal cases
        root = rect('Workspace form')
        label = rect('Form name label')
        control = rect(name, 'AXTextField')
        summary = rect('Form summary content')
        footer = rect('Form footer content')
        gap = {'XS': 8, 'S': 12, 'M': 16, 'L': 24}[size]
        inner_gap = {'XS': 4, 'S': 6, 'M': 8, 'L': 12}[size]
        track = (root[2] - gap * (columns - 1)) / columns
        span = min(columns, 2) if mixed else 1
        width = track * span + gap * (span - 1)
        expected = width - 112 - inner_gap if horizontal else width
        assert abs(control[2] - expected) < 1.1, ('form input width', columns, horizontal, size, mixed, control, expected, root)
        if horizontal:
            assert abs(control[0] - label[0] - 112 - inner_gap) < 1.1, ('form horizontal label', label, control)
        else:
            assert abs(control[0] - label[0]) < 1.1 and control[1] > label[1], ('form vertical label', label, control)
        assert abs(summary[2] - root[2]) < 1.1, ('full-span summary', root, summary)
        assert abs(footer[0] + footer[2] - root[0] - root[2]) < 1.1, ('trailing footer', root, footer)
        assert footer[1] >= summary[1] + summary[3] - 1, ('footer follows items', summary, footer)
        identity()
        cases += 1

    try:
        geometry()
        for theme_button in ('Light', 'Dark'):
            button = mac.find(TITLE, theme_button, 'AXButton')
            if button:
                activate(mac, button)
                time.sleep(.08)
            for _ in range(4):
                for _ in range(3):
                    geometry()
                    toggle('Horizontal form labels')
                    horizontal = not horizontal
                    geometry()
                    toggle('Horizontal form labels')
                    horizontal = not horizontal
                    mac.press(TITLE, f'Form columns: {columns}')
                    columns = 1 + columns % 3
                    mac.release(mac.wait_find(TITLE, f'Form columns: {columns}', 'AXButton'))
                mac.press(TITLE, 'Form size: ' + size)
                size = ['XS', 'S', 'M', 'L'][(['XS', 'S', 'M', 'L'].index(size) + 1) % 4]
                mac.release(mac.wait_find(TITLE, 'Form size: ' + size, 'AXButton'))
        toggle('Mixed form spans')
        mixed = True
        geometry()
        toggle('Reverse form items')
        geometry()
        toggle('Reverse form items')
        toggle('Form validation error')
        mac.wait_text(TITLE, 'A workspace name is required.')
        identity()
        current = mac.wait_find(TITLE, name, 'AXTextField')
        try:
            help_text = mac.text(current, 'AXHelp')
            assert 'Visible to your team.' in help_text and 'A workspace name is required.' in help_text, ('form semantic help/error', help_text)
        finally:
            mac.release(current)
        toggle('Form validation error')
        toggle('Horizontal form labels')
        horizontal = True
        before = rect('Form unlabeled content')
        toggle('Indent absent form labels')
        after = rect('Form unlabeled content')
        assert abs(before[0] - after[0] - 112 - 8) < 1.1, ('absent-label indentation', before, after)
        geometry()
        toggle('Indent absent form labels')
        toggle('Refine form labels')
        identity()
        toggle('Refine form labels')
        toggle('Hide form name')
        absent = mac.find(TITLE, name, 'AXTextField')
        if absent:
            mac.release(absent)
            raise AssertionError('Hidden Form editor remained in accessibility tree')
        toggle('Hide form name')
        # Accessibility removes hidden nodes and may recreate its AX object on
        # reveal. Native editing retention is the buffer and undo history, not
        # CFEqual across removal from the accessibility tree.
        expect_field(mac, TITLE, name, 'a')
        mac.release(editor)
        editor = mac.wait_find(TITLE, name, 'AXTextField')
        focus_gallery_control(mac, name, 'AXTextField')
        mac.key(6, flags=1 << 20)
        expect_field(mac, TITLE, name, 'Northstar')
        mac.key(6, flags=(1 << 20) | (1 << 17))
        expect_field(mac, TITLE, name, 'a')
        identity()
        # Real OS keys activate both arbitrary-content and footer controls.
        for count, label in enumerate(('Form inspect permissions', 'Form save workspace'), 1):
            focus_gallery_control(mac, label, 'AXButton')
            mac.key(49)
            mac.wait_text(TITLE, f'Form actions: {count}')
        focus_gallery_control(mac, 'Form shared access', 'AXCheckBox')
        mac.key(49)
        identity()
        if images:
            reveal_gallery_control(mac, name, 'AXTextField')
            screenshot(mac, images / 'gallery-form.png', title=TITLE)
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        retired = mac.find(TITLE, name, 'AXTextField')
        if retired:
            mac.release(retired)
            raise AssertionError('Retired Form editor remained accessible')
        mac.press(TITLE, 'Text editing')
        expect_field(mac, TITLE, name, 'Northstar')
        replacement = mac.wait_find(TITLE, name, 'AXTextField')
        try:
            assert not equal(editor, replacement), 'Form page retirement retained editor'
        finally:
            mac.release(replacement)
        expect_field(mac, TITLE, name, 'Northstar')
    finally:
        mac.release(editor)
    print(f'GALLERY_FORMS_OK geometry_cases={cases} native_edit=pass native_actions=3 retention=pass retirement=pass', flush=True)
