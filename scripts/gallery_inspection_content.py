"""Public rich chart inspection: ordinary OCaml rows, effects and native editor."""
from test_gallery import (TITLE, activate, focus_gallery_control,
                          reveal_gallery_control, wait_absent, expect_enabled,
                          expect_field, expect_focus)
from test_canvas import screenshot


def exercise(mac, images):
    def press(label):
        reveal_gallery_control(mac, label, 'AXButton')
        mac.press(TITLE, label)

    def preview():
        reveal_gallery_control(mac, 'Chart preview: Pie', 'AXGroup')
        focus_gallery_control(mac, 'Chart preview: Pie', 'AXGroup')
        mac.key(115)  # Home previews stable slice ID 1 without committing it.

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    press('Pie')
    mac.wait_text(TITLE, 'Ready: Pie · 4 source values · Fit · Uniform radii')
    press('Inspection rows')
    preview()
    mac.wait_text(TITLE, 'Weight')
    mac.wait_text(TITLE, '44.0%')
    wait_absent(mac, 'Inspection note', 'AXTextField')
    if images:
        screenshot(mac, images / 'gallery-inspection-rows.png', title=TITLE)

    press('Interactive inspection')
    preview()
    mac.release(mac.wait_find(TITLE, 'Inspect chart reasoning', 'AXButton'))
    mac.key(48)  # Tab enters uncommitted inspection content.
    expect_focus(mac, 'Inspect chart reasoning', 'AXButton')
    mac.key(49)
    mac.wait_text(TITLE, 'Inspection activations: 1')
    focus_gallery_control(mac, 'Inspection note', 'AXTextField')
    mac.key(0)
    expect_field(mac, TITLE, 'Inspection note', 'a')
    mac.key(51)
    expect_field(mac, TITLE, 'Inspection note', '')
    mac.key(11)
    expect_field(mac, TITLE, 'Inspection note', 'b')

    # AXPress avoids scrolling the editor outside the viewport. The ordinary
    # palette buttons update Bonsai while the native draft remains focused.
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    initial = 'Dark' if theme else 'Light'
    if theme:
        mac.release(theme)
    other = 'Light' if initial == 'Dark' else 'Dark'
    for current, alternate in [(initial, other), (other, initial)]:
        mac.press(TITLE, current)
        mac.release(mac.wait_find(TITLE, alternate, 'AXButton'))
        expect_field(mac, TITLE, 'Inspection note', 'b')
        expect_focus(mac, 'Inspection note', 'AXTextField')
        if images:
            screenshot(mac, images / f'gallery-inspection-card-{alternate.lower()}.png', title=TITLE)

    mac.press(TITLE, 'Update chart samples')
    expect_field(mac, TITLE, 'Inspection note', 'b')
    expect_focus(mac, 'Inspection note', 'AXTextField')
    for current, alternate in [('Comfortable', 'Large'), ('Large', 'Compact'),
                               ('Compact', 'Comfortable')]:
        mac.press(TITLE, current)
        mac.release(mac.wait_find(TITLE, alternate, 'AXButton'))
        # Resizing can legitimately clip a focused child in the scroll viewport.
        # Reveal the retained field before asserting its native draft.
        preview()
        focus_gallery_control(mac, 'Inspection note', 'AXTextField')
        expect_field(mac, TITLE, 'Inspection note', 'b')
    for mode in ['Inspection overlay', 'Interactive inspection']:
        mac.press(TITLE, mode)
        expect_field(mac, TITLE, 'Inspection note', 'b')
        expect_focus(mac, 'Inspection note', 'AXTextField')
        if images:
            screenshot(mac, images / f'gallery-{mode.lower().replace(" ", "-")}.png', title=TITLE)
    press('Inspect chart reasoning')
    mac.wait_text(TITLE, 'Inspection activations: 2')

    focus_gallery_control(mac, 'Inspection note', 'AXTextField')
    toggle('Disable chart input')
    # Disabling the chart retires inspection itself, unlike always-present
    # radar labels. Its children must leave accessibility and keyboard input.
    wait_absent(mac, 'Inspection note', 'AXTextField')
    wait_absent(mac, 'Inspect chart reasoning', 'AXButton')
    mac.key(0)
    toggle('Disable chart input')
    preview()
    expect_enabled(mac, 'Inspection note', True, role='AXTextField')
    expect_field(mac, TITLE, 'Inspection note', 'b')

    press('View data')
    wait_absent(mac, 'Inspection note', 'AXTextField')
    mac.release(mac.wait_find(TITLE, 'Chart preview: Pie · original data', 'AXTable'))
    press('Back to chart')
    preview()
    expect_field(mac, TITLE, 'Inspection note', 'b')
    press('Native inspection')
    wait_absent(mac, 'Inspection note', 'AXTextField')
    press('Interactive inspection')
    preview()
    expect_field(mac, TITLE, 'Inspection note', '')
    mac.wait_text(TITLE, 'Inspection activations: 2')
    press('Inspection overlay')
    preview()
    focus_gallery_control(mac, 'Inspection note', 'AXTextField')
    mac.key(2)
    expect_field(mac, TITLE, 'Inspection note', 'd')
    focus_gallery_control(mac, 'Inspect chart reasoning', 'AXButton')
    mac.key(49)
    mac.wait_text(TITLE, 'Inspection activations: 3')
    mac.press(TITLE, 'Presentation')
    wait_absent(mac, 'Inspection note', 'AXTextField')
    mac.press(TITLE, 'Charts & data')
    preview()
    expect_field(mac, TITLE, 'Inspection note', '')
    mac.wait_text(TITLE, 'Inspection activations: 3')
    press('Native inspection')
    print('GALLERY_INSPECTION_CONTENT_OK: structured rows, uncommitted keyboard entry, '
          'native editing/actions, theme/scale and Card/Overlay identity, disabled ancestor, '
          'browser retention, removal/page draft reset and retained Bonsai count', flush=True)
