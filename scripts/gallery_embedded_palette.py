"""Exercise an inline palette beside ordinary native controls."""


def exercise(mac, images):
    from test_gallery import (TITLE, reveal_gallery_control, raise_gallery,
                              expect_field, expect_focus, wait_absent)
    from test_canvas import screenshot

    reveal_gallery_control(mac, 'Continue workspace', 'AXButton')
    raise_gallery(mac)
    mac.wait_text(TITLE, 'Steps added: 0')
    mac.field(TITLE, 'Workspace note', 'AXTextField', 'An editable note')
    mac.press(TITLE, 'Focus command browser')
    expect_focus(mac, 'Workspace commands', 'AXComboBox')
    mac.field(TITLE, 'Workspace commands', 'AXComboBox', 'Add')
    mac.key(36)
    mac.wait_text(TITLE, 'Steps added: 1')
    mac.key(36)
    mac.wait_text(TITLE, 'Steps added: 2')
    expect_field(mac, TITLE, 'Workspace commands', 'Add', 'AXComboBox')
    mac.key(48)  # Tab leaves the embedded query.
    expect_focus(mac, 'Continue workspace')
    mac.key(48, 1 << 17)
    expect_focus(mac, 'Workspace commands', 'AXComboBox')
    mac.key(53)
    expect_field(mac, TITLE, 'Workspace commands', '', 'AXComboBox')
    mac.wait_text(TITLE, 'Browser cancellation requests: 0')
    mac.key(53)
    mac.wait_text(TITLE, 'Browser cancellation requests: 1')
    expect_focus(mac, 'Workspace commands', 'AXComboBox')
    mac.field(TITLE, 'Workspace commands', 'AXComboBox', 'Add')
    mac.press(TITLE, 'Hide command browser')
    wait_absent(mac, 'Workspace commands', 'AXComboBox')
    mac.press(TITLE, 'Show command browser')
    expect_field(mac, TITLE, 'Workspace commands', 'Add', 'AXComboBox')
    if images:
        screenshot(mac, images / 'gallery-palette-embedded.png', title=TITLE)

    # Registry edit actions select the document, never the private search query.
    mac.field(TITLE, 'Workspace note', 'AXTextField', 'An editable note')
    mac.press(TITLE, 'Focus command browser')
    expect_focus(mac, 'Workspace commands', 'AXComboBox')
    mac.field(TITLE, 'Workspace commands', 'AXComboBox', 'Select workspace note')
    mac.key(36)
    expect_focus(mac, 'Workspace note', 'AXTextField')
    mac.key(6)  # z replaces the selected document text.
    expect_field(mac, TITLE, 'Workspace note', 'z')
    expect_field(mac, TITLE, 'Workspace commands', 'Select workspace note', 'AXComboBox')
    mac.key(6, 1 << 20)
    expect_field(mac, TITLE, 'Workspace note', 'An editable note')
    print('GALLERY_PALETTE_EMBEDDED_OK: repeated commands, ordinary Tab, clear/cancel, '
          'hidden query retention and native document selection/undo', flush=True)
