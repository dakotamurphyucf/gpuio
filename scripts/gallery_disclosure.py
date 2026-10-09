"""Physical accordion state, native draft ownership and keyboard checks."""
import json


def exercise_matrix(mac, images=None):
    from mac_input_source import Sources, foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, activate, reveal_gallery_control, select_gallery_appearance, wait_absent,
    )

    report = {'complete': False, 'cases': [],
              'scope': 'OS keyboard/AX draft lifetimes and policies; not VoiceOver or animation timing'}
    sources = Sources(mac)
    original_post = mac.post_key
    try:
        report['input_source'] = sources.selected()
    finally:
        sources.close()
    assert report['input_source'] in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC')
    try:
        foreground_keys(mac)
        for current, following in (('Large', 'Compact'), ('Compact', 'Comfortable')):
            control = mac.find(TITLE, current, 'AXButton')
            if control:
                activate(mac, control)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for scale, following in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                mac.release(mac.wait_find(TITLE, scale, 'AXButton'))
                mac.press(TITLE, 'Presentation')
                wait_absent(mac, 'Disclosure notes', 'AXTextArea')
                mac.press(TITLE, 'Navigation & layout')
                exercise(mac)
                if images and scale == 'Comfortable':
                    reveal_gallery_control(mac, 'Disclosure notes', 'AXTextArea',
                                           scroll_in_left_gutter=True)
                    screenshot(mac, images / f'disclosure-{theme.lower()}.png', title=TITLE)
                report['cases'].append({'theme': theme, 'scale': scale, 'complete': True})
                print('GALLERY_DISCLOSURE_CASE', theme, scale, 'PASS', flush=True)
                mac.press(TITLE, scale)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        report['complete'] = True
        print('GALLERY_DISCLOSURE_MATRIX_OK', len(report['cases']), 'cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'disclosure-report.json').write_text(json.dumps(report, indent=2) + '\n')


def exercise(mac):
    from test_gallery import (
        TITLE, expect_enabled, expect_field, expect_focus, expect_popup_expanded,
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
    focus_gallery_control(mac, 'Identity', 'AXButton')
    mac.key(119)  # End navigates headings without changing expansion.
    expect_focus(mac, 'Lifetime')
    mac.key(115)  # Home.
    expect_focus(mac, 'Identity')
    mac.key(125)  # Down.
    expect_focus(mac, 'Behavior')
    expect_popup_expanded(mac, 'Identity', True)
    expect_popup_expanded(mac, 'Behavior', False)
    mac.press(TITLE, 'Toggle Behavior availability')
    expect_enabled(mac, 'Behavior', False)
    focus_gallery_control(mac, 'Identity', 'AXButton')
    mac.key(125)
    expect_focus(mac, 'Lifetime')
    mac.press(TITLE, 'Toggle Behavior availability')
    expect_enabled(mac, 'Behavior', True)
    mac.press(TITLE, 'Disabled')
    for label in ('Identity', 'Behavior', 'Lifetime'):
        expect_enabled(mac, label, False)
    expect_popup_expanded(mac, 'Identity', True)
    mac.press(TITLE, 'Disabled')
    for label in ('Identity', 'Behavior', 'Lifetime'):
        expect_enabled(mac, label, True)
    expect_field(mac, TITLE, field, initial, role='AXTextArea')
    print('GALLERY_DISCLOSURE_OK: retained native draft and Unicode undo/redo, OS Space, '
          'single/multiple/nonempty modes, heading Home/End/arrows, disabled skipping/group policy, hidden AX retirement '
          'and fresh buffer after unmount', flush=True)
