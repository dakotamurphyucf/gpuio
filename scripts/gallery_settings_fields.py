"""Public Settings field retention and native editing acceptance."""
import ctypes as C
import time


def exercise(mac):
    from test_gallery import (
        TITLE, activate, element_rect, expect_enabled, expect_field,
        focus_gallery_control,
    )

    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]

    def checked(label, expected):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, label, 'AXCheckBox')
            value = mac.attr(node, 'AXValue')
            try:
                if value and bool(boolean(value)) == expected:
                    return
            finally:
                if value:
                    mac.release(value)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError((label, 'checked', expected))

    def navigate(group):
        activate(mac, mac.wait_find(TITLE, group, 'AXLink'))

    def replace(label, codes):
        focus_gallery_control(mac, label, 'AXTextField')
        mac.key(0, flags=1 << 20)
        for code in codes:
            mac.key(code)

    def choose_last(label):
        focus_gallery_control(mac, label, 'AXPopUpButton')
        mac.key(49)
        mac.key(119)
        mac.key(36)

    def saved():
        node = mac.wait_find(
            TITLE, 'Stored name: a · region: Asia Pacific · model: 249 · custom: 0',
            'AXStaticText')
        mac.release(node)

    name, budget = 'Settings workspace name', 'Settings response budget'
    notifications, reports = 'Settings notifications', 'Settings summaries'
    mac.press(TITLE, 'Settings')
    replace(name, [0])
    expect_field(mac, TITLE, name, 'a')
    for label, expected in [(notifications, False), (reports, True)]:
        focus_gallery_control(mac, label, 'AXCheckBox')
        mac.key(49)
        checked(label, expected)
    navigate('Generation')
    replace(budget, [18, 29, 29, 18])  # 1001: native commit clamps to the domain.
    expect_field(mac, TITLE, budget, '1001')
    mac.key(36)
    expect_field(mac, TITLE, budget, '1000')
    replace(budget, [27, 18])  # -1: lower bound, independent of the upper case.
    mac.key(36)
    expect_field(mac, TITLE, budget, '0')
    replace(budget, [21, 19])  # 42
    mac.key(36)
    expect_field(mac, TITLE, budget, '42')
    choose_last('Settings region')
    choose_last('Settings model')
    saved()

    variants = ['Outline groups', 'Filled groups', 'Plain groups', 'Card groups']
    sizes = ['Medium fields', 'Large fields', 'Small fields']
    variant, size, narrow, light = 0, 0, False, False

    def refine(button, next_button):
        mac.press(TITLE, button)
        mac.release(mac.wait_find(TITLE, next_button, 'AXButton'))

    # Every variant/size pair, alternating width and theme. This is 24 group
    # cases, not an assertion of the full four-dimensional Cartesian product.
    for group, fields in [
        ('Make it yours', [(name, 'AXTextField'), (notifications, 'AXCheckBox'),
                          (reports, 'AXCheckBox')]),
        ('Generation', [(budget, 'AXTextField'), ('Settings region', 'AXPopUpButton'),
                        ('Settings model', 'AXPopUpButton')]),
    ]:
        navigate(group)
        originals = [(label, role, mac.wait_find(TITLE, label, role))
                     for label, role in fields]
        try:
            for case in range(12):
                refine(sizes[size], sizes[(size + 1) % 3])
                size = (size + 1) % 3
                if case % 3 == 0:
                    refine(variants[variant], variants[(variant + 1) % 4])
                    variant = (variant + 1) % 4
                refine('Widen settings' if narrow else 'Narrow settings',
                       'Narrow settings' if narrow else 'Widen settings')
                narrow = not narrow
                refine('Light' if light else 'Dark', 'Dark' if light else 'Light')
                light = not light
                for label, role, original in originals:
                    current = mac.wait_find(TITLE, label, role)
                    try:
                        assert equal(original, current), (group, case, label, 'replaced')
                        _, _, w, h = element_rect(mac, current)
                        assert w > 0 and h > 0, (group, case, label, w, h)
                    finally:
                        mac.release(current)
                if group == 'Make it yours':
                    expect_field(mac, TITLE, name, 'a')
                    checked(notifications, False)
                    checked(reports, True)
                else:
                    expect_field(mac, TITLE, budget, '42')
                saved()
        finally:
            for _, _, original in originals:
                mac.release(original)

    # These are application-owned values; destroying every field placement must
    # not reset them. Revisit all field families, then exercise explicit resets.
    navigate('Advanced')
    mac.release(mac.wait_find(TITLE, 'Experiment 00', 'AXStaticText'))
    navigate('Workspace')
    navigate('Make it yours')
    expect_field(mac, TITLE, name, 'a')
    checked(notifications, False)
    checked(reports, True)
    navigate('Generation')
    expect_field(mac, TITLE, budget, '42')
    saved()
    for field in ['budget', 'region', 'model']:
        expect_enabled(mac, 'Reset ' + field, True)
    mac.press(TITLE, 'Reset entire page')
    expect_field(mac, TITLE, budget, '25')
    for field in ['budget', 'region', 'model']:
        expect_enabled(mac, 'Reset ' + field, False)
    navigate('Make it yours')
    expect_field(mac, TITLE, name, 'Northstar')
    checked(notifications, True)
    checked(reports, False)
    mac.release(mac.wait_find(
        TITLE, 'Stored name: Northstar · region: Americas · model: 000 · custom: 0',
        'AXStaticText'))
    print('GALLERY_SETTINGS_FIELDS_OK: six field controls, 24 variant/size/layout/theme '
          'cases, retained identity/data, native Boolean/dropdown edits, numeric '
          'commit bounds, page remount and explicit whole-page reset', flush=True)
