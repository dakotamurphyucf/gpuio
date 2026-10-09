"""Installed numeric-frame input, retained editing and OCaml step requests."""
import ctypes as C
import json
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, element_rect, expect_enabled, expect_field,
        focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
        wait_absent,
    )
    from test_canvas import screenshot

    label = 'Preview quantity'
    increase, decrease = label + ' increase', label + ' decrease'
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    cases = []

    def setting(name, expected):
        # Settings use AX actions; physical editing targets are revealed below.
        node = mac.wait_find(TITLE, name, 'AXCheckBox')
        raw = mac.attr(node, 'AXValue')
        try:
            assert raw
            if bool(boolean(raw)) != expected:
                activate(mac, mac.retain(node))
        finally:
            if raw:
                mac.release(raw)
            mac.release(node)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, name, 'AXCheckBox')
            raw = mac.attr(node, 'AXValue')
            try:
                if raw and bool(boolean(raw)) == expected:
                    return
            finally:
                if raw:
                    mac.release(raw)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError(('numeric setting did not settle', name, expected))

    def focus():
        reveal_gallery_control(mac, label, 'AXTextField')
        focus_gallery_control(mac, label, 'AXTextField')

    def draft(text, committed=None):
        expect_field(mac, TITLE, label, text)
        if committed is not None:
            mac.wait_text(TITLE, f'Committed quantity: {committed:g}')

    def replace(text):
        focus()
        mac.field(TITLE, label, 'AXTextField', text)
        draft(text)

    def layout(name):
        selected = mac.find(TITLE, '✓ ' + name, 'AXButton')
        if selected:
            mac.release(selected)
        else:
            mac.press(TITLE, name)
            mac.release(mac.wait_find(TITLE, '✓ ' + name, 'AXButton'))

    def prefix_count():
        def visit(node):
            count = int(mac.text(node, 'AXRole') == 'AXButton'
                        and 'Qty' in (mac.text(node, 'AXTitle'), mac.text(node, 'AXDescription')))
            children = mac.children(node)
            try:
                return count + sum(visit(child) for child in children)
            finally:
                for child in children:
                    mac.release(child)
        window = mac.window(TITLE)
        try:
            return visit(window)
        finally:
            mac.release(window)

    def geometry(name):
        editor = mac.wait_find(TITLE, label, 'AXTextField')
        try:
            assert equal(original, editor), 'layout replaced numeric editor'
            e = element_rect(mac, editor)
        finally:
            mac.release(editor)
        if name == 'Keyboard only':
            wait_absent(mac, increase, 'AXButton')
            wait_absent(mac, decrease, 'AXButton')
            return {'editor': e}
        rectangles = []
        for button in (decrease, increase):
            node = mac.wait_find(TITLE, button, 'AXButton')
            try:
                rectangles.append(element_rect(mac, node))
            finally:
                mac.release(node)
        down, up = rectangles
        if name == 'Side controls':
            assert down[0] + down[2] <= e[0] + 1 and up[0] >= e[0] + e[2] - 1, (name, e, rectangles)
        else:
            assert up[0] >= e[0] + e[2] - 1 and abs(down[0] - up[0]) <= 1, (name, e, rectangles)
            assert up[1] + up[3] <= down[1] + 1, (name, rectangles)
        return {'editor': e, 'decrease': down, 'increase': up}

    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Numbers & codes')
    for name, expected in (
        ('Read-only numeric inputs', False), ('Disable quantity', False),
        ('Allow an empty quantity', False), ('Choose step size in OCaml', False),
        ('Style the complete numeric control', True), ('Custom step symbols', False),
    ):
        setting(name, expected)
    layout('Side controls')
    original = mac.wait_find(TITLE, label, 'AXTextField')
    try:
        draft('1e-', 12)
        focus()
        mac.key(36)
        draft('1e-', 12)
        mac.press(TITLE, 'Commit quantity')
        mac.wait_text(TITLE, 'Finish the number before committing')
        draft('1e-', 12)
        mac.press(TITLE, 'Restore quantity')
        draft('12', 12)
        replace('19.13')
        mac.key(36)
        draft('19.25', 19.25)
        # Make one real typed replacement with a known undo predecessor.
        focus()
        mac.key(0, flags=1 << 20)
        mac.key(26)  # 7
        draft('7', 19.25)
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for framed in (False, True):
                setting('Style the complete numeric control', framed)
                for symbols in (False, True):
                    setting('Custom step symbols', symbols)
                    for mode in ('Side controls', 'Stacked controls', 'Keyboard only'):
                        layout(mode)
                        reveal_gallery_control(mac, label, 'AXTextField')
                        case = f'{theme}-{"frame" if framed else "plain"}-{"custom" if symbols else "default"}-{mode.replace(" ", "-")}'
                        result = geometry(mode)
                        assert prefix_count() == int(framed), ('duplicate/missing numeric prefix', case)
                        for glyph in ('↓', '↑'):
                            wait_absent(mac, glyph, 'AXStaticText')
                        draft('7', 19.25)
                        cases.append({'case': case, **result})
                        if images:
                            screenshot(mac, images / (case + '.png'), title=TITLE)
        mac.press(TITLE, 'Undo quantity')
        draft('19.25', 19.25)
        mac.press(TITLE, 'Redo quantity')
        draft('7', 19.25)
        mac.press(TITLE, 'Qty')
        mac.wait_text(TITLE, 'The prefix action keeps the numeric draft and committed value')
        draft('7', 19.25)
        mac.press(TITLE, 'Restore quantity')
        draft('19.25', 19.25)
        setting('Choose step size in OCaml', True)
        for current, expected in (('9.75', '10'), ('10', '11'), ('49', '50'), ('50', '55')):
            replace(current)
            mac.key(126)
            draft(expected, float(expected))
            mac.wait_text(TITLE, 'Applied a step selected by the application')
        incrementor = mac.wait_find(TITLE, label, 'AXIncrementor')
        try:
            mac.perform(incrementor, 'AXDecrement')
        finally:
            mac.release(incrementor)
        draft('50', 50)
        layout('Stacked controls')
        mac.press(TITLE, increase)
        draft('55', 55)
        setting('Choose step size in OCaml', False)
        focus()
        mac.key(125)
        draft('54.75', 54.75)
        setting('Read-only numeric inputs', True)
        focus()
        mac.key(126)
        mac.key(0, flags=1 << 20)
        mac.key(51)
        draft('54.75', 54.75)
        expect_enabled(mac, 'Qty', True)
        mac.press(TITLE, 'Qty')
        mac.wait_text(TITLE, 'The prefix action keeps the numeric draft and committed value')
        setting('Read-only numeric inputs', False)
        setting('Disable quantity', True)
        expect_enabled(mac, label, False, role='AXTextField')
        expect_enabled(mac, 'Qty', False)
        setting('Disable quantity', False)
        replace('')
        mac.press(TITLE, 'Commit quantity')
        mac.wait_text(TITLE, 'Enter a number, or allow an empty value')
        draft('', 54.75)
        setting('Allow an empty quantity', True)
        mac.press(TITLE, 'Commit quantity')
        mac.wait_text(TITLE, 'No quantity committed')
        draft('')
        replace('12')
        mac.press(TITLE, 'Commit quantity')
        draft('12', 12)
        setting('Allow an empty quantity', False)
        setting('Custom step symbols', False)
        layout('Side controls')
        geometry('Side controls')
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, label, 'AXTextField')
        mac.press(TITLE, 'Numbers & codes')
        current = mac.wait_find(TITLE, label, 'AXTextField')
        try:
            assert not equal(original, current), 'page retirement reused native editor'
        finally:
            mac.release(current)
        draft('1e-', 12)
        mac.press(TITLE, 'Restore quantity')
        focus()
        mac.key(126)
        draft('12.25', 12.25)
        print(f'GALLERY_NUMBERS_OK: {len(cases)} presentation/identity cases, '
              'native draft/commit and retained undo/redo, OCaml adaptive steps '
              'from keys/AX/button, independent prefix, policy/empty handling/remount', flush=True)
    finally:
        mac.release(original)
        if images:
            (images / 'number-layout-samples.json').write_text(json.dumps(cases, indent=2) + '\n')
