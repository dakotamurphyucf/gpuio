"""Public pie radii/captions preserve source values, selection and ownership."""
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

    def ready(radius, mode):
        mac.wait_text(TITLE, f'Ready: Pie · 4 source values · {radius} · {mode}')
        reveal_gallery_control(mac, 'Chart preview: Pie', 'AXGroup')

    def select():
        focus_gallery_control(mac, 'Chart preview: Pie', 'AXGroup')
        mac.key(115)
        mac.key(36)
        mac.wait_text(TITLE, 'Selected: Reasoning · 44')

    press('Pie')
    ready('Fit', 'Uniform radii')
    select()
    for fixed, variable, label in [
            ('80 px', 'Uniform radii', 'Fixed pie radius 80'),
            ('80 px', 'Per-slice radii', 'Per-slice pie radii'),
            ('Fit', 'Per-slice radii', 'Fixed pie radius 80')]:
        toggle(label)
        ready(fixed, variable)
        mac.wait_text(TITLE, 'Selected: Reasoning · 44')
        select()
        if images:
            screenshot(mac, images / f'gallery-pie-{fixed}-{variable}.png', title=TITLE)
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    initial = 'Dark' if theme else 'Light'
    if theme:
        mac.release(theme)
    other = 'Light' if initial == 'Dark' else 'Dark'
    for label, next_label in [(initial, other), (other, initial)]:
        mac.press(TITLE, label)
        mac.release(mac.wait_find(TITLE, next_label, 'AXButton'))
        ready('Fit', 'Per-slice radii')
        select()
    press('View data')
    mac.release(mac.wait_find(TITLE, 'Chart preview: Pie · original data', 'AXTable'))
    mac.key(115)
    mac.release(mac.wait_find(TITLE, 'Row 1: Reasoning. 44', 'AXRow', contains=True,
                              search_files=True))
    mac.key(119)
    mac.release(mac.wait_find(TITLE, 'Row 4: Other. 10', 'AXRow', contains=True,
                              search_files=True))
    press('Back to chart')
    press('Update chart samples')  # This sample deliberately has unchanged values.
    ready('Fit', 'Per-slice radii')
    mac.wait_text(TITLE, 'Selected: Reasoning · 44')
    toggle('Outside pie labels')
    ready('Fit', 'Per-slice radii')
    toggle('Custom pie captions')
    mac.wait_text(TITLE, 'Agent reasoning')
    mac.wait_text(TITLE, 'Code generation')
    if images:
        screenshot(mac, images / 'gallery-pie-outside-captions.png', title=TITLE)
    select()
    for label, next_label in [(initial, other), (other, initial)]:
        mac.press(TITLE, label)
        mac.release(mac.wait_find(TITLE, next_label, 'AXButton'))
        mac.wait_text(TITLE, 'Agent reasoning')
        mac.wait_text(TITLE, 'Code generation')
        select()
    press('View data')
    mac.release(mac.wait_find(TITLE, 'Chart preview: Pie · original data', 'AXTable'))
    mac.key(115)
    mac.release(mac.wait_find(TITLE, 'Row 1: Reasoning. 44', 'AXRow', contains=True,
                              search_files=True))
    mac.key(119)
    mac.release(mac.wait_find(TITLE, 'Row 4: Other. 10', 'AXRow', contains=True,
                              search_files=True))
    press('Back to chart')
    mac.wait_text(TITLE, 'Agent reasoning')
    toggle('Pie label gap 32')
    mac.wait_text(TITLE, 'Agent reasoning')
    select()
    toggle('Show pie labels')
    wait_absent(mac, 'Agent reasoning', 'AXStaticText')
    mac.wait_text(TITLE, 'Selected: Reasoning · 44')
    toggle('Show pie labels')
    mac.wait_text(TITLE, 'Agent reasoning')
    toggle('Outside pie labels')
    mac.wait_text(TITLE, 'Agent reasoning')
    for control in ['Custom pie captions', 'Pie label gap 32']:
        toggle(control)
    toggle('Per-slice pie radii')
    ready('Fit', 'Uniform radii')
    select()
