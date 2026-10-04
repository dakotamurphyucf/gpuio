"""Physical accordion state, native draft ownership and keyboard checks."""


def exercise(mac):
    from test_gallery import (
        TITLE, expect_enabled, expect_field, expect_popup_expanded,
        focus_gallery_control, reveal_gallery_control, wait_absent,
    )

    field = 'Disclosure notes'
    initial = 'Write something here, then close and reopen this section.'
    draft = 'A retained disclosure draft 🪴'
    reveal_gallery_control(mac, 'Identity', 'AXButton')
    expect_popup_expanded(mac, 'Identity', True)
    expect_field(mac, TITLE, field, initial, role='AXTextArea')
    mac.field(TITLE, field, 'AXTextArea', draft)
    focus_gallery_control(mac, field, 'AXTextArea')
    mac.key(0, flags=1 << 20)  # Command+A, then a native keyboard edit.
    mac.key(0)
    expect_field(mac, TITLE, field, 'a', role='AXTextArea')
    mac.press(TITLE, 'Identity')
    expect_popup_expanded(mac, 'Identity', False)
    wait_absent(mac, field, 'AXTextArea')
    mac.press(TITLE, 'Identity')
    expect_popup_expanded(mac, 'Identity', True)
    # AX removes hidden nodes and may recreate its object. The native buffer
    # and undo history establish retention across that accessibility removal.
    expect_field(mac, TITLE, field, 'a', role='AXTextArea')
    focus_gallery_control(mac, field, 'AXTextArea')
    mac.key(6, flags=1 << 20)
    expect_field(mac, TITLE, field, draft, role='AXTextArea')
    mac.key(6, flags=(1 << 20) | (1 << 17))
    expect_field(mac, TITLE, field, 'a', role='AXTextArea')
    focus_gallery_control(mac, 'Identity', 'AXButton')
    mac.key(49)  # Space closes the focused heading.
    expect_popup_expanded(mac, 'Identity', False)
    wait_absent(mac, field, 'AXTextArea')
    mac.press(TITLE, 'Behavior')
    expect_popup_expanded(mac, 'Behavior', True)
    mac.wait_text(TITLE, 'Use the arrow keys to move between section headings.')
    mac.press(TITLE, 'Multiple')
    mac.press(TITLE, 'Identity')
    expect_popup_expanded(mac, 'Identity', True)
    expect_popup_expanded(mac, 'Behavior', True)
    expect_field(mac, TITLE, field, 'a', role='AXTextArea')
    mac.press(TITLE, 'Identity')
    expect_popup_expanded(mac, 'Identity', False)
    expect_popup_expanded(mac, 'Behavior', True)
    mac.press(TITLE, 'Keep one open')
    mac.press(TITLE, 'Behavior')
    expect_popup_expanded(mac, 'Behavior', True)
    mac.press(TITLE, 'Single')
    mac.press(TITLE, 'Toggle Behavior availability')
    expect_enabled(mac, 'Behavior', False)
    mac.press(TITLE, 'Toggle Behavior availability')
    expect_enabled(mac, 'Behavior', True)
    # Turn retention off while Identity is closed, then verify a fresh buffer.
    expect_popup_expanded(mac, 'Identity', False)
    mac.press(TITLE, 'Keep drafts')
    mac.press(TITLE, 'Identity')
    expect_popup_expanded(mac, 'Identity', True)
    expect_field(mac, TITLE, field, initial, role='AXTextArea')
    mac.press(TITLE, 'Keep drafts')
    print('GALLERY_DISCLOSURE_OK: retained native draft and Unicode undo/redo, OS Space, '
          'single/multiple/nonempty modes, disabled item policy, hidden AX retirement '
          'and fresh buffer after unmount', flush=True)
