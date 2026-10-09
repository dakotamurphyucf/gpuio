"""Rendered root and nested SVG decorations in the public AppKit context menu."""
import tempfile
import time
from pathlib import Path

from gallery_native_menu_bar import capture
from test_native_popup_macos import wait_popup, enabled


def exercise(mac, images):
    from test_gallery import TITLE
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-popup-icons-')
    output = images or Path(temporary.name)
    menu = wait_popup(mac, title=TITLE)
    rows = mac.children(menu)
    child_menu = None
    try:
        time.sleep(.3)
        root = capture(mac, menu, output/'native-popup-icons-root.png')
        assert root['names'] == ['Preview workflow', 'Advance preview', 'Save preview', '', 'Editing']
        assert all(root['checkmark_match'][i] > .92 for i in (1,4)), root
        # Use native typeahead to reach Editing, then open its submenu.
        mac.key(14)  # E.
        deadline = time.monotonic()+5
        while True:
            selected = mac.children(menu, 'AXSelectedChildren')
            try:
                names = [mac.text(row, 'AXTitle') for row in selected]
            finally:
                for row in selected:
                    mac.release(row)
            if names == ['Editing']:
                break
            assert time.monotonic()<deadline, ('Editing selection', names)
            time.sleep(.03)
        mac.key(124)  # Right.
        deadline = time.monotonic()+5
        from test_gallery import element_rect
        while time.monotonic()<deadline:
            children = mac.children(rows[4])
            try:
                if children and element_rect(mac, children[0])[2]>0:
                    child_menu = mac.retain(children[0])
                    break
            finally:
                for child in children:
                    mac.release(child)
            time.sleep(.03)
        assert child_menu, 'Native Editing submenu did not open'
        time.sleep(.3)
        # Native Copy is disabled without an editing target; AppKit dims both
        # text and artwork. Preserve the shape threshold but classify its dim ink.
        nested = capture(mac, child_menu, output/'native-popup-icons-nested.png',
                         minimum_contrast=20)
        assert nested['names'] == ['Selection actions', 'Copy preview selection'], nested
        nested_rows = mac.children(child_menu)
        try:
            assert not enabled(mac, nested_rows[0]), 'Section label must be passive'
            assert not enabled(mac, nested_rows[1]), 'Copy without an editing target must be disabled'
        finally:
            for row in nested_rows:
                mac.release(row)
        assert nested['checkmark_match'][0] < .90, ('Label unexpectedly has artwork', nested)
        assert nested['checkmark_match'][1] > .92, nested
        import json
        (output/'native-popup-icons-report.json').write_text(json.dumps({'root':root,'nested':nested},indent=2)+'\n')
        print('GALLERY_NATIVE_POPUP_ICONS_OK: root/submenu/nested command silhouettes through native keyboard navigation',flush=True)
    finally:
        if child_menu:
            mac.release(child_menu)
        for row in rows:
            mac.release(row)
        mac.release(menu)
        mac.key(53)
        mac.key(53)
        wait_popup(mac,False,title=TITLE)
        temporary.cleanup()
