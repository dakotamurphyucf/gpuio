"""Real native input and dynamic-result admission through the public OCaml demo."""
import time


def exercise(mac, images):
    from test_gallery import (TITLE, reveal_gallery_control, raise_gallery,
                              expect_field, expect_focus, absent, wait_absent)
    from test_canvas import screenshot

    reveal_gallery_control(mac, 'Search workspace', 'AXButton')
    raise_gallery(mac)
    mac.press(TITLE, 'Search workspace')
    mac.wait_text(TITLE, 'Recent notes')
    mac.wait_text(TITLE, 'Recent references')
    mac.wait_text(TITLE, 'Search results ready')
    expect_focus(mac, 'Workspace search', 'AXComboBox')
    mac.field(TITLE, 'Workspace search', 'AXComboBox', 'slow')
    mac.wait_text(TITLE, 'Searching workspace…')
    # A real native edit supersedes the longer Eio producer.
    mac.key(6)  # z
    expect_field(mac, TITLE, 'Workspace search', 'slowz', 'AXComboBox')
    mac.wait_text(TITLE, 'slowz notes')
    mac.wait_text(TITLE, 'slowz references')
    mac.wait_text(TITLE, 'Search results ready')
    time.sleep(.9)  # Past the original producer's 0.8-second delay.
    absent(mac, 'slow notes', 'AXStaticText')
    absent(mac, 'Recent notes', 'AXStaticText')
    mac.wait_text(TITLE, 'slowz notes')
    absent(mac, 'Loading commands', 'AXProgressIndicator')
    if images:
        screenshot(mac, images / 'gallery-palette-external-results.png', title=TITLE)
    mac.key(36)
    mac.wait_text(TITLE, 'Opened slowz notes')
    wait_absent(mac, 'Workspace search', 'AXComboBox')

    mac.press(TITLE, 'Search workspace')
    mac.wait_text(TITLE, 'Recent notes')
    mac.field(TITLE, 'Workspace search', 'AXComboBox', 'missing')
    mac.wait_text(TITLE, 'Searching workspace…')
    mac.wait_text(TITLE, 'Search results ready')
    wait_absent(mac, 'Recent notes', 'AXStaticText')
    absent(mac, 'Loading commands', 'AXProgressIndicator')
    mac.field(TITLE, 'Workspace search', 'AXComboBox', 'beta')
    mac.wait_text(TITLE, 'beta notes')
    mac.wait_text(TITLE, 'beta references')
    mac.key(125)  # Down to the second result.
    mac.key(36)
    mac.wait_text(TITLE, 'Opened beta references')

    mac.press(TITLE, 'Search workspace')
    mac.wait_text(TITLE, 'Recent notes')
    mac.field(TITLE, 'Workspace search', 'AXComboBox', 'slow')
    mac.wait_text(TITLE, 'Searching workspace…')
    mac.key(53)
    wait_absent(mac, 'Workspace search', 'AXComboBox')
    time.sleep(.9)
    absent(mac, 'slow notes', 'AXStaticText')
    # Bonsai keeps ordinary selection state across page visits. The closed
    # palette stays closed, and reopening starts a fresh native query/session.
    mac.press(TITLE, 'Presentation')
    mac.press(TITLE, 'Commands & feedback')
    mac.wait_text(TITLE, 'Opened beta references')
    absent(mac, 'Workspace search', 'AXComboBox')
    mac.press(TITLE, 'Search workspace')
    mac.wait_text(TITLE, 'Recent notes')
    absent(mac, 'slow notes', 'AXStaticText')
    mac.key(53)
    print('GALLERY_PALETTE_EXTERNAL_OK: dynamic staged commands, native typing, '
          'superseded producer, empty/replaced results, native selection and close/page cleanup', flush=True)
