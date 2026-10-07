"""Mark appearance preserves original selection across styles and projections."""
from test_gallery import (TITLE, activate, focus_gallery_control,
                          reveal_gallery_control)
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

    patterns = ['Slash pattern', 'Checkerboard pattern']
    paths = ['Styled paths', 'Styled markers'] + patterns
    bars = ['Base-to-tip bars', 'Domain-colored bars', 'Value-colored bars',
            'Uniform aggregate colors'] + patterns
    for family, count, value, presets in [
            ('Line', 48, 'Atlas · x 0 · value 30', paths),
            ('Area', 24, 'Active capacity · x 0 · value 30', paths),
            ('Bar', 24, 'Completed evaluations · x 0 · value 30', bars),
            ('Categorical', 8, 'Completed · Research (category 42) · value 30', paths + bars),
            ('Stacked bars', 15, 'Completed · Mon (category 42) · value 30', bars),
            ('Stacked areas', 15, 'Completed · Mon (category 42) · value 30', paths),
            ('Radar', 10, 'Atlas · Quality · 88 / 100', paths),
            ('Mixed layers', 72, 'Capacity · x 0 · value 30', paths + bars)]:
        press(family)
        # Preparation needs a measured viewport; Radar's controls can push its
        # chart entirely below the fold after returning to the family buttons.
        reveal_gallery_control(mac, 'Chart preview: ' + family, 'AXGroup')
        mac.wait_text(TITLE, f'Ready: {family} · {count} source values')
        select(family, value)
        selected = value
        for preset in presets:
            press(preset)
            mac.wait_text(TITLE, 'Default inspection · ' + preset)
            # Changing presentation retains the last committed observation.
            # A new Home/Enter below selects under the new sampling policy.
            mac.wait_text(TITLE, 'Selected: ' + selected)
            selected = value
            if preset == 'Uniform aggregate colors':
                selected = {
                    'Bar': 'Completed evaluations · 12 samples selected',
                    'Categorical': 'Completed · 2 categories selected',
                    'Stacked bars': 'Completed · 2 categories selected',
                }.get(family, value)
            select(family, selected)
            if images:
                reveal_gallery_control(mac, 'Chart preview: ' + family, 'AXGroup')
                screenshot(mac, images / ('gallery-marks-' + family.lower().replace(' ', '-')
                                         + '-' + preset.lower().replace(' ', '-') + '.png'), title=TITLE)
        press('Default marks')
        mac.wait_text(TITLE, f'Ready: {family} · {count} source values')
        select(family, value)
        if family in ('Bar', 'Stacked bars'):
            for preset in ['Base-to-tip bars'] + patterns:
                press(preset)
                for control, direction in [
                        ('Horizontal axes', 'Horizontal'),
                        ('Reverse value axis', 'Horizontal reversed'),
                        ('Horizontal axes', 'Vertical reversed'),
                        ('Reverse value axis', 'Vertical')]:
                    toggle(control)
                    mac.wait_text(TITLE, f'Ready: {family} · {count} source values · {direction}')
                    select(family, value)
        press('View data')
        mac.release(mac.wait_find(TITLE, f'Chart preview: {family} · original data', 'AXTable'))
        mac.key(119)
        mac.release(mac.wait_find(TITLE, f'Row {count}:', 'AXRow', contains=True, search_files=True))
        press('Back to chart')
        select(family, value)
        press('Default marks')
        mac.wait_text(TITLE, f'Ready: {family} · {count} source values')
    press('Line')
    press('Styled markers')
    mac.wait_text(TITLE, 'Default inspection · Styled markers')
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    initial = 'Dark' if theme else 'Light'
    if theme:
        mac.release(theme)
    other = 'Light' if initial == 'Dark' else 'Dark'
    for label, next_label in [(initial, other), (other, initial)]:
        mac.press(TITLE, label)
        mac.release(mac.wait_find(TITLE, next_label, 'AXButton'))
        select('Line', 'Atlas · x 0 · value 30')
        for family, value in [('Area', 'Active capacity · x 0 · value 30'),
                              ('Bar', 'Completed evaluations · x 0 · value 30')]:
            press(family)
            for preset in patterns:
                press(preset)
                select(family, value)
                if images:
                    reveal_gallery_control(mac, 'Chart preview: ' + family, 'AXGroup')
                    screenshot(mac, images / ('gallery-pattern-theme-' + next_label.lower()
                                             + '-' + family.lower() + '-'
                                             + preset.lower().replace(' ', '-') + '.png'), title=TITLE)
        press('Line')
        press('Styled markers')
        select('Line', 'Atlas · x 0 · value 30')
    press('Update chart samples')
    mac.wait_text(TITLE, 'Selected: Atlas · x 0 · value 33')
    select('Line', 'Atlas · x 0 · value 33')
    press('Default marks')
