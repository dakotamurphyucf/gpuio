"""Public picker walkthrough. Requires a real macOS desktop; never a headless pass."""


def exercise_keyboard_query(mac):
    """Route real OS keys into the open query; never silently assume a layout."""
    from mac_input_source import Sources, foreground_keys
    from test_gallery import TITLE, expect_field, wait_absent

    sources = Sources(mac)
    try:
        selected = sources.selected()
    finally:
        sources.close()
    if selected not in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC'):
        raise RuntimeError('Choice keyboard fixture requires active US or ABC layout: ' + str(selected))
    print('CHOICE_KEYBOARD_SOURCE', selected, flush=True)
    query = mac.wait_find(TITLE, 'Workspace directory', 'AXTextField')
    try:
        # The query remains the native keyboard owner while its active result
        # receives accessibility focus through aria_active_descendant_for.
        # Requiring AXFocused on the editor would reject that intended policy.
        mac.set(query, 'AXFocused', mac.true)
    finally:
        mac.release(query)
    original_post = mac.post_key
    try:
        foreground_keys(mac)  # Refuse global events if another app takes focus.
        mac.key(0, flags=1 << 20)  # Select retained query.
        for key in (21, 29, 25, 23):  # 4095 with the verified layout.
            mac.key(key)
        expect_field(mac, TITLE, 'Workspace directory', '4095')
        mac.release(mac.wait_find(TITLE, 'Workspace 4095', 'AXStaticText'))
        mac.key(51)  # Backspace changes the actual query and filtered result set.
        expect_field(mac, TITLE, 'Workspace directory', '409')
        mac.key(23)
        expect_field(mac, TITLE, 'Workspace directory', '4095')
        mac.release(mac.wait_find(TITLE, 'Workspace 4095', 'AXStaticText'))
        mac.key(125)  # Down highlights the sole enabled filtered result.
        mac.key(36)  # Return commits through native selection and Bonsai delivery.
        mac.wait_text(TITLE, 'Selected: Workspace 4095')
        wait_absent(mac, 'Workspace directory', 'AXTextField')
    finally:
        mac.post_key = original_post
    print('CHOICE_KEYBOARD_QUERY_OK: OS typing, backspace, filtered selection and dismissal', flush=True)


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
    exercise_keyboard_query(mac)
    open_picker('Workspace directory')
    assert mac.field(TITLE, 'Workspace directory', 'AXTextField') == '4095'
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
          'disabled policy, 4096-item AX and OS-key search, retained query, empty/create/reset', flush=True)
