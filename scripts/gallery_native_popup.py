"""AppKit context popup in the public Feedback gallery; no Rust test hooks."""
import time
from test_native_popup_macos import wait_popup, enabled


def exercise(mac, images):
    from test_gallery import TITLE, reveal_gallery_control, raise_gallery
    mac.press(TITLE, 'Commands & feedback')
    reveal_gallery_control(mac, 'Advance preview', 'AXButton')
    raise_gallery(mac)
    button = mac.wait_find(TITLE, 'Advance preview', 'AXButton')
    try:
        assert enabled(mac, button)
        mac.set(button, 'AXFocused', mac.true)
    finally:
        mac.release(button)
    time.sleep(.6)  # Let the small decorative SVG complete before tracking.
    mac.key(109, 1 << 17)
    from gallery_native_popup_icons import exercise as exercise_icons
    exercise_icons(mac, images)
    mac.key(109, 1 << 17)
    menu = wait_popup(mac, title=TITLE)
    try:
        rows = mac.children(menu)
        try:
            assert mac.text(rows[0], 'AXTitle') == 'Preview workflow'
            assert not enabled(mac, rows[0]), 'Section label must stay non-actionable'
            mac.key(1)  # Native typeahead: Save preview.
            deadline = time.monotonic() + 5
            while True:
                selected = mac.children(menu, 'AXSelectedChildren')
                try:
                    names = [mac.text(row, 'AXTitle') for row in selected]
                finally:
                    for row in selected:
                        mac.release(row)
                if names == ['Save preview']:
                    break
                if time.monotonic() > deadline:
                    raise RuntimeError(f'Gallery native selection not observed: {names}')
                time.sleep(.03)
            mac.key(36)
        finally:
            for row in rows:
                mac.release(row)
    finally:
        mac.release(menu)
    wait_popup(mac, False, title=TITLE)
    mac.wait_text(TITLE, 'Preview saved')
    mac.press(TITLE, 'Dismiss saved preview')
    print('GALLERY_NATIVE_POPUP_OK: public wrapper, passive label, native keyboard command and Bonsai toast', flush=True)
