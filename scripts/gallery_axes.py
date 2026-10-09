"""Public axis presets keep source values and selection separate from presentation."""
from test_gallery import (TITLE, activate, focus_gallery_control,
                          reveal_gallery_control, wait_absent)
from test_canvas import screenshot


def exercise(mac, images):
    def press(label):
        reveal_gallery_control(mac, label, 'AXButton')
        mac.press(TITLE, label)

    def toggle(label):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    def select(family, value):
        focus_gallery_control(mac, 'Chart preview: ' + family, 'AXGroup')
        mac.key(115)
        mac.key(36)
        mac.wait_text(TITLE, 'Selected: ' + value)

    for family, count, value in [
            ('Line', 48, 'Atlas · x 0 · value 30'),
            ('Categorical', 8, 'Completed · Research (category 42) · value 30'),
            ('Candlestick', 24, 'Session 1 · close 34')]:
        press(family)
        mac.wait_text(TITLE, f'Ready: {family} · {count} source values')
        select(family, value)
        for preset in ['Styled axes', 'Floating axes', 'Axis labels only', 'Axis lines only']:
            press(preset)
            mac.wait_text(TITLE, 'Default inspection · ' + preset)
            reveal_gallery_control(mac, 'Chart preview: ' + family, 'AXGroup')
            if preset == 'Axis lines only':
                wait_absent(mac, 'Physical midpoint', 'AXStaticText')
            else:
                mac.wait_text(TITLE, 'Physical midpoint')
            mac.wait_text(TITLE, 'Selected: ' + value)
            select(family, value)
        press('Floating axes')
        mac.wait_text(TITLE, 'Default inspection · Floating axes')
        for control in ['Horizontal axes', 'Reverse value axis',
                        'Horizontal axes', 'Reverse value axis']:
            toggle(control)
            select(family, value)
            mac.wait_text(TITLE, 'Physical midpoint')
        if images:
            screenshot(mac, images / f'gallery-axes-{family.lower()}.png', title=TITLE)
        press('View data')
        mac.release(mac.wait_find(TITLE, f'Chart preview: {family} · original data', 'AXTable'))
        mac.key(119)
        mac.release(mac.wait_find(TITLE, f'Row {count}:', 'AXRow', contains=True, search_files=True))
        press('Back to chart')
        mac.wait_text(TITLE, 'Physical midpoint')
        press('Default axes')
        wait_absent(mac, 'Physical midpoint', 'AXStaticText')
    press('Line')
    press('Styled axes')
    mac.wait_text(TITLE, 'Default inspection · Styled axes')
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    initial = 'Dark' if theme else 'Light'
    if theme:
        mac.release(theme)
    other = 'Light' if initial == 'Dark' else 'Dark'
    for label, next_label in [(initial, other), (other, initial)]:
        mac.press(TITLE, label)
        mac.release(mac.wait_find(TITLE, next_label, 'AXButton'))
        mac.wait_text(TITLE, 'Physical midpoint')
        select('Line', 'Atlas · x 0 · value 30')
    press('Update chart samples')
    mac.wait_text(TITLE, 'Selected: Atlas · x 0 · value 33')
    mac.wait_text(TITLE, 'Physical midpoint')
    press('Default axes')
    wait_absent(mac, 'Physical midpoint', 'AXStaticText')
