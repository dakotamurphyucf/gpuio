"""Physical macOS walkthrough for the public searchable-list gallery.

Requires the same accessibility/input permissions as test_gallery.py. This is
acceptance code, not evidence that a desktop run has passed.
"""

from test_canvas import screenshot

TITLE = "GPUIO · Component Studio 1"


def focus(mac, label, role):
    node = mac.wait_find(TITLE, label, role)
    try:
        mac.set(node, "AXFocused", mac.true)
    finally:
        mac.release(node)


def exercise(mac, images):
    from mac_input_source import Sources
    from test_gallery import expect_field

    sources = Sources(mac)
    try:
        source = sources.selected()
    finally:
        sources.close()
    assert source in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC'), source
    mac.press(TITLE, "Lists, trees & tables")
    mac.press(TITLE, "Searchable list")
    mac.wait_text(TITLE, "Find something worth keeping")
    mac.wait_text(TITLE, "0 selected · 1004 loaded")
    focus(mac, "Search catalog", "AXTextField")
    mac.key(125)  # Down in the real native query; section heading is skipped.
    mac.key(36)
    mac.wait_text(TITLE, "Opened · Design notes · 0001")
    mac.key(36, flags=1 << 20)
    mac.wait_text(TITLE, "Preview · Design notes · 0001")
    focus(mac, "Searchable catalog", "AXList")
    mac.key(49)  # Space belongs to the focused list, not the query editor.
    mac.wait_text(TITLE, "1 selected · 1004 loaded")
    mac.key(109, flags=1 << 17)  # Shift-F10 routes the independent context intent.
    mac.wait_text(TITLE, "Actions for · Design notes · 0001 — Copy title, pin, or open")
    mac.key(53)
    mac.wait_text(TITLE, "Cursor cleared. Selection and search text are unchanged.")
    mac.wait_text(TITLE, "1 selected · 1004 loaded")
    focus(mac, "Search catalog", "AXTextField")
    mac.key(0, flags=1 << 20)
    for code in (15, 14, 46, 31, 17, 14):  # Actual US-layout typing: remote.
        mac.key(code)
    expect_field(mac, TITLE, 'Search catalog', 'remote')
    mac.wait_text(TITLE, "Remote field notes")
    mac.wait_text(TITLE, "1 selected · 1006 loaded")
    focus(mac, "Search catalog", "AXTextField")
    mac.key(125)  # Escape cleared the cursor; explicitly choose the fetched option.
    mac.key(36)
    mac.wait_text(TITLE, "Opened · Remote field notes")
    mac.press(TITLE, "Update current detail")
    mac.wait_text(TITLE, "A live update arrived while you were reading.")
    mac.press(TITLE, "Use horizontal list")
    mac.wait_text(TITLE, "Remote field notes")
    mac.press(TITLE, "Use vertical list")
    mac.press(TITLE, "No matches")
    mac.wait_text(TITLE, "No entries match. Try another search.")
    mac.wait_text(TITLE, "1 selected · 1006 loaded")
    mac.press(TITLE, "Try recovery")
    mac.wait_text(TITLE, "The sample catalog is temporarily unavailable.")
    mac.press(TITLE, "Retry search")
    mac.wait_text(TITLE, "Ready to explore")
    mac.wait_text(TITLE, "Design notes · 0001")
    mac.press(TITLE, "New collection")
    mac.wait_text(TITLE, "0 selected · 1004 loaded")
    mac.wait_text(TITLE, "Ready to explore")
    # Source replacement preserves the current query; it does not reset a live editor.
    expect_field(mac, TITLE, 'Search catalog', 'offline')
    if images:
        screenshot(mac, images / "gallery-selectable-list.png", title=TITLE)
    mac.press(TITLE, "Presentation")
    mac.wait_text(TITLE, "A little context goes a long way")
