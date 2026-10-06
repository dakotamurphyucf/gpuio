"""Public radar projection controls with retained source values and teardown."""
from test_gallery import TITLE, activate, focus_gallery_control, reveal_gallery_control
from test_canvas import screenshot


def exercise(mac, images):
    def press(label):
        reveal_gallery_control(mac, label, 'AXButton')
        mac.press(TITLE, label)

    def toggle(label):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    def ready(scale, radius, gap):
        mac.wait_text(TITLE, f'Ready: Radar · 10 source values · {scale} · {radius} · gap {gap}')
        reveal_gallery_control(mac, 'Chart preview: Radar', 'AXGroup')

    def select():
        focus_gallery_control(mac, 'Chart preview: Radar', 'AXGroup')
        mac.key(115)
        mac.key(36)
        mac.wait_text(TITLE, 'Selected: Atlas · Quality · 88 / 100')

    press('Radar')
    ready('Per-axis maxima', 'Fit', 0)
    select()
    toggle('Fixed radar radius 80')
    ready('Per-axis maxima', '80 px', 0)
    mac.wait_text(TITLE, 'Selected: Atlas · Quality · 88 / 100')
    press('Shared data maximum')
    ready('Shared data maximum', '80 px', 0)
    mac.wait_text(TITLE, 'Selected: Atlas · Quality · 88 / 100')
    select()
    toggle('Radar label gap 24')
    ready('Shared data maximum', '80 px', 24)
    press('Shared maximum 50')
    ready('Shared maximum 50', '80 px', 24)
    mac.wait_text(TITLE, 'Selected: Atlas · Quality · 88 / 100')
    select()
    if images:
        screenshot(mac, images / 'gallery-radar-shared-50.png', title=TITLE)
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    initial = 'Dark' if theme else 'Light'
    if theme:
        mac.release(theme)
    other = 'Light' if initial == 'Dark' else 'Dark'
    mac.press(TITLE, initial)  # Theme toggle lives in the fixed header.
    mac.release(mac.wait_find(TITLE, other, 'AXButton'))
    select()
    if images:
        screenshot(mac, images / 'gallery-radar-other-theme.png', title=TITLE)
    mac.press(TITLE, other)
    mac.release(mac.wait_find(TITLE, initial, 'AXButton'))
    select()
    press('View data')
    mac.release(mac.wait_find(TITLE, 'Chart preview: Radar · original data', 'AXTable'))
    mac.key(119)
    mac.release(mac.wait_find(TITLE, 'Row 10: Nova · Reliability. 86 / 100',
                              'AXRow', contains=True, search_files=True))
    mac.wait_text(TITLE, 'Selected: Atlas · Quality · 88 / 100')
    press('Back to chart')
    focus_gallery_control(mac, 'Chart preview: Radar', 'AXGroup')
    mac.key(119)
    mac.key(36)
    mac.wait_text(TITLE, 'Selected: Nova · Reliability · 86 / 100')
    press('Update chart samples')
    mac.wait_text(TITLE, 'Selected: Nova · Reliability · 86 / 100')
    press('Per-axis maxima')
    ready('Per-axis maxima', '80 px', 24)
    toggle('Fixed radar radius 80')
    ready('Per-axis maxima', 'Fit', 24)
    toggle('Radar label gap 24')
    ready('Per-axis maxima', 'Fit', 0)
    select()
    print('GALLERY_RADAR_OK: shared/explicit/per-axis scale, radius, gap, native selection and original values', flush=True)
