"""Public tab presentations, retained panels and independent tab controls."""
import json
import time
from test_gallery import (
    TITLE, activate, element_rect, expect_field, expect_focus,
    focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
    wait_absent,
)


def exercise_basic(mac, images=None):
    mac.press(TITLE, 'Navigation & layout')
    mac.wait_text(TITLE, 'A workspace that keeps your place')
    window = mac.window(TITLE)
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    field = mac.wait_find(TITLE, 'Retained notes', 'AXTextArea')
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.set(field, 'AXFocused', mac.true)
        expect_focus(mac, 'Retained notes', role='AXTextArea')
        mac.key(0, flags=1 << 20)
        mac.key(0)
    finally:
        mac.release(field)
    expect_field(mac, TITLE, 'Retained notes', 'a', role='AXTextArea')
    # Both presentations keep the configured tab names and editor owners.
    # This exercises the public constructor's rich/plain transition, not merely
    # whether the badge text appears in accessibility (it is decorative).
    for _ in range(2):
        activate(mac, mac.wait_find(TITLE, 'Decorated workspace tabs', 'AXCheckBox'))
        expect_field(mac, TITLE, 'Retained notes', 'a', role='AXTextArea')
        tab = mac.wait_find(TITLE, 'Draft')
        mac.release(tab)
    for name in ('Underline', 'Tab', 'Outline', 'Pill', 'Segmented'):
        mac.press(TITLE, f'Tab style: {name}')
        expect_field(mac, TITLE, 'Retained notes', 'a', role='AXTextArea')
    for _ in range(2):
        activate(mac, mac.wait_find(TITLE, 'Customize tab targets', 'AXCheckBox'))
        expect_field(mac, TITLE, 'Retained notes', 'a', role='AXTextArea')
    activate(mac, mac.wait_find(TITLE, 'Draft'))
    expect_field(mac, TITLE, 'Retained draft', 'A separate draft with its own native editing history.', role='AXTextArea')
    hidden = mac.find(TITLE, 'Retained notes', 'AXTextArea')
    if hidden:
        mac.release(hidden)
        raise RuntimeError('Inactive retained tab editor remains accessible')
    activate(mac, mac.wait_find(TITLE, 'Notes'))
    expect_field(mac, TITLE, 'Retained notes', 'a', role='AXTextArea')
    reveal_gallery_control(mac, 'Close draft', 'AXButton')
    mac.press(TITLE, 'Close draft')
    mac.wait_text(TITLE, 'Closed draft')
    closed = mac.find(TITLE, 'Close draft', 'AXButton')
    if closed:
        mac.release(closed)
        raise RuntimeError('Closed structured tab retained its Close button')
    mac.press(TITLE, 'Reverse tabs')
    mac.press(TITLE, 'Restore tabs')
    close = mac.wait_find(TITLE, 'Close draft', 'AXButton')
    mac.release(close)
    for _ in range(2):
        activate(mac, mac.wait_find(TITLE, 'Truncate long tab names', 'AXCheckBox'))
    def tab_rect(label):
        node = mac.wait_find(TITLE, label)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)
    first_x = tab_rect('A thoughtful plan for a new workspace')[0]
    fixed_restore = tab_rect('Restore workspace tabs')
    mac.press(TITLE, 'Select last tab')
    mac.wait_text(TITLE, 'Selected archive')
    assert abs(tab_rect('A thoughtful plan for a new workspace')[0] - first_x) < 1, \
        'Controlled tab selection must not implicitly scroll'
    mac.press(TITLE, 'Reveal last tab')
    mac.wait_text(TITLE, 'Reveal requested for archive')
    for _ in range(40):
        vx, _, vw, _ = tab_rect('Closable workspace tabs')
        tx, _, tw, _ = tab_rect('Archived conversations')
        if vx - 1 <= tx and tx + tw <= vx + vw + 1:
            break
        time.sleep(.05)
    else:
        raise RuntimeError('Explicit tab reveal did not bring the last target into view')
    restore_after_scroll = tab_rect('Restore workspace tabs')
    assert abs(restore_after_scroll[0] - fixed_restore[0]) < 1, \
        'Tab-frame suffix moved with the scrolling tabs'
    archive_before_menu = tab_rect('Archived conversations')
    mac.press(TITLE, 'All tabs')
    activate(mac, mac.wait_find(TITLE, 'A thoughtful plan for a new workspace', 'AXMenuItem'))
    mac.wait_text(TITLE, 'Selected plan')
    assert abs(tab_rect('Archived conversations')[0] - archive_before_menu[0]) < 1, \
        'All-tabs menu selection must not implicitly reveal a tab'
    mac.press(TITLE, 'Close archive')
    mac.wait_text(TITLE, 'Closed archive')
    mac.press(TITLE, 'Restore tabs')
    expect_field(mac, TITLE, 'Retained notes', 'a', role='AXTextArea')

def exercise(mac, images=None):
    from mac_input_source import Sources, foreground_keys
    report = {'complete': False, 'cases': [],
              'scope': 'native geometry, keyboard and retained editing; not VoiceOver or presentation timing'}
    original_post = mac.post_key
    sources = Sources(mac)
    try:
        source = sources.selected()
    finally:
        sources.close()
    assert source in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC'), source
    report['input_source'] = source
    try:
        foreground_keys(mac)
        exercise_basic(mac, images)
        # The baseline ends back at Underline and with default target styling.
        activate(mac, mac.wait_find(TITLE, 'Customize tab targets', 'AXCheckBox'))
        for theme in ('Dark', 'Light'):
            select_gallery_appearance(mac, theme)
            for size, next_size in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                for variant in ('Underline', 'Tab', 'Outline', 'Pill', 'Segmented'):
                    mac.release(mac.wait_find(TITLE, 'Tab style: ' + variant, 'AXButton'))
                    reveal_gallery_control(mac, 'Notes', None)
                    notes = mac.wait_find(TITLE, 'Notes')
                    draft = mac.wait_find(TITLE, 'Draft')
                    try:
                        a, b = element_rect(mac, notes), element_rect(mac, draft)
                        assert abs(a[2]-160) < 1 and abs(b[2]-140) < 1, (a,b)
                        assert abs(a[3]-40) < 1 and abs(b[3]-40) < 1, (a,b)
                        assert abs(a[1]-b[1]) < 1 and a[0]+a[2] <= b[0]+1, (a,b)
                        case = dict(theme=theme, size=size, variant=variant,
                                    notes=a, draft=b, role=mac.text(notes, 'AXRole'))
                    finally:
                        mac.release(notes)
                        mac.release(draft)
                    focus_gallery_control(mac, 'Notes', None)
                    mac.key(124)  # Right selects/focuses Draft through the native tab group.
                    expect_field(mac, TITLE, 'Retained draft',
                                 'A separate draft with its own native editing history.', role='AXTextArea')
                    mac.key(123)
                    expect_field(mac, TITLE, 'Retained notes', 'a', role='AXTextArea')
                    report['cases'].append(case)
                    mac.press(TITLE, 'Tab style: ' + variant)
                mac.press(TITLE, size)
                mac.release(mac.wait_find(TITLE, next_size, 'AXButton'))
        activate(mac, mac.wait_find(TITLE, 'Customize tab targets', 'AXCheckBox'))
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, 'Retained notes', 'AXTextArea')
        mac.press(TITLE, 'Navigation & layout')
        expect_field(mac, TITLE, 'Retained notes',
                     'This note stays intact when you change tabs.', role='AXTextArea')
        report['page_remount'] = 'fresh native notes editor'
        report['complete'] = True
        print('GALLERY_TABS_OK', len(report['cases']), 'geometry/key cases; rich/plain, overflow, menu, close and retained panels', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'tabs-report.json').write_text(json.dumps(report, indent=2) + '\n')
