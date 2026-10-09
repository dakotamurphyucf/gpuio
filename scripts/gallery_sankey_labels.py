"""Public three-column labels: native selection, layout and scoped retirement."""
import time

from test_gallery import (
    TITLE, activate, element_rect, focus_gallery_control, reveal_gallery_control,
)
from test_canvas import screenshot


def exercise(mac, images):
    def press(label):
        reveal_gallery_control(mac, label, 'AXButton')
        mac.press(TITLE, label)

    def toggle(label):
        reveal_gallery_control(mac, label, 'AXCheckBox')
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    def rectangle(label, role='AXStaticText'):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def layout(tag, outside=True, hidden=False, narrow=False):
        reveal_gallery_control(mac, 'Chart preview: Flow labels', 'AXGroup')
        deadline = time.monotonic() + 10
        last = None
        while time.monotonic() < deadline:
            chart = rectangle('Chart preview: Flow labels', 'AXGroup')
            left = rectangle('Recorded activity')
            middle = rectangle('Step λ')
            right = None if hidden else rectangle('Delivered locally')
            last = chart, left, middle, right
            x, y, w, h = chart
            visible = all(x - 1 <= a and a + c <= x + w + 1 and y - 1 <= b
                          and b + d <= y + h + 1 for a, b, c, d in [left, middle, *([right] if right else [])])
            placed = not outside or (middle[1] + middle[3] < left[1]
                                     and (right is None or middle[1] + middle[3] < right[1]))
            if visible and placed and (not narrow or abs(w - 420) < 2):
                break
            time.sleep(.04)
        else:
            raise AssertionError(('Flow label bounds did not settle', tag, last))
        if hidden:
            absent = mac.find(TITLE, 'Delivered locally', 'AXStaticText')
            try:
                assert absent is None, 'Hidden target still exposes its label'
            finally:
                if absent:
                    mac.release(absent)
        if images:
            screenshot(mac, images / ('gallery-flow-labels-' + tag + '.png'), title=TITLE)
        print('FLOW_LABEL_LAYOUT', tag, last, flush=True)

    press('Flow labels')
    mac.wait_text(TITLE, 'Ready: Flow labels · 4 source values')
    press('Rich flow labels')
    mac.wait_text(TITLE, 'Ready: Flow labels · 4 source values · Vertical · Rich flow labels · Outside')
    layout('outside')
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    initial_theme = 'Dark' if theme else 'Light'
    if theme:
        mac.release(theme)
    changed_theme = 'Light' if initial_theme == 'Dark' else 'Dark'
    # Theme lives in the fixed header, outside the page scroll viewport.
    mac.press(TITLE, initial_theme)
    mac.release(mac.wait_find(TITLE, changed_theme, 'AXButton'))
    layout('theme-changed')
    mac.press(TITLE, changed_theme)
    mac.release(mac.wait_find(TITLE, initial_theme, 'AXButton'))
    layout('theme-restored')
    focus_gallery_control(mac, 'Chart preview: Flow labels', 'AXGroup')
    mac.key(115)
    mac.key(36)
    mac.wait_text(TITLE, 'Selected: Input → Processing · 100')
    press('Update chart samples')
    mac.wait_text(TITLE, 'Selected: Input → Processing · 101')
    toggle('Outside flow labels')
    mac.wait_text(TITLE, 'Ready: Flow labels · 4 source values · Vertical · Rich flow labels · Inside')
    layout('inside', outside=False)
    toggle('Outside flow labels')
    mac.wait_text(TITLE, 'Ready: Flow labels · 4 source values · Vertical · Rich flow labels · Outside')
    toggle('Long flow captions')
    mac.wait_text(TITLE, 'Intake · 日本語 λ · a deliberately long caption')
    layout('long')
    toggle('Narrow flow plot')
    layout('narrow', narrow=True)
    toggle('Bold flow captions')
    layout('bold', narrow=True)
    press('Hide target label')
    mac.wait_text(TITLE, 'Ready: Flow labels · 4 source values · Vertical · Hide target label · Outside')
    layout('hidden-target', hidden=True, narrow=True)
    press('Rich flow labels')
    toggle('Show flow labels')
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        found = mac.find(TITLE, 'Recorded activity', 'AXStaticText')
        if found is None:
            break
        mac.release(found)
        time.sleep(.04)
    else:
        raise AssertionError('Global label hiding did not retire labels')
    toggle('Show flow labels')
    layout('restored', narrow=True)
    press('View data')
    mac.release(mac.wait_find(TITLE, 'Chart preview: Flow labels · original data', 'AXTable'))
    mac.key(119)
    mac.release(mac.wait_find(TITLE, 'Row 7:', 'AXRow', contains=True, search_files=True))
    press('Back to chart')
    for label in ['Narrow flow plot', 'Bold flow captions', 'Long flow captions']:
        toggle(label)
    press('Default flows')
    press('Stacked bars')
    print('FLOW_LABEL_PUBLIC_OK: three columns, rich/long/hidden labels, placement, width/font changes, raw selection/update and original data', flush=True)
