"""Public radar projection controls with retained source values and teardown."""
from test_gallery import (TITLE, activate, focus_gallery_control, reveal_gallery_control,
                          wait_absent, expect_enabled, expect_field, expect_focus)
from test_canvas import screenshot


def exercise(mac, images):
    def press(label):
        reveal_gallery_control(mac, label, 'AXButton')
        mac.press(TITLE, label)

    def toggle(label, *, keep_viewport=False):
        if not keep_viewport:
            reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    def ready(scale, radius, gap):
        # Native preparation needs visible layout. A preceding gallery section
        # can leave this plot below the viewport after the family changes.
        reveal_gallery_control(mac, 'Chart preview: Radar', 'AXGroup')
        mac.wait_text(TITLE, f'Ready: Radar · 10 source values · {scale} · {radius} · gap {gap}')

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
    toggle('Custom radar labels')
    reveal_gallery_control(mac, 'Radar axis note', 'AXTextField')
    focus_gallery_control(mac, 'Radar axis note', 'AXTextField')
    mac.key(0, flags=1 << 20)
    mac.key(0)
    expect_field(mac, TITLE, 'Radar axis note', 'a')
    mac.key(51)
    expect_field(mac, TITLE, 'Radar axis note', '')
    mac.key(11)
    expect_field(mac, TITLE, 'Radar axis note', 'b')
    # Keep the editor in the viewport: revealing the update button scrolls
    # the lower label away and legitimately retires its focus. AXPress exercises
    # publication without introducing that unrelated visibility transition.
    mac.press(TITLE, 'Update chart samples')
    expect_focus(mac, 'Radar axis note', 'AXTextField')
    expect_field(mac, TITLE, 'Radar axis note', 'b')
    press('Inspect radar quality')
    mac.wait_text(TITLE, 'Radar label activations: 1')
    focus_gallery_control(mac, 'Inspect radar quality', 'AXButton')
    mac.key(49)
    mac.wait_text(TITLE, 'Radar label activations: 2')
    focus_gallery_control(mac, 'Radar axis note', 'AXTextField')
    # The public theme is Bonsai state above both ordinary label children.
    # Repainting their styles must retain the editor draft/focus and counter.
    for current, alternate in [(initial, other), (other, initial)]:
        mac.press(TITLE, current)
        mac.release(mac.wait_find(TITLE, alternate, 'AXButton'))
        expect_field(mac, TITLE, 'Radar axis note', 'b')
        expect_focus(mac, 'Radar axis note', 'AXTextField')
        expect_enabled(mac, 'Inspect radar quality', True)
        mac.wait_text(TITLE, 'Radar label activations: 2')
        if images:
            screenshot(mac, images / f'gallery-radar-labels-theme-{alternate.lower()}.png', title=TITLE)
    toggle('Disable chart input', keep_viewport=True)
    expect_enabled(mac, 'Inspect radar quality', False)
    expect_enabled(mac, 'Radar axis note', False, role='AXTextField')
    expect_focus(mac, 'Radar axis note', 'AXTextField', focused=False)
    mac.key(0)
    expect_field(mac, TITLE, 'Radar axis note', 'b')
    toggle('Disable chart input', keep_viewport=True)
    expect_enabled(mac, 'Inspect radar quality', True)
    toggle('Show radar labels', keep_viewport=True)
    wait_absent(mac, 'Inspect radar quality', 'AXButton')
    wait_absent(mac, 'Radar axis note', 'AXTextField')
    toggle('Show radar labels', keep_viewport=True)
    expect_field(mac, TITLE, 'Radar axis note', 'b')
    press('Inspect radar quality')
    mac.wait_text(TITLE, 'Radar label activations: 3')
    press('View data')
    wait_absent(mac, 'Inspect radar quality', 'AXButton')
    wait_absent(mac, 'Radar axis note', 'AXTextField')
    press('Back to chart')
    reveal_gallery_control(mac, 'Chart preview: Radar', 'AXGroup')
    expect_field(mac, TITLE, 'Radar axis note', 'b')
    press('Inspect radar quality')
    mac.wait_text(TITLE, 'Radar label activations: 4')
    if images:
        screenshot(mac, images / 'gallery-radar-custom-labels.png', title=TITLE)
    toggle('Custom radar labels', keep_viewport=True)
    wait_absent(mac, 'Inspect radar quality', 'AXButton')
    wait_absent(mac, 'Radar axis note', 'AXTextField')
    toggle('Custom radar labels', keep_viewport=True)
    expect_field(mac, TITLE, 'Radar axis note', '')
    mac.wait_text(TITLE, 'Radar label activations: 4')
    toggle('Custom radar labels', keep_viewport=True)
    wait_absent(mac, 'Radar axis note', 'AXTextField')
    print('GALLERY_RADAR_OK: shared/explicit/per-axis scale, radius, gap, native selection and original values', flush=True)
    print('GALLERY_RADAR_LABELS_OK: ordinary OCaml Views, click/keyboard effects, theme focus/draft retention, ancestor disable, hide/browser retention and unmount', flush=True)
