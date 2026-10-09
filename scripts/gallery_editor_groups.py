"""Keep every gallery editor example reachable within native memory admission."""


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, expect_field, focus_gallery_control, reveal_gallery_control,
    )

    def absent(label):
        node = mac.find(TITLE, label)
        if node:
            mac.release(node)
            raise AssertionError(f'Retired editor group remains mounted: {label}')

    mac.press(TITLE, 'Text editing')
    expect_field(mac, TITLE, 'Form workspace name', 'Northstar')
    focus_gallery_control(mac, 'Document title', 'AXTextField')
    mac.key(0, flags=1 << 20)
    mac.key(0)
    expect_field(mac, TITLE, 'Document title', 'a')

    # Visit every group twice. Search replacement mounts two additional editors;
    # repeated traversal also detects registrations left charged after retirement.
    for _ in range(2):
        mac.press(TITLE, 'Native input options')
        expect_field(mac, TITLE, 'Contact detail', 'hello@example.test')
        expect_field(mac, TITLE, 'Formatted draft', '12-34')
        expect_field(mac, TITLE, 'Filtered draft', 'workspace-17')
        absent('Document title')
        absent('Form workspace name')

        mac.press(TITLE, 'Multiline & search')
        mac.release(mac.wait_find(TITLE, 'Working notes', 'AXTextArea'))
        absent('Contact detail')
        reveal_gallery_control(mac, 'Find and replace', 'AXButton')
        mac.press(TITLE, 'Find and replace')
        mac.release(mac.wait_find(TITLE, 'Find in editor', 'AXTextArea'))
        mac.release(mac.wait_find(TITLE, 'Replace with', 'AXTextArea'))

        mac.press(TITLE, 'Forms & basic editing')
        expect_field(mac, TITLE, 'Document title', 'A place for good ideas')
        expect_field(mac, TITLE, 'Form workspace name', 'Northstar')
        absent('Working notes')
        absent('Find in editor')
        absent('Replace with')

    print('GALLERY_EDITOR_GROUPS_OK: all groups admitted, real editing, '
          'find/replace mounted, repeated retirement and fresh editor sessions',
          flush=True)
