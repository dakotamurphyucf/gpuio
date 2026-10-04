"""Public picker walkthrough. Requires a real macOS desktop; never a headless pass."""


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, expect_enabled, expect_popup_expanded, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot

    def open_picker(label):
        reveal_gallery_control(mac, label, 'AXPopUpButton')
        focus_gallery_control(mac, label, 'AXPopUpButton')
        activate(mac, mac.wait_find(TITLE, label, 'AXPopUpButton'))
        # Controlled opening makes a round trip through Bonsai before focus
        # enters the native popup. Do not send Escape to the closed trigger.
        expect_popup_expanded(mac, label, True, role='AXPopUpButton')

    def select(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXStaticText'))

    mac.press(TITLE, 'Dates & colors')
    open_picker('Workspace capabilities')
    select('Research')
    select('Drafting')
    mac.wait_text(TITLE, 'Selected: research, draft')
    activate(mac, mac.wait_find(TITLE, 'Clear capabilities', 'AXButton'))
    mac.wait_text(TITLE, 'Selected: None yet')
    mac.key(53)  # Escape closes the managed popup.
    wait_absent(mac, 'Clear capabilities', 'AXButton')

    open_picker('Export destination')
    expect_enabled(mac, 'Team archive · unavailable', False, 'AXStaticText')
    select('Downloads')
    mac.wait_text(TITLE, 'Destination: Downloads')
    wait_absent(mac, 'Team archive · unavailable', 'AXStaticText')
    reveal_gallery_control(mac, 'Allow destination changes', 'AXCheckBox')
    activate(mac, mac.wait_find(TITLE, 'Allow destination changes', 'AXCheckBox'))
    expect_enabled(mac, 'Export destination', False, 'AXPopUpButton')
    mac.wait_text(TITLE, 'Destination: Downloads')
    activate(mac, mac.wait_find(TITLE, 'Allow destination changes', 'AXCheckBox'))
    expect_enabled(mac, 'Export destination', True, 'AXPopUpButton')
    open_picker('Export destination')
    mac.key(53)
    wait_absent(mac, 'Team archive · unavailable', 'AXStaticText')

    open_picker('Workspace directory')
    # AXValue exercises the native editor setter; it is not physical typing/IME.
    mac.field(TITLE, 'Workspace directory', 'AXTextField', '4096')
    select('Workspace 4096')
    mac.wait_text(TITLE, 'Selected: Workspace 4096')
    wait_absent(mac, 'Workspace directory', 'AXTextField')
    open_picker('Workspace directory')
    assert mac.field(TITLE, 'Workspace directory', 'AXTextField') == '4096'
    mac.key(53)
    wait_absent(mac, 'Workspace directory', 'AXTextField')

    open_picker('New workspace')
    mac.wait_text(TITLE, 'Make room for your first idea')
    activate(mac, mac.wait_find(TITLE, 'Create workspace', 'AXButton'))
    select('My first workspace')
    mac.wait_text(TITLE, 'Selected: My first workspace')
    reveal_gallery_control(mac, 'Reset workspace example', 'AXButton')
    mac.press(TITLE, 'Reset workspace example')
    open_picker('New workspace')
    mac.wait_text(TITLE, 'Make room for your first idea')
    if images:
        screenshot(mac, images / 'gallery-choice-picker-empty.png', title=TITLE)
    mac.key(53)
    wait_absent(mac, 'Create workspace', 'AXButton')
    print('GALLERY_CHOICE_PICKER_OK: grouped multi-selection, controlled single selection, '
          'disabled policy, 4096-item search and retained query, empty/create/reset', flush=True)
