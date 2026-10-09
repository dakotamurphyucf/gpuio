"""Installed segmented OTP retains one editor through presentation updates."""
import ctypes as C
import json
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, element_rect, expect_field, expect_focus,
        focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
        wait_absent,
    )
    from test_canvas import screenshot

    label = 'Preview verification code'
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    cases = []

    def setting(name, expected):
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
        raise AssertionError(('OTP setting', name, expected))

    def expect(text):
        expect_field(mac, TITLE, label, text)

    def focus():
        reveal_gallery_control(mac, label, 'AXTextField')
        focus_gallery_control(mac, label, 'AXTextField')

    def field(masked=False):
        node = mac.wait_find(TITLE, label, 'AXTextField')
        try:
            assert equal(original, node), 'OTP presentation replaced editor'
            if masked:
                assert mac.text(node, 'AXSubrole') == 'AXSecureTextField'
                text = mac.text(node, 'AXValue')
                assert text is None or all(char == '•' for char in text), ('masked OTP disclosure', text)
            return element_rect(mac, node)
        finally:
            mac.release(node)

    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Numbers & codes')
    setting('Read-only numeric inputs', False)
    setting('Mask verification code', False)
    setting('Group verification cells', True)
    setting('Larger verification cells', False)
    original = mac.wait_find(TITLE, label, 'AXTextField')
    try:
        expect('')
        focus()
        for code in (18, 19, 20, 21, 23, 22):
            mac.key(code)  # Physical US-layout keys 123456.
        expect('123456')
        mac.wait_text(TITLE, 'Code complete')
        mac.key(123)
        mac.key(51)
        expect('12346')
        mac.wait_text(TITLE, 'Waiting for six digits')
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for grouped in (False, True):
                setting('Group verification cells', grouped)
                for large in (False, True):
                    setting('Larger verification cells', large)
                    for masked in (False, True):
                        setting('Mask verification code', masked)
                        reveal_gallery_control(mac, label, 'AXTextField')
                        bounds = field(masked)
                        if not masked:
                            expect('12346')
                        cases.append({'theme': theme, 'grouped': grouped, 'large': large,
                                      'masked': masked, 'bounds': bounds})
                        if images and not masked:
                            screenshot(mac, images / f'otp-{theme}-{grouped}-{large}.png', title=TITLE)
        setting('Mask verification code', False)
        focus()
        mac.key(6, flags=1 << 20)
        expect('123456')
        mac.key(6, flags=(1 << 20) | (1 << 17))
        expect('12346')
        mac.field(TITLE, label, 'AXTextField', '１２３４５６')
        expect('123456')
        mac.wait_text(TITLE, 'Code complete')
        mac.field(TITLE, label, 'AXTextField', '12x')
        expect('123456')
        setting('Read-only numeric inputs', True)
        focus()
        mac.key(0, flags=1 << 20)
        mac.key(51)
        expect('123456')
        expect_focus(mac, label, 'AXTextField')
        field()
        setting('Read-only numeric inputs', False)
        setting('Larger verification cells', False)
        setting('Group verification cells', True)
        field()
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, label, 'AXTextField')
        mac.press(TITLE, 'Numbers & codes')
        node = mac.wait_find(TITLE, label, 'AXTextField')
        try:
            assert not equal(original, node), 'OTP page retirement reused editor'
        finally:
            mac.release(node)
        expect('')
        focus()
        mac.key(18)
        expect('1')
        print(f'GALLERY_OTP_OK: {len(cases)} retained presentation cases, physical '
              'typing/delete/history, completion, normalization/rejection, masking, '
              'read-only focus/edit fence and fresh page remount', flush=True)
    finally:
        mac.release(original)
        if images:
            (images / 'otp-presentation-samples.json').write_text(json.dumps(cases, indent=2) + '\n')
