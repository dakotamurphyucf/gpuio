#!/usr/bin/env python3
"""Exercise the public gallery's native navigation, editing and window isolation."""
import argparse
import ctypes as C
from collections import Counter, defaultdict
import os
import re
from pathlib import Path
import subprocess
import tempfile
import time

from test_agent_chat import Mac
from test_canvas import screenshot
from window_pixels import read_png

TITLE = 'GPUIO · Component Studio 1'
SECOND = 'GPUIO · Component Studio 2'


def expect_field(mac, title, label, expected, role="AXTextField"):
    deadline = time.monotonic() + 10
    actual = None
    while time.monotonic() < deadline:
        actual = mac.field(title, label, role)
        if actual == expected:
            return
        time.sleep(0.025)
    raise RuntimeError(f'{label}: expected {expected!r}, got {actual!r}')


def wait_for_resource_cleanup(mac):
    """Refresh the explicit snapshot while native release acknowledgments settle."""
    deadline = time.monotonic() + 35
    actual = None
    while time.monotonic() < deadline:
        mac.press(TITLE, 'Refresh resource counts')
        node = mac.find(TITLE, 'Images:', 'AXStaticText', contains=True, deadline=deadline)
        if node:
            try:
                actual = mac.text(node, 'AXTitle')
            finally:
                mac.release(node)
            print('GALLERY_RESOURCE_SNAPSHOT', actual, flush=True)
            if actual == 'Images: 0 · Charts: 0 · Canvases: 0':
                return
        time.sleep(.05)
    raise RuntimeError(f'Native registrations did not retire: {actual!r}')


def exercise(mac, images):
    mac.wait_text(TITLE, 'A little context goes a long way')
    exercise_status_regions(mac, images)
    exercise_badges(mac, images)
    exercise_labels(mac, images)
    exercise_shimmer(mac, images)
    exercise_markers(mac, images)
    exercise_alerts(mac, images)
    exercise_tags(mac, images)
    exercise_chat_composition(mac, images)
    exercise_chat_list(mac, images)
    exercise_descriptions(mac, images)
    exercise_keyboard_labels(mac, images)
    exercise_binding_observations(mac, images)
    exercise_attachments(mac, images)
    exercise_groups(mac, images)
    exercise_separators(mac, images)
    exercise_links(mac, images)
    exercise_empty(mac, images)
    if images:
        screenshot(mac, images / 'gallery-presentation-dark.png', title=TITLE)
    mac.release(mac.wait_find(TITLE, 'Aster avatar', 'AXImage'))
    activate(mac, mac.wait_find(TITLE, 'Animate loading previews', 'AXCheckBox'))
    mac.wait_text(TITLE, 'Loading spinner')
    activate(mac, mac.wait_find(TITLE, 'Animate loading previews', 'AXCheckBox'))
    mac.press(TITLE, 'Selection & actions')
    mac.press(TITLE, 'Pressed 0 times')
    mac.wait_text(TITLE, 'Pressed 1 times')
    mac.press(TITLE, 'Numbers & codes')
    mac.wait_text(TITLE, 'Level: 35')
    slider = mac.wait_find(TITLE, 'Preview level', 'AXSlider')
    try:
        mac.perform(slider, 'AXIncrement')
    finally:
        mac.release(slider)
    mac.wait_text(TITLE, 'Level: 36')
    mac.wait_text(TITLE, 'Committed quantity: 12')
    mac.press(TITLE, 'Text editing')
    focus_gallery_control(mac, 'Document title', 'AXTextField')
    mac.key(0, flags=1 << 20)  # Command+A
    mac.key(0)  # a
    expect_field(mac, TITLE, 'Document title', 'a')
    activate(mac, mac.wait_find(TITLE, 'Show validation error', 'AXCheckBox'))
    mac.wait_text(TITLE, 'Choose a different title for this example.')
    expect_field(mac, TITLE, 'Document title', 'a')
    activate(mac, mac.wait_find(TITLE, 'Show validation error', 'AXCheckBox'))
    field = mac.wait_find(TITLE, 'Document title', 'AXTextField')
    try:
        mac.set(field, 'AXFocused', mac.true)
        expect_focus(mac, 'Document title', 'AXTextField')
    finally:
        mac.release(field)
    mac.key(36)  # Return: real OS submit into OCaml effect.
    mac.wait_text(TITLE, 'Submitted: a')
    mac.press(TITLE, 'Dark')
    mac.release(mac.wait_find(TITLE, 'Light', 'AXButton'))
    expect_field(mac, TITLE, 'Document title', 'a')
    for current, next_label in [('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')]:
        mac.press(TITLE, current)
        mac.release(mac.wait_find(TITLE, next_label, 'AXButton'))
        expect_field(mac, TITLE, 'Document title', 'a')
    if images:
        screenshot(mac, images / 'gallery-editing-light.png', title=TITLE)
    mac.press(TITLE, 'New window')
    mac.wait_text(SECOND, 'A little context goes a long way')
    mac.press(SECOND, 'Text editing')
    expect_field(mac, SECOND, 'Document title', 'A place for good ideas')
    expect_field(mac, TITLE, 'Document title', 'a')
    mac.close(SECOND)
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        node = mac.find(TITLE, 'Document title', 'AXTextField')
        if node:
            mac.release(node)
            raise RuntimeError('Unmounted editor remains accessible')
        mac.press(TITLE, 'Text editing')
        expect_field(mac, TITLE, 'Document title', 'A place for good ideas')


def reveal_gallery_control(mac, label, role, *, scroll_fraction=.78):
    # This helper sends desktop pointer events. Re-establish window ownership
    # for each action group; raising does not request focus on the target leaf.
    # The owner guard below still rejects occlusion before sending a click/wheel.
    raise_gallery(mac)
    mouse = GalleryMouse(mac)
    window = mac.window(TITLE)
    try:
        wx, wy, ww, wh = element_rect(mac, window)
    finally:
        mac.release(window)
    point = (wx + ww * scroll_fraction, wy + wh * .67)
    create = mac.cg.CGEventCreateScrollWheelEvent
    create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
    locate = mac.cg.CGEventSetLocation
    locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]
    for _ in range(24):
        node = mac.wait_find(TITLE, label, role)
        try:
            x, y, w, h = element_rect(mac, node)
        finally:
            mac.release(node)
        if y >= wy + 170 and y + h <= wy + wh - 30:
            return x, y, w, h
        mouse.send(5, point)
        mouse.check_owner(point)
        # A standalone section may start several screens above its target.
        # Fixed 75-pixel steps exhausted this bounded loop before reaching later
        # cards. Scale toward the measured target, at most one viewport per
        # event, and re-read layout after every step.
        below = y + h - (wy + wh - 30)
        distance = below if below > 0 else wy + 170 - y
        amount = round(min(max(75, distance), max(75, wh - 200)))
        event = create(None, 0, 1, C.c_int(-amount if below > 0 else amount))
        assert event
        try:
            locate(event, GalleryMouse.Point(*point))
            mouse.post(0, event)
        finally:
            mac.release(event)
        time.sleep(.08)
    raise RuntimeError(f'{label} did not become visible: control={(x,y,w,h)}, window={(wx,wy,ww,wh)}')


def exercise_links(mac, images):
    """Public composed links use real AppKit input and scoped SVG registrations."""
    mac.wait_text(TITLE, 'One destination. A richer invitation.')
    raise_gallery(mac)
    mouse = GalleryMouse(mac)
    labels = ['Open Design guide', 'Open Release notes', 'Open API reference']
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    original = mac.wait_find(TITLE, labels[0], 'AXLink')
    clicks = 0

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.08)

    def expect_revealed(label):
        expect_focus(mac, label, 'AXLink')
        deadline = time.monotonic() + 10
        while True:
            node = mac.wait_find(TITLE, label, 'AXLink')
            viewport = mac.wait_find(TITLE, 'Component preview', 'AXGroup')
            try:
                x, y, w, h = element_rect(mac, node)
                vx, vy, vw, vh = element_rect(mac, viewport)
            finally:
                mac.release(node)
                mac.release(viewport)
            if (w > 0 and h > 0 and x >= vx - 1 and y >= vy - 1
                    and x + w <= vx + vw + 1 and y + h <= vy + vh + 1):
                return
            assert time.monotonic() < deadline, (label, (x, y, w, h), (vx, vy, vw, vh))
            time.sleep(.025)

    def activated():
        nonlocal clicks
        clicks += 1
        mac.wait_text(TITLE, f'Link opens: {clicks}')

    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    try:
        for appearance in (current, alternate):
            for detail in (True, False):
                for icons in (True, False):
                    other = mac.wait_find(TITLE, labels[0], 'AXLink')
                    try:
                        assert equal(original, other), 'Composed content replaced link identity'
                    finally:
                        mac.release(other)
                    # Only reveal the first target; Tab must reach clipped successors.
                    x, y, w, h = reveal_gallery_control(mac, labels[0], 'AXLink')
                    point = (x + 28, y + h / 2)
                    mouse.check_owner(point)
                    mouse.send(5, point)
                    mouse.send(1, point)
                    mouse.send(2, point)
                    activated()
                    focus_gallery_control(mac, labels[0], 'AXLink')
                    for code in (36, 49):
                        mac.key(code)
                        activated()
                    node = mac.wait_find(TITLE, labels[0], 'AXLink')
                    try:
                        mac.perform(node, 'AXPress')
                    finally:
                        mac.release(node)
                    activated()
                    focus_gallery_control(mac, labels[0], 'AXLink')
                    mac.key(48)
                    expect_revealed(labels[1])
                    mac.key(48)
                    expect_revealed(labels[2])
                    mac.key(48, 1 << 17)
                    expect_revealed(labels[1])
                    if images and detail and icons:
                        screenshot(mac, images / f'gallery-links-{appearance.lower()}.png', title=TITLE)
                    toggle('Link icons')
                toggle('Link descriptions')
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
        toggle('Rich link previews')
        for label in labels:
            x, y, w, h = reveal_gallery_control(mac, label, 'AXLink')
            point = (x + 28, y + h / 2)
            print('GALLERY_RICH_LINK_POINTER', label, (x, y, w, h), point, flush=True)
            mouse.check_owner(point)
            mouse.send(5, point)
            mouse.send(1, point)
            mouse.send(2, point)
            activated()
            mac.wait_text(TITLE, f'Last opened: {label.removeprefix("Open ")}')
            expect_focus(mac, label, 'AXLink')
            mac.key(36)
            activated()
        other = mac.wait_find(TITLE, labels[0], 'AXLink')
        try:
            assert equal(original, other), 'Rich preview replaced link identity'
        finally:
            mac.release(other)
        if images:
            screenshot(mac, images / 'gallery-links-rich.png', title=TITLE)
        # Focused non-stops remain anchors between their signed-order neighbors.
        # AX focus and OS pointer events exercise distinct entry routes.
        toggle('Skip Release in Tab order')
        focus_gallery_control(mac, labels[1], 'AXLink')
        mac.key(48)
        expect_revealed(labels[2])
        x, y, w, h = reveal_gallery_control(mac, labels[1], 'AXLink')
        point = (x + 28, y + h / 2)
        mouse.check_owner(point)
        mouse.send(5, point)
        mouse.send(1, point)
        mouse.send(2, point)
        activated()
        expect_focus(mac, labels[1], 'AXLink')
        mac.key(48, 1 << 17)
        expect_revealed(labels[0])
        focus_gallery_control(mac, labels[1], 'AXLink')
        mac.key(36)
        activated()
        toggle('Skip Release in Tab order')
        toggle('Reverse link order')
        mac.wait_text(TITLE, 'Link order: API · Release · Design')
        focus_gallery_control(mac, labels[2], 'AXLink')
        mac.key(48)
        expect_revealed(labels[1])
        mac.key(48)
        expect_revealed(labels[0])
        toggle('Links in Tab order')
        focus_gallery_control(mac, labels[0], 'AXLink')
        mac.key(36)
        activated()
        toggle('Disable composed links')
        for label in labels:
            expect_enabled(mac, label, False, 'AXLink')
        x, y, w, h = reveal_gallery_control(mac, labels[0], 'AXLink')
        point = (x + 28, y + h / 2)
        mouse.check_owner(point)
        mouse.send(5, point)
        mouse.send(1, point)
        mouse.send(2, point)
        time.sleep(.12)
        mac.wait_text(TITLE, f'Link opens: {clicks}')
        toggle('Disable composed links')
        expect_enabled(mac, labels[0], True, 'AXLink')
        focus_gallery_control(mac, labels[0], 'AXLink')
        mac.key(36)
        activated()
        toggle('Links in Tab order')
        toggle('Reverse link order')
    finally:
        mac.release(original)
    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Refresh resource counts')
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    wait_for_resource_cleanup(mac)
    mac.press(TITLE, 'Presentation')
    mac.release(mac.wait_find(TITLE, labels[0], 'AXLink'))
    print('GALLERY_COMPOSED_LINK_OK: 8 theme/content/icon cases plus image/avatar/loading, '
          '42 pointer/Return/Space/AX '
          'actions, stable identity, signed Tab/reverse order and focused non-stop anchors '
          'with viewport reveal, disabled recovery and scoped SVG cleanup', flush=True)


def check_separator_clip_pixels(mac, root_bounds, frame_bounds, path):
    window = mac.window(TITLE)
    assert window, 'Separator window disappeared before pixel capture'
    try:
        wx, wy, ww, wh = element_rect(mac, window)
    finally:
        mac.release(window)
    screenshot(mac, path, title=TITLE)
    pixels = read_png(mac, path)

    def sample(x, y):
        return pixels.rgb((x - wx) * pixels.width / ww, (y - wy) * pixels.height / wh)

    x, y, w, h = root_bounds
    fx, fy, fw, fh = frame_bounds
    background = sample(fx + 2, fy + 2)
    outside = inside_paint = 0
    for ix in range(80):
        for iy in range(60):
            sx, sy = fx + 3 + ix * (fw - 6) / 79, fy + 3 + iy * (fh - 6) / 59
            difference = max(abs(a - b) for a, b in zip(sample(sx, sy), background))
            if sx < x - 2 or sx > x + w + 2 or sy < y - 2 or sy > y + h + 2:
                assert difference <= 8, ('Separator paints outside its clip', sx, sy, difference)
                outside += 1
            elif x + 2 < sx < x + w - 2 and y + 2 < sy < y + h - 2 and difference > 20:
                inside_paint += 1
    assert outside > 100 and inside_paint > 5, (outside, inside_paint)
    print('GALLERY_SEPARATOR_CLIP_PIXELS', outside, inside_paint, flush=True)


def exercise_separators(mac, images):
    mac.wait_text(TITLE, 'Room between ideas.')
    focus_gallery_control(mac, 'Keep separator updates', 'AXCheckBox')
    original = mac.wait_find(TITLE, 'Keep separator updates', 'AXCheckBox')
    root = mac.wait_find(TITLE, 'Rich separator preview', 'AXSplitter')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    toggles = {'Vertical separator': False, 'Dashed separator': False,
               'Label separator': True, 'Narrow separator': False,
               'Custom separator colors': False, 'Long separator label': False,
               'Bound separator label': False}

    def toggle(name, wanted):
        if toggles[name] != wanted:
            activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))
            toggles[name] = wanted

    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    cases = clipping_cases = 0
    try:
        mac.key(49)
        for appearance in (current, alternate):
            for vertical in (False, True):
                toggle('Vertical separator', vertical)
                for dashed in (False, True):
                    toggle('Dashed separator', dashed)
                    for labelled in (False, True):
                        toggle('Label separator', labelled)
                        for narrow in (False, True):
                            toggle('Narrow separator', narrow)
                            toggle('Custom separator colors', narrow)
                            mac.wait_text(TITLE, f'Separator: {"vertical" if vertical else "horizontal"} · '
                                          f'{"dashed" if dashed else "solid"} · '
                                          f'{"labelled" if labelled else "plain"} · '
                                          f'{"narrow" if narrow else "wide"} · '
                                          f'{"custom" if narrow else "default"}')
                            reveal_gallery_control(mac, 'Separator frame', 'AXGroup')
                            updated = mac.wait_find(TITLE, 'Rich separator preview', 'AXSplitter')
                            control = mac.wait_find(TITLE, 'Keep separator updates', 'AXCheckBox')
                            frame = mac.wait_find(TITLE, 'Separator frame', 'AXGroup')
                            value = mac.attr(control, 'AXValue')
                            try:
                                assert equal(root, updated) and equal(original, control), 'Separator restyle remounted retained nodes'
                                assert value and boolean(value), 'Separator restyle reset caller state'
                                x, y, w, h = element_rect(mac, updated)
                                fx, fy, fw, fh = element_rect(mac, frame)
                                assert abs(x + w / 2 - fx - fw / 2) < 1
                                assert abs(y + h / 2 - fy - fh / 2) < 1
                                assert abs((h if vertical else w) - (fh if vertical else fw)) < 1
                                cross = w if vertical else h
                                assert cross >= 20 if labelled else abs(cross - 1) < 1
                                assert x >= fx - 1 and y >= fy - 1 and x + w <= fx + fw + 1 and y + h <= fy + fh + 1
                            finally:
                                if value:
                                    mac.release(value)
                                for node in (updated, control, frame):
                                    mac.release(node)
                            expect_focus(mac, 'Keep separator updates', 'AXCheckBox')
                            if labelled:
                                mac.release(mac.wait_find(TITLE, 'Continue · 世界', 'AXStaticText'))
                            else:
                                wait_absent(mac, 'Continue · 世界', 'AXStaticText')
                            cases += 1
                            if images and not narrow and labelled:
                                screenshot(mac, images / f'gallery-separator-{appearance.lower()}-'
                                           f'{"vertical" if vertical else "horizontal"}-'
                                           f'{"dashed" if dashed else "solid"}.png', title=TITLE)
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
        toggle('Label separator', True)
        toggle('Narrow separator', True)
        toggle('Custom separator colors', True)
        toggle('Long separator label', True)
        long_text = 'Continue with another account · 保存した内容は保持されます · Your progress stays here'
        for appearance in (current, alternate):
            for vertical in (False, True):
                toggle('Vertical separator', vertical)
                mac.wait_text(TITLE, f'Separator: {"vertical" if vertical else "horizontal"} · '
                              'dashed · labelled · narrow · custom')
                sizes = []
                for clipped in (False, True, False):
                    toggle('Bound separator label', clipped)
                    mac.wait_text(TITLE, f'Separator label: long · {"bounded" if clipped else "natural"}')
                    reveal_gallery_control(mac, 'Separator frame', 'AXGroup')
                    updated = mac.wait_find(TITLE, 'Rich separator preview', 'AXSplitter')
                    label = mac.wait_find(TITLE, long_text, 'AXStaticText')
                    try:
                        assert equal(root, updated), 'Label clipping replaced the separator root'
                        x, y, w, h = element_rect(mac, updated)
                        lx, ly, lw, lh = element_rect(mac, label)
                        cross = w if vertical else h
                        sizes.append(cross)
                        if clipped:
                            assert abs(cross - (96 if vertical else 32)) < 1, (vertical, cross)
                        # Overflow clips paint, not the child's natural text layout
                        # or the full source exposed to accessibility.
                        assert lx < x + w and lx + lw > x and ly < y + h and ly + lh > y
                        if clipped:
                            frame = mac.wait_find(TITLE, 'Separator frame', 'AXGroup')
                            try:
                                frame_bounds = element_rect(mac, frame)
                            finally:
                                mac.release(frame)
                            name = f'gallery-separator-clipped-{appearance.lower()}-{"vertical" if vertical else "horizontal"}.png'
                            with tempfile.TemporaryDirectory(prefix='gpuio-separator-pixels-') as directory:
                                path = (images if images else Path(directory)) / name
                                check_separator_clip_pixels(mac, (x, y, w, h), frame_bounds, path)
                    finally:
                        mac.release(updated)
                        mac.release(label)
                    expect_focus(mac, 'Keep separator updates', 'AXCheckBox')
                    clipping_cases += 1
                assert sizes[0] > sizes[1] and abs(sizes[0] - sizes[2]) < 1, sizes
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
        toggle('Long separator label', False)
    finally:
        mac.release(original)
        mac.release(root)
    mac.press(TITLE, 'Runtime & windows')
    wait_absent(mac, 'Rich separator preview', 'AXSplitter')
    mac.press(TITLE, 'Presentation')
    mac.release(mac.wait_find(TITLE, 'Rich separator preview', 'AXSplitter'))
    print(f'GALLERY_SEPARATOR_OK: {cases} theme/axis/pattern/label/width cases, {clipping_cases} clipping/reset cases, centered geometry, '
          'native identity, checked state/focus and page retirement', flush=True)


def exercise_empty(mac, images):
    """Public rich slots: geometry, native identity and independent action owners."""
    mac.wait_text(TITLE, 'Make space for a fresh start.')
    raise_gallery(mac)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    original = mac.wait_find(TITLE, 'Keep empty-state updates', 'AXCheckBox')
    clicks = cases = border_cases = typography_cases = 0
    toggles = {'Frame empty media': False, 'Align empty slots to start': False,
               'Narrow empty preview': False, 'Show empty border': False,
               'Use solid empty border': False, 'Large empty description': False,
               'Compact empty line spacing': False}

    def checked(node):
        value = mac.attr(node, 'AXValue')
        try:
            assert value, 'Missing checkbox value'
            return bool(boolean(value))
        finally:
            if value:
                mac.release(value)

    def toggle(name, wanted):
        if toggles[name] != wanted:
            activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))
            toggles[name] = wanted

    def rect(label, role='AXGroup'):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def retained():
        node = mac.wait_find(TITLE, 'Keep empty-state updates', 'AXCheckBox')
        try:
            assert equal(original, node), 'Another slot replaced the content control'
            assert checked(node), 'Another slot reset the caller-owned checked model'
        finally:
            mac.release(node)

    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    try:
        focus_gallery_control(mac, 'Keep empty-state updates', 'AXCheckBox')
        mac.key(49)
        deadline = time.monotonic() + 5
        while not checked(original):
            assert time.monotonic() < deadline, 'Initial Space did not check the control'
            time.sleep(.025)
        for appearance in (current, alternate):
            for framed in (False, True):
                toggle('Frame empty media', framed)
                for leading in (False, True):
                    toggle('Align empty slots to start', leading)
                    wide_height = None
                    for narrow in (False, True):
                        toggle('Narrow empty preview', narrow)
                        mac.wait_text(TITLE, f'Empty layout: {"icon" if framed else "unframed"} · '
                                      f'{"start" if leading else "center"} · '
                                      f'{"narrow" if narrow else "wide"}')
                        retained()
                        expect_focus(mac, 'Keep empty-state updates', 'AXCheckBox')
                        reveal_gallery_control(mac, 'Create first item', 'AXButton')
                        media = rect('Empty rich media')
                        title = rect('Empty rich title')
                        description = rect('Empty rich description')
                        content = rect('Empty rich content')
                        extra = rect('Import instead', 'AXButton')
                        for above, below in zip((media, title, description, content),
                                                (title, description, content, extra)):
                            assert above[1] + above[3] <= below[1] + 1, 'Slots overlap or reorder'
                        assert media[2] > 0 and media[3] > 0, 'Media lost intrinsic dimensions'
                        anchor = content[0] if leading else content[0] + content[2] / 2
                        for slot in (media, title, description):
                            edge = slot[0] if leading else slot[0] + slot[2] / 2
                            assert abs(edge - anchor) <= 2, 'Independent slot alignment diverged'
                        if framed:
                            assert abs(media[2] - 32) <= 1 and abs(media[3] - 32) <= 1
                        else:
                            alex = rect('Empty preview Alex', 'AXImage')
                            sam = rect('Empty preview Sam', 'AXImage')
                            assert alex[2] > 0 and sam[2] > 0
                            assert alex[0] + alex[2] <= sam[0] + 1, 'Avatar row collapsed'
                        if narrow:
                            assert description[3] > wide_height, 'Narrow rich description must wrap'
                        else:
                            wide_height = description[3]
                        focus_gallery_control(mac, 'Create first item', 'AXButton')
                        mac.key(36)
                        clicks += 1
                        mac.wait_text(TITLE, f'Empty actions: {clicks}')
                        expect_focus(mac, 'Create first item')
                        focus_gallery_control(mac, 'Keep empty-state updates', 'AXCheckBox')
                        cases += 1
                        if images and not narrow and not leading and not framed:
                            reveal_gallery_control(mac, 'Import instead', 'AXButton')
                            screenshot(mac, images / f'gallery-empty-{appearance.lower()}.png', title=TITLE)
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
        toggle('Frame empty media', False)
        toggle('Align empty slots to start', False)
        toggle('Narrow empty preview', False)
        root = mac.wait_find(TITLE, 'Rich empty state', 'AXGroup')
        try:
            for appearance in (current, alternate):
                for bordered, solid in ((False, False), (True, False), (True, True),
                                         (False, True), (False, False)):
                    toggle('Show empty border', bordered)
                    toggle('Use solid empty border', solid)
                    mac.wait_text(TITLE, f'Empty border: {"visible" if bordered else "hidden"} · '
                                  f'{"solid override" if solid else "default dashed"}')
                    retained()
                    expect_focus(mac, 'Keep empty-state updates', 'AXCheckBox')
                    updated = mac.wait_find(TITLE, 'Rich empty state', 'AXGroup')
                    try:
                        assert equal(root, updated), 'Border restyle replaced the empty root'
                        _, _, width, _ = element_rect(mac, updated)
                        assert abs(width - 440) < 1, 'Border changed the requested outer width'
                    finally:
                        mac.release(updated)
                    border_cases += 1
                    if images and bordered:
                        reveal_gallery_control(mac, 'Empty rich media', 'AXGroup')
                        screenshot(mac, images / f'gallery-empty-border-{appearance.lower()}-'
                                   f'{"solid" if solid else "dashed"}.png', title=TITLE)
                mac.press(TITLE, appearance)
                mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
        finally:
            mac.release(root)
        description_text = mac.wait_find(TITLE, 'Empty description text', 'AXGroup')
        try:
            for appearance in (current, alternate):
                for large, font_size in ((False, 14), (True, 20)):
                    toggle('Large empty description', large)
                    heights = []
                    for compact in (False, True, False):
                        toggle('Compact empty line spacing', compact)
                        mac.wait_text(TITLE, f'Empty description: {20 if large else 14} px · '
                                      f'{"24 px spacing" if compact else "relative spacing"}')
                        reveal_gallery_control(mac, 'Empty description text', 'AXGroup')
                        updated = mac.wait_find(TITLE, 'Empty description text', 'AXGroup')
                        try:
                            assert equal(description_text, updated), 'Typography remounted the description'
                            heights.append(element_rect(mac, updated)[3])
                        finally:
                            mac.release(updated)
                        retained()
                        expect_focus(mac, 'Keep empty-state updates', 'AXCheckBox')
                        typography_cases += 1
                    relative, compact, restored = heights
                    lines = round(compact / 24)
                    assert lines > 1 and abs(compact - lines * 24) < 1, heights
                    # GPUI snaps each line to device pixels, preserving half
                    # logical pixels on Retina. Allow at most half a logical
                    # pixel per line (also valid on a 1x display).
                    assert abs(relative / lines - font_size * 1.625) <= .5, (large, heights)
                    assert abs(restored - relative) < 1, 'Omitting line-height override failed to restore ratio'
                    print('GALLERY_EMPTY_TYPOGRAPHY', appearance, font_size, lines, heights, flush=True)
                mac.press(TITLE, appearance)
                mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
            toggle('Large empty description', False)
        finally:
            mac.release(description_text)
        for toggle_label, absent_label, role in [
                ('Empty media', 'Empty rich media', 'AXGroup'),
                ('Empty title', 'Empty rich title', 'AXGroup'),
                ('Empty description', 'Read the empty-state guide', 'AXButton'),
                ('Empty extras', 'Import instead', 'AXButton')]:
            activate(mac, mac.wait_find(TITLE, toggle_label, 'AXCheckBox'))
            wait_absent(mac, absent_label, role)
            retained()
            expect_focus(mac, 'Keep empty-state updates', 'AXCheckBox')
            activate(mac, mac.wait_find(TITLE, toggle_label, 'AXCheckBox'))
            mac.release(mac.wait_find(TITLE, absent_label, role))
            retained()
        for label in ('Read the empty-state guide', 'Import instead'):
            mac.press(TITLE, label)
            clicks += 1
            mac.wait_text(TITLE, f'Empty actions: {clicks}')
        focus_gallery_control(mac, 'Keep empty-state updates', 'AXCheckBox')
        toggle('Frame empty media', False)
        toggle('Align empty slots to start', False)
        toggle('Narrow empty preview', False)
        activate(mac, mac.wait_find(TITLE, 'Use image empty media', 'AXCheckBox'))
        mac.wait_text(TITLE, 'Empty image decoded: 96 × 48')
        picture = rect('Empty media image', 'AXImage')
        assert abs(picture[2] - 96) <= 1 and abs(picture[3] - 48) <= 1
        retained()
        expect_focus(mac, 'Keep empty-state updates', 'AXCheckBox')
        if images:
            reveal_gallery_control(mac, 'Empty media image', 'AXImage')
            screenshot(mac, images / 'gallery-empty-image.png', title=TITLE)
        activate(mac, mac.wait_find(TITLE, 'Empty content', 'AXCheckBox'))
        wait_absent(mac, 'Keep empty-state updates', 'AXCheckBox')
        wait_absent(mac, 'Create first item', 'AXButton')
        activate(mac, mac.wait_find(TITLE, 'Empty content', 'AXCheckBox'))
        restored = mac.wait_find(TITLE, 'Keep empty-state updates', 'AXCheckBox')
        try:
            assert checked(restored), 'Caller-owned model must survive an absent slot'
            assert not equal(original, restored), 'Removed native control must retire'
        finally:
            mac.release(restored)
    finally:
        mac.release(original)
    mac.press(TITLE, 'Runtime & windows')
    wait_absent(mac, 'Rich empty state', 'AXGroup')
    mac.press(TITLE, 'Refresh resource counts')
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    wait_for_resource_cleanup(mac)
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, f'Empty actions: {clicks}')
    print(f'GALLERY_EMPTY_OK: {cases} theme/media/alignment/width cases; {border_cases} border cases; '
          f'{typography_cases} typography cases; intrinsic media, '
          f'wrapped slots, retained identity/focus, {clicks} Return/AX actions, '
          'native Space, decoded media, optional retirement and asset teardown', flush=True)


def exercise_groups(mac, images):
    """Panel geometry and independently styled slots retain native controls."""
    mac.wait_text(TITLE, 'Change the frame. Keep your place.')
    raise_gallery(mac)
    mouse = GalleryMouse(mac)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    original = mac.wait_find(TITLE, 'Keep group updates', 'AXCheckBox')
    get_bool = mac.cf.CFBooleanGetValue
    get_bool.restype, get_bool.argtypes = C.c_bool, [C.c_void_p]
    bool_type = mac.cf.CFBooleanGetTypeID
    bool_type.restype, bool_type.argtypes = C.c_ulong, []

    def checked(node=original):
        value = mac.attr(node, 'AXValue')
        try:
            assert value and mac.type_id(value) == bool_type()
            return bool(get_bool(value))
        finally:
            if value:
                mac.release(value)

    def rect(label, role):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    header = footer = True
    clicks = cases = 0
    try:
        assert not checked()
        focus_gallery_control(mac, 'Keep group updates', 'AXCheckBox')
        mac.key(49)  # Real Space input changes the retained body checkbox.
        deadline = time.monotonic() + 5
        while not checked():
            assert time.monotonic() < deadline, 'Group checkbox did not receive Space'
            time.sleep(.025)
        for appearance in (current, alternate):
            for variant in ('Card', 'Plain', 'Filled', 'Outline'):
                mac.press(TITLE, 'Group: ' + variant)
                for refined in (False, True):
                    if refined:
                        activate(mac, mac.wait_find(TITLE, 'Refine group slots', 'AXCheckBox'))
                    for wanted_header, wanted_footer in ((True, True), (False, True),
                                                          (True, False), (False, False)):
                        if header != wanted_header:
                            activate(mac, mac.wait_find(TITLE, 'Group header', 'AXCheckBox'))
                        if footer != wanted_footer:
                            activate(mac, mac.wait_find(TITLE, 'Group footer', 'AXCheckBox'))
                        header, footer = wanted_header, wanted_footer
                        mac.wait_text(TITLE, f'Group layout: {variant} · '
                                      f'{"refined" if refined else "default"} · '
                                      f'{"header" if header else "no header"} · '
                                      f'{"footer" if footer else "no footer"}')
                        node = mac.wait_find(TITLE, 'Keep group updates', 'AXCheckBox')
                        try:
                            assert equal(original, node), 'Group variant replaced its body control'
                            assert checked(), 'Group variant reset checked state'
                        finally:
                            mac.release(node)
                        reveal_gallery_control(mac, 'Configurable group', 'AXGroup')
                        gx, gy, gw, gh = rect('Configurable group', 'AXGroup')
                        bx, by, bw, bh = rect('Group content', 'AXGroup')
                        outer = 17 if variant == 'Card' else 0
                        padding = 24 if refined else 16 if variant in ('Filled', 'Outline') else 0
                        expected = outer + padding + (1 if variant == 'Outline' else 0)
                        assert abs(bx - gx - expected) < 2, (variant, refined, bx - gx, expected)
                        assert bw > 200 and bh > 40 and abs(gw - 440) < 2
                        for present, label, inset in ((header, 'Group heading action', 10),
                                                       (footer, 'Group footer action', 20)):
                            if present:
                                x, y, w, h = rect(label, 'AXButton')
                                assert abs(x - gx - outer - (inset if refined else 0)) < 2
                                assert (y + h <= by if label == 'Group heading action'
                                        else y >= by + bh)
                                point = (x + w / 2, y + h / 2)
                                mouse.check_owner(point)
                                mouse.send(5, point)
                                mouse.send(1, point)
                                mouse.send(2, point)
                                clicks += 1
                                mac.wait_text(TITLE, f'Group actions: {clicks}')
                            else:
                                wait_absent(mac, label, 'AXButton')
                        focus_gallery_control(mac, 'Run group action', 'AXButton')
                        mac.key(36)
                        clicks += 1
                        mac.wait_text(TITLE, f'Group actions: {clicks}')
                        expect_focus(mac, 'Run group action')
                        if images and header and footer and not refined:
                            screenshot(mac, images / f'gallery-group-{appearance.lower()}-{variant.lower()}.png', title=TITLE)
                        cases += 1
                    if refined:
                        activate(mac, mac.wait_find(TITLE, 'Refine group slots', 'AXCheckBox'))
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
    finally:
        mac.release(original)
    mac.press(TITLE, 'Runtime & windows')
    wait_absent(mac, 'Keep group updates', 'AXCheckBox')
    mac.press(TITLE, 'Refresh resource counts')
    wait_for_resource_cleanup(mac)
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    mac.press(TITLE, 'Presentation')
    # Bonsai retains this page's scalar model while its native subtree is absent.
    mac.wait_text(TITLE, f'Group actions: {clicks}')
    mac.wait_text(TITLE, 'Group layout: Outline · default · no header · no footer')
    restored = mac.wait_find(TITLE, 'Keep group updates', 'AXCheckBox')
    try:
        assert checked(restored), 'Page remount discarded its caller-owned model'
    finally:
        mac.release(restored)
    print(f'GALLERY_GROUP_OK: {cases} theme/variant/style/slot cases; geometry, retained '
          f'checked state and native identity, {clicks} pointer/Return actions, '
          'real Space input, focus and teardown', flush=True)


def exercise_keyboard_labels(mac, images):
    """Declared chord labels: appearance, identity and explicit registration."""
    mac.press(TITLE,'Presentation')
    mac.wait_text(TITLE,'Keys with meaning')
    focus_gallery_control(mac,'Keyboard preview draft','AXTextField')
    mac.key(0,flags=1<<20);mac.key(0)
    editor=mac.wait_find(TITLE,'Keyboard preview draft','AXTextField')
    equal=mac.cf.CFEqual
    equal.restype,equal.argtypes=C.c_bool,[C.c_void_p,C.c_void_p]
    temporary=tempfile.TemporaryDirectory(prefix='gpuio-keyboard-labels-')
    directory=images or Path(temporary.name)
    names=['filled','outline','plain']
    retained={}
    cases=paints=0

    def toggle(label):
        activate(mac,mac.wait_find(TITLE,label,'AXCheckBox'));time.sleep(.1)

    def cap(name):
        root=mac.wait_find(TITLE,f'Keyboard {name} example','AXGroup')
        children=mac.children(root,'AXChildren')
        try:
            assert len(children)==2,('keycap children',name,len(children))
            node=children[1]
            assert mac.text(node,'AXRole')=='AXStaticText', ('keycap role',name,mac.text(node,'AXRole'))
            return mac.retain(node)
        finally:
            for child in children:mac.release(child)
            mac.release(root)

    def check(spoken,refined=False):
        nonlocal cases
        rects={}
        for name in names:
            node=cap(name)
            try:
                values,children=mac.node_values(node)
                for child in children:mac.release(child)
                assert spoken in values[1:4],('keycap accessible name',name,spoken,values[:4])
                if name in retained:assert equal(retained[name],node),('keycap remounted',name)
                else:retained[name]=mac.retain(node)
                rects[name]=element_rect(mac,node)
            finally:mac.release(node)
        if not refined:
            assert abs(rects['filled'][3]-16)<1 and abs(rects['outline'][3]-18)<1,('keycap default height',rects)
            assert rects['plain'][3]>rects['filled'][3],('plain inherits typography',rects)
        else:
            assert abs(rects['filled'][3]-28)<1 and abs(rects['outline'][3]-30)<1,('refined cap height',rects)
        current=mac.wait_find(TITLE,'Keyboard preview draft','AXTextField')
        try:assert equal(editor,current),'keycap changes remounted editor'
        finally:mac.release(current)
        expect_field(mac,TITLE,'Keyboard preview draft','a')
        cases+=1
        return rects

    def paint(theme,spoken,refined):
        nonlocal paints
        focus_gallery_control(mac,'Keyboard preview draft','AXTextField')
        reveal_gallery_control(mac,'Keyboard filled example','AXGroup',scroll_fraction=.94)
        rects=check(spoken,refined)
        window=mac.window(TITLE)
        try:wx,wy,ww,wh=element_rect(mac,window)
        finally:mac.release(window)
        GalleryMouse(mac).send(5,(wx+ww-25,wy+110))
        time.sleep(.1)
        path=directory/f'gallery-keyboard-{theme.lower()}-{int(refined)}.png'
        screenshot(mac,path,title=TITLE);pixels=read_png(mac,path)
        def rgb(x,y):return pixels.rgb((x-wx)*pixels.width/ww,(y-wy)*pixels.height/wh)
        close=lambda a,b:max(abs(c-d) for c,d in zip(a,b))<=5
        dark=theme=='Dark'
        expected={'filled':(39,46,59) if dark else (240,242,246),
                  'outline':(27,32,43) if dark else (255,255,255),
                  'plain':(25,33,44) if dark else (255,255,255)}
        for name,(x,y,w,h) in rects.items():
            assert y>wy+165 and y+h<wy+wh-25,('keycap visibility',name,rects)
            # Plain has no padding; sample beside its glyphs within the stretched
            # column, rather than reading an antialiased text pixel.
            observed=rgb(x+w-3,y+3)
            assert close(observed,expected[name]),('keycap fill',theme,name,refined,observed,expected[name])
        x,y,w,h=rects['outline']
        border=(62,72,91) if dark else (211,217,227)
        assert any(close(rgb(x+w/2,y+d),border) for d in [.25,.5,.75]),('keycap outline',theme)
        paints+=1

    def invoke(expected):
        focus_gallery_control(mac,'Keyboard preview draft','AXTextField')
        mac.key(40,flags=1<<20) # Command-K, independent of the preview platform.
        time.sleep(.2)
        mac.wait_text(TITLE,f'Keyboard invocations: {expected}')
        expect_field(mac,TITLE,'Keyboard preview draft','a')

    try:
        check('Command + K')
        invoke(0) # Displaying a chord alone never registers it.
        toggle('Register example shortcut');invoke(1)
        toggle('Enable example command');invoke(1)
        toggle('Enable example command');invoke(2)
        toggle('Linux keyboard labels');check('Control + K');invoke(3)
        toggle('Register example shortcut');invoke(3)
        toggle('Reverse keyboard labels');check('Control + K')
        toggle('Reverse keyboard labels');check('Control + K')
        # Formatting changes retain the same native text objects.
        for old,new,spoken in [('k','delete','Delete'),('delete','left','Left arrow'),('left','é','É'),('é','k','K')]:
            mac.press(TITLE,'Keyboard key: '+old)
            mac.release(mac.wait_find(TITLE,'Keyboard key: '+new,'AXButton'))
            check('Control + '+spoken)
        toggle('Linux keyboard labels');check('Command + K')
        theme_node=mac.find(TITLE,'Dark','AXButton')
        initial='Dark' if theme_node else 'Light'
        if theme_node:mac.release(theme_node)
        for theme in [initial,'Light' if initial=='Dark' else 'Dark']:
            paint(theme,'Command + K',False)
            toggle('Refine keyboard labels');paint(theme,'Command + K',True);toggle('Refine keyboard labels')
            if theme==initial:mac.press(TITLE,theme)
        toggle('Show keyboard labels');wait_absent(mac,'Keyboard filled example','AXGroup')
        toggle('Show keyboard labels')
        for name in names:
            replacement=cap(name)
            try:assert not equal(retained[name],replacement),('removed keycap retained owner',name)
            finally:mac.release(replacement)
        mac.press(TITLE,'Runtime & windows');wait_absent(mac,'Keyboard preview draft','AXTextField')
        mac.press(TITLE,'Presentation')
        replacement=mac.wait_find(TITLE,'Keyboard preview draft','AXTextField')
        try:assert not equal(editor,replacement),'keyboard page retained native editor'
        finally:mac.release(replacement)
        mac.wait_text(TITLE,'Keyboard invocations: 3')
        print(f'GALLERY_KEYBOARD_LABELS_OK: {cases} layout/name/identity cases, {paints} GPU theme/refinement cases; explicit registration/disable/platform routing and slot/page retirement',flush=True)
    finally:
        mac.release(editor)
        for node in retained.values():mac.release(node)
        temporary.cleanup()


def exercise_binding_observations(mac, images):
    """Public mounted observer: live focus, declarations, native keymaps and retirement."""
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'Shortcuts in context')
    focus_gallery_control(mac, 'Live binding draft', 'AXTextField')
    mac.key(0, flags=1 << 20); mac.key(0)
    editor = mac.wait_find(TITLE, 'Live binding draft', 'AXTextField')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    clipboard_env = dict(os.environ, LANG='en_US.UTF-8')
    saved = subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=clipboard_env).stdout
    cases = 0

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.1)

    def row(name, status, cap=None):
        nonlocal cases
        mac.wait_text(TITLE, name + ': ' + status)
        root = mac.wait_find(TITLE, 'Live binding ' + name, 'AXGroup')
        children = mac.children(root, 'AXChildren')
        try:
            assert len(children) == (2 if cap else 1), ('binding row children', name, len(children), cap)
            if cap:
                values, descendants = mac.node_values(children[1])
                for child in descendants: mac.release(child)
                assert values[0] == 'AXStaticText' and cap in values[1:4], ('observed keycap', values, cap)
        finally:
            for child in children: mac.release(child)
            mac.release(root)
        current = mac.wait_find(TITLE, 'Live binding draft', 'AXTextField')
        try: assert equal(editor, current), 'binding observation remounted editor'
        finally: mac.release(current)
        expect_field(mac, TITLE, 'Live binding draft', 'a')
        cases += 1

    def epoch():
        node = mac.wait_find(TITLE, 'Binding sample: ', 'AXStaticText', contains=True)
        try:
            values, children = mac.node_values(node)
            for child in children: mac.release(child)
            return next(int(text.split(': ')[1]) for text in values[1:4] if text.startswith('Binding sample: '))
        finally: mac.release(node)

    def invoke(keycode, count):
        focus_gallery_control(mac, 'Live binding draft', 'AXTextField')
        mac.key(keycode, flags=1 << 20)
        time.sleep(.15)
        mac.wait_text(TITLE, f'Live invocations: {count}')
        expect_field(mac, TITLE, 'Live binding draft', 'a')

    def mode(old, new):
        mac.press(TITLE, 'Binding context: ' + old)
        mac.release(mac.wait_find(TITLE, 'Binding context: ' + new, 'AXButton'))

    try:
        row('Preview action', 'Available (override)', 'Command + K')
        row('Copy', 'Native widget binding', 'Command + C')
        time.sleep(.2)
        before = epoch(); time.sleep(.4)
        assert epoch() == before, 'unchanged live observer produced repeated samples'
        # Follow the observed Copy chord through actual OS input.
        mac.key(0, flags=1 << 20); mac.key(8, flags=1 << 20)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=clipboard_env).stdout == b'a':
                break
            time.sleep(.05)
        else: raise AssertionError('observed native Copy chord did not copy the selection')
        invoke(40, 1)
        toggle('Enable live shortcut'); row('Preview action', 'Disabled', 'Command + K')
        invoke(40, 1)
        toggle('Register live shortcut'); row('Preview action', 'Not registered')
        toggle('Use alternate live shortcut')
        toggle('Register live shortcut'); row('Preview action', 'Disabled', 'Command + L')
        toggle('Enable live shortcut'); row('Preview action', 'Available (override)', 'Command + L')
        invoke(40, 1); invoke(37, 2)
        toggle('Linux live labels')
        row('Preview action', 'Available (override)', 'Control + L')
        row('Copy', 'Native widget binding', 'Super + C')
        toggle('Linux live labels')
        mode('Focused', 'Editor')
        focus_gallery_control(mac, 'Run preview action', 'AXButton')
        row('Preview action', 'Declared', 'Command + L')
        row('Copy', 'Declared', 'Command + C')
        mode('Editor', 'Here')
        row('Preview action', 'Declared', 'Command + L')
        wait_absent(mac, 'Live binding Copy', 'AXGroup')
        mode('Here', 'Native')
        row('Copy', 'Declared', 'Command + C')
        wait_absent(mac, 'Live binding Preview action', 'AXGroup')
        toggle('Invalid native facts'); mac.wait_text(TITLE, 'Invalid native context')
        wait_absent(mac, 'Live binding Copy', 'AXGroup')
        toggle('Invalid native facts'); row('Copy', 'Declared', 'Command + C')
        assert epoch() == 1, 'configuration replacement did not restart the epoch'
        mode('Native', 'Focused')
        row('Copy', 'No binding in this context')
        focus_gallery_control(mac, 'Live binding draft', 'AXTextField')
        row('Copy', 'Native widget binding', 'Command + C')
        toggle('Show live bindings'); mac.wait_text(TITLE, 'Live bindings hidden')
        wait_absent(mac, 'Live binding Copy', 'AXGroup')
        invoke(37, 3)  # Observation retirement does not unregister the command.
        toggle('Show live bindings'); row('Copy', 'Native widget binding', 'Command + C')
        for _ in range(2):
            theme = mac.find(TITLE, 'Dark', 'AXButton')
            old = 'Dark' if theme else 'Light'
            if theme: mac.release(theme)
            mac.press(TITLE, old)
            row('Preview action', 'Available (override)', 'Command + L')
            row('Copy', 'Native widget binding', 'Command + C')
        if images:
            reveal_gallery_control(mac, 'Live binding Copy', 'AXGroup')
            screenshot(mac, images / 'gallery-binding-observations.png', title=TITLE)
        mac.wait_text(TITLE, 'Live invocations: 3')
        mac.press(TITLE, 'Runtime & windows')
        wait_absent(mac, 'Live binding draft', 'AXTextField')
        wait_absent(mac, 'Live binding Copy', 'AXGroup')
        mac.press(TITLE, 'Presentation')
        replacement = mac.wait_find(TITLE, 'Live binding draft', 'AXTextField')
        try: assert not equal(editor, replacement), 'page departure retained native binding editor'
        finally: mac.release(replacement)
        focus_gallery_control(mac, 'Live binding draft', 'AXTextField')
        mac.wait_text(TITLE, 'Copy: Native widget binding')
        mac.wait_text(TITLE, 'Live invocations: 3')
        print(f'GALLERY_BINDING_OBSERVATIONS_OK: {cases} live binding/name/identity cases, OS Copy and shortcut invocation, context/config replacement, idle silence, observer/page retirement', flush=True)
    finally:
        mac.release(editor)
        subprocess.run(['/usr/bin/pbcopy'], input=saved, check=True, env=clipboard_env)


def exercise_descriptions(mac, images):
    """Rich description packing, semantic pairs and retained native children."""
    mac.press(TITLE,'Presentation')
    mac.wait_text(TITLE,'Details that stay together')
    focus_gallery_control(mac,'Description value','AXTextField')
    mac.key(0,flags=1<<20);mac.key(0)
    editor=mac.wait_find(TITLE,'Description value','AXTextField')
    term_action=mac.wait_find(TITLE,'Description term action','AXButton')
    value_action=mac.wait_find(TITLE,'Description value action','AXButton')
    equal=mac.cf.CFEqual
    equal.restype,equal.argtypes=C.c_bool,[C.c_void_p,C.c_void_p]
    columns=3;vertical=False;bordered=True;mixed=False;separators=False;reversed_items=False
    cases=actions=paint_cases=0
    temporary=tempfile.TemporaryDirectory(prefix='gpuio-description-')
    directory=images or Path(temporary.name)

    def toggle(label):
        activate(mac,mac.wait_find(TITLE,label,'AXCheckBox'));time.sleep(.1)

    def cycle(prefix,value,values):
        mac.press(TITLE,prefix+str(value))
        result=values[(values.index(value)+1)%len(values)]
        mac.release(mac.wait_find(TITLE,prefix+str(result),'AXButton'))
        return result

    def identity(value_present=True):
        for label,role,old in [('Description value','AXTextField',editor),('Description term action','AXButton',term_action)]+([('Description value action','AXButton',value_action)] if value_present else []):
            current=mac.wait_find(TITLE,label,role)
            try:assert equal(current,old),('description control remounted',label)
            finally:mac.release(current)
        expect_field(mac,TITLE,'Description value','a')

    def reading_order(root):
        pairs=[]
        def visit(node):
            subrole=mac.text(node,'AXSubrole')
            children=mac.children(node,'AXChildren')
            try:
                if subrole in ('AXTerm','AXDefinition'):
                    assert len(children)==1, ('semantic slot children',subrole,len(children))
                    pairs.append((subrole,mac.text(children[0],'AXTitle')))
                else:
                    for child in children:visit(child)
            finally:
                for child in children:mac.release(child)
        visit(root)
        return pairs

    def slot(n,kind):
        child=mac.wait_find(TITLE,f'Description {kind} {n}','AXGroup')
        parent=mac.attr(child,'AXParent')
        try:
            assert parent and mac.text(parent,'AXSubrole')==('AXTerm' if kind=='term' else 'AXDefinition'),('description semantic parent',kind,mac.text(parent,'AXSubrole') if parent else None)
            return element_rect(mac,parent)
        finally:mac.release(parent);mac.release(child)

    def geometry():
        nonlocal cases
        root=mac.wait_find(TITLE,'Workspace details','AXList')
        try:
            x,y,w,h=element_rect(mac,root)
            pairs=reading_order(root)
        finally:mac.release(root)
        border=1 if bordered else 0
        inner=w-2*border
        spans=[1,min(2,columns),1,1,1,columns,1] if mixed else [1]*12
        entries=list(enumerate(spans))
        if reversed_items:entries.reverse()
        assert pairs==[(role,f'Description {kind} {n}') for n,_ in entries for role,kind in [('AXTerm','term'),('AXDefinition','definition')]], ('description reading order',pairs,entries)
        if separators:entries[3:3]=[(None,columns),(None,columns)]
        rows=[];used=0
        for n,span in entries:
            if not rows or used+span>columns:rows.append([]);used=0
            rows[-1].append((n,span));used+=span
        previous_y=None
        for row in rows:
            cells=[(n,span) for n,span in row if n is not None]
            if not cells:continue
            extra=(columns-sum(span for _,span in cells))/columns/len(cells)
            offset=0.;row_y=None
            for n,span in cells:
                tx,ty,tw,th=slot(n,'term')
                dx,dy,dw,dh=slot(n,'definition')
                cell_width=inner*(span/columns+extra)
                assert abs(tx-(x+border+offset))<1,('description cell x',columns,n,tx,x+border+offset,vertical,w)
                if row_y is None:row_y=ty
                else:assert abs(ty-row_y)<1,('description unexpectedly wrapped',columns,n,row_y,ty,vertical,w)
                if vertical:
                    assert abs(tw-cell_width)<1 and abs(dw-cell_width)<1,('stacked cell width',columns,n,tw,dw,cell_width)
                    assert abs(dx-tx)<1 and abs(dy-(ty+th))<1,('stacked term/definition order',n,(tx,ty,tw,th),(dx,dy,dw,dh))
                else:
                    assert abs(dx-(tx+tw))<1 and abs(dy-ty)<1,('horizontal term/definition order',n,(tx,ty,tw,th),(dx,dy,dw,dh))
                    if cell_width>=100:assert abs(tw+dw-cell_width)<1,('horizontal cell width',n,tw,dw,cell_width)
                offset+=cell_width
            if previous_y is not None:assert row_y>previous_y+5,('description row order',previous_y,row_y)
            previous_y=row_y
        cases+=1

    def paint(theme,refined=False):
        nonlocal paint_cases
        focus_gallery_control(mac,'Description term action','AXButton')
        reveal_gallery_control(mac,'Description term 2','AXGroup')
        window=mac.window(TITLE)
        try:wx,wy,ww,wh=element_rect(mac,window)
        finally:mac.release(window)
        GalleryMouse(mac).send(5,(wx+ww-25,wy+110))
        time.sleep(.08)
        tx,ty,tw,th=slot(2,'term');dx,dy,dw,dh=slot(2,'definition')
        assert ty>=wy+160 and ty+th<=wy+wh-15,('description paint visibility',ty,th,wy,wh)
        path=directory/f'gallery-description-paint-{paint_cases:03d}.png'
        screenshot(mac,path,title=TITLE)
        pixels=read_png(mac,path)
        def rgb(x,y):return pixels.rgb((x-wx)*pixels.width/ww,(y-wy)*pixels.height/wh)
        dark=theme=='Dark'
        parent=(25,33,44) if dark else (255,255,255)
        label=((45,57,73) if dark else (211,220,229)) if refined else ((39,46,59) if dark else (240,242,246)) if bordered else parent
        close=lambda a,b:max(abs(c-d) for c,d in zip(a,b))<=5
        assert close(rgb(tx+tw-4,ty+th/2),label),('description label fill',theme,vertical,bordered,refined,rgb(tx+tw-4,ty+th/2),label)
        assert close(rgb(dx+dw-3,dy+dh/2),parent),('description value fill',theme,rgb(dx+dw-3,dy+dh/2),parent)
        if bordered:
            edge=(62,72,91) if dark else (211,217,227)
            samples=[rgb(tx+tw/2,ty+th-d) if vertical else rgb(tx+tw-d,ty+th/2) for d in [.25,.5,.75]]
            assert any(close(value,edge) for value in samples),('description slot border',theme,vertical,samples,edge)
        paint_cases+=1

    try:
        for _ in range(10):
            for axis in [False,True]:
                if axis:toggle('Vertical description');vertical=True
                geometry();identity()
            toggle('Vertical description');vertical=False
            columns=cycle('Description columns: ',columns,list(range(1,11)))
        toggle('Mixed description spans');mixed=True
        for _ in range(10):
            geometry();identity()
            columns=cycle('Description columns: ',columns,list(range(1,11)))
        toggle('Description separators');separators=True
        geometry();identity()
        toggle('Reverse description entries');reversed_items=True
        geometry();identity()
        toggle('Vertical description');vertical=True
        geometry();identity()
        toggle('Mixed description spans');mixed=False
        toggle('Description separators');separators=False
        toggle('Reverse description entries');reversed_items=False
        width='720.0'
        for _ in range(4):
            while columns!=10:columns=cycle('Description columns: ',columns,list(range(1,11)))
            geometry();identity()
            width=cycle('Description width: ',width,['720.0','607.3','541.0','333.3'])
        while columns!=3:columns=cycle('Description columns: ',columns,list(range(1,11)))
        for _ in range(2):
            focus_gallery_control(mac,'Description term action','AXButton');mac.key(36);actions+=1
            mac.wait_text(TITLE,f'Description actions: {actions}')
            focus_gallery_control(mac,'Description value action','AXButton');mac.key(49);actions+=1
            mac.wait_text(TITLE,f'Description actions: {actions}')
            toggle('Vertical description');vertical=not vertical
            geometry();identity()
        toggle('Long description value');geometry();identity();toggle('Long description value')
        toggle('Bordered description');bordered=False
        geometry();identity();toggle('Bordered description');bordered=True
        toggle('Proportional description labels');geometry();identity();toggle('Proportional description labels')
        toggle('Refine description slots');geometry();identity();toggle('Refine description slots')
        # Sizes, both bordered axes and real theme paint. All cells remain under
        # their original keyed parents while native line packing changes.
        theme_node=mac.find(TITLE,'Dark','AXButton')
        initial_theme='Dark' if theme_node else 'Light'
        if theme_node:mac.release(theme_node)
        size_name='M'
        for theme in [initial_theme,'Light' if initial_theme=='Dark' else 'Dark']:
            for _ in range(2):
                for _ in range(2):
                    for _ in range(4):
                        geometry();identity()
                        parent=slot(2,'term')
                        child=mac.wait_find(TITLE,'Description term 2','AXGroup')
                        try:content=element_rect(mac,child)
                        finally:mac.release(child)
                        padding={'XS':(4,2),'S':(4,2),'M':(8,4),'L':(12,6)}[size_name] if bordered else (0,0)
                        edge=1 if bordered and not vertical else 0
                        assert abs(content[0]-parent[0]-padding[0]-edge)<1 and abs(content[1]-parent[1]-padding[1])<1, ('description size padding',size_name,vertical,bordered,parent,content)
                        paint(theme)
                        size_name=cycle('Description size: ',size_name,['XS','S','M','L'])
                    toggle('Bordered description');bordered=not bordered
                toggle('Vertical description');vertical=not vertical
            toggle('Refine description slots');paint(theme,True);identity();toggle('Refine description slots')
            if theme==initial_theme:mac.press(TITLE,theme)
        if images:
            focus_gallery_control(mac,'Description value','AXTextField')
            reveal_gallery_control(mac,'Workspace details','AXList',scroll_fraction=.94)
            screenshot(mac,images/'gallery-description.png',title=TITLE)
        toggle('Description value action visible');wait_absent(mac,'Description value action','AXButton');identity(False)
        toggle('Description value action visible')
        replacement=mac.wait_find(TITLE,'Description value action','AXButton')
        assert not equal(value_action,replacement),'removed definition action retained native identity'
        mac.release(value_action);value_action=replacement
        mac.press(TITLE,'Runtime & windows');wait_absent(mac,'Description value','AXTextField')
        mac.press(TITLE,'Presentation')
        replacement=mac.wait_find(TITLE,'Description value','AXTextField')
        try:assert not equal(editor,replacement),'page retained old description editor'
        finally:mac.release(replacement)
        mac.wait_text(TITLE,f'Description actions: {actions}')
        print(f'GALLERY_DESCRIPTION_OK: {cases} packing/axis/width/style cases; {paint_cases} GPU theme/size/axis/border/refinement cases, native Term/Definition parents, {actions} OS actions, rich control/editor retention and slot/page retirement',flush=True)
    finally:
        mac.release(editor);mac.release(term_action);mac.release(value_action)
        temporary.cleanup()


def exercise_chat_composition(mac, images):
    """Native message geometry, reaction hit areas and editor/stream ownership."""
    mac.press(TITLE,'Presentation')
    mac.wait_text(TITLE,'Room for a conversation')
    mac.release(mac.wait_find(TITLE,'Bubble payload','AXGroup'))
    editor=mac.wait_find(TITLE,'Message draft','AXTextField')
    body=mac.wait_find(TITLE,'Message body action','AXButton')
    reaction=mac.wait_find(TITLE,'React to message','AXButton')
    equal=mac.cf.CFEqual
    equal.restype,equal.argtypes=C.c_bool,[C.c_void_p,C.c_void_p]
    mouse=GalleryMouse(mac)
    variant='Secondary'
    variants=['Filled','Secondary','Muted','Tinted','Outline','Ghost','Destructive']
    actions=reactions=cases=0
    typed=True
    temporary=tempfile.TemporaryDirectory(prefix='gpuio-chat-composition-')
    directory=images or Path(temporary.name)
    paint_cases=0

    def toggle(label):
        activate(mac,mac.wait_find(TITLE,label,'AXCheckBox'))
        time.sleep(.1)

    def cycle(prefix,current,values):
        mac.press(TITLE,prefix+current)
        value=values[(values.index(current)+1)%len(values)]
        mac.release(mac.wait_find(TITLE,prefix+value,'AXButton'))
        return value

    def rect(label,role='AXGroup'):
        node=mac.wait_find(TITLE,label,role)
        try: return element_rect(mac,node)
        finally: mac.release(node)

    def identity(check_reaction=True):
        for label,role,old in [('Message draft','AXTextField',editor),('Message body action','AXButton',body)]+([('React to message','AXButton',reaction)] if check_reaction else []):
            node=mac.wait_find(TITLE,label,role)
            try: assert equal(old,node), ('chat control remounted',label)
            finally: mac.release(node)
        expect_field(mac,TITLE,'Message draft','a')

    def counts():
        mac.wait_text(TITLE,f'Message actions: {actions} · reactions: {reactions}')

    def surface():
        x,y,w,h=rect('Bubble payload')
        horizontal,vertical=(0,0) if variant=='Ghost' else (13,9)
        return x-horizontal,y-vertical,w+2*horizontal,h+2*vertical

    def paint(theme):
        nonlocal paint_cases
        reveal_gallery_control(mac,'Bubble payload','AXGroup')
        window=mac.window(TITLE)
        try: wx,wy,ww,wh=element_rect(mac,window)
        finally: mac.release(window)
        mouse.send(5,(wx+ww-25,wy+110))
        time.sleep(.08)
        x,y,w,h=surface()
        assert y>=wy+160 and y+h<=wy+wh-15, ('bubble paint visibility',x,y,w,h)
        path=directory/f'gallery-bubble-paint-{paint_cases:02d}.png'
        screenshot(mac,path,title=TITLE)
        pixels=read_png(mac,path)
        def rgb(px,py):
            return pixels.rgb((px-wx)*pixels.width/ww,(py-wy)*pixels.height/wh)
        dark=theme=='Dark'
        parent=(25,33,44) if dark else (255,255,255)
        accent=(163,181,255) if dark else (64,88,183)
        danger=(255,160,175) if dark else (183,52,75)
        blend=lambda color,alpha:tuple(round(c*alpha+b*(1-alpha)) for c,b in zip(color,parent))
        fill={
            'Filled':accent,
            'Secondary':(39,46,59) if dark else (240,242,246),
            'Muted':(39,46,59) if dark else (240,242,246),
            'Tinted':blend(accent,.12),
            'Outline':(27,32,43) if dark else (255,255,255),
            'Ghost':parent,
            'Destructive':blend(danger,.1)}[variant]
        close=lambda a,b:max(abs(c-d) for c,d in zip(a,b))<=5
        point=(x+w-4,y+12) if variant=='Ghost' else (x+4,y+h/2)
        actual=rgb(*point)
        assert close(actual,fill), ('bubble fill',theme,variant,actual,fill)
        if variant=='Outline':
            border=(62,72,91) if dark else (211,217,227)
            samples=[rgb(x+d,y+h/2) for d in [.25,.5,.75]]
            assert any(close(sample,border) for sample in samples), ('bubble border',theme,samples,border)
        paint_cases+=1

    def geometry(end=False,top=False,start=False,compact=False,bubble_end=None):
        x,y,w,h=rect('Composed message')
        sx,sy,sw,sh=surface()
        bx,by,bw,bh=rect('Composed bubble')
        ax,ay,aw,ah=rect('Message avatar')
        assert abs(w-(360 if compact else 560))<1, ('message width',w)
        expected=(w-40)*(1 if variant=='Ghost' else .8)
        assert abs(bw-expected)<1, ('bubble root width',variant,bw,expected)
        if bubble_end is None:
            assert abs(sw-bw)<1, ('unconstrained surface stretch',sw,bw)
        else:
            assert sw<=bw+1 and abs((sx+sw if bubble_end else sx)-(bx+bw if bubble_end else bx))<1, ('explicit surface alignment',bubble_end,sx,sw,bx,bw)
        assert abs(sy-by)<1 and abs(sh-bh)<1, ('surface/root vertical bounds',(sy,sh),(by,bh))
        assert abs(ay+ah-(sy+sh))<1, ('avatar bottom',ay+ah,sy+sh)
        if end:
            assert abs(ax+aw-(x+w))<1, ('end avatar alignment',ax,aw,x,w)
        else:
            assert abs(ax-x)<1, ('start avatar alignment',ax,x)
        bubble_end=end if bubble_end is None else bubble_end
        assert abs((bx+bw if bubble_end else bx)-(x+w-(40 if end else 0) if bubble_end else x+(0 if end else 40)))<1, ('independent bubble alignment',end,bubble_end,bx,bw,x,w)
        inset=0 if variant=='Ghost' else 12
        for label in ['Message header','Message footer']:
            tx,ty,tw,th=rect(label,'AXStaticText')
            assert abs((tx+tw if end else tx)-(x+w-40-inset if end else x+40+inset))<1, ('metadata inset',label,variant,(tx,tw),x,w,inset)
        rx,ry,rw,rh=element_rect(mac,reaction)
        pad_x,pad_y=(3,3) if typed else (9,5)
        assert abs((rx-pad_x if start else rx+rw+pad_x)-(bx+12 if start else bx+bw-12))<1, ('reaction edge',start,rx,rw,sx,sw)
        assert abs((ry-pad_y if top else ry+rh+pad_y)-(sy-20 if top else sy+sh+20))<1, ('reaction side',top,(ry,rh),(sy,sh))
        return (sx-x,sy-y,sw,sh),(ax-x,ay-y,aw,ah)

    def click_reaction(top=False):
        x,y,w,h=reveal_gallery_control(mac,'React to message','AXButton')
        point=(x+w/2,y+3 if top else y+h-3)
        mouse.check_owner(point)
        mouse.send(5,point);mouse.send(1,point);mouse.send(2,point)

    node=mac.find(TITLE,'Dark','AXButton')
    initial='Dark' if node else 'Light'
    if node:mac.release(node)
    try:
        focus_gallery_control(mac,'Message draft','AXTextField')
        mac.key(0,flags=1<<20);mac.key(0)
        expect_field(mac,TITLE,'Message draft','a')
        for theme in [initial,'Light' if initial=='Dark' else 'Dark']:
            for _ in range(7):
                for end in [False,True]:
                    if end:toggle('Message end aligned')
                    identity()
                    geometry(end=end)
                    if not end:paint(theme)
                    focus_gallery_control(mac,'Message body action','AXButton')
                    mac.key(36);actions+=1;counts()
                    focus_gallery_control(mac,'React to message','AXButton')
                    mac.key(49);reactions+=1;counts()
                    cases+=1
                toggle('Message end aligned')
                variant=cycle('Bubble variant: ',variant,variants)
            if images:
                reveal_gallery_control(mac,'Message body action','AXButton')
                screenshot(mac,images/f'gallery-chat-composition-{theme.lower()}.png',title=TITLE)
            if theme==initial:mac.press(TITLE,theme)
        for typed_case in [True,False]:
            if not typed_case:toggle('Typed reaction action');typed=False
            for top in [False,True]:
                if top:toggle('Top reactions')
                for start in [False,True]:
                    if start:toggle('Start reactions')
                    identity();geometry(top=top,start=start)
                    click_reaction(top);reactions+=1;counts()
                    toggle('Clip bubble overflow')
                    click_reaction(top)
                    time.sleep(.2);counts()
                    toggle('Clip bubble overflow')
                    click_reaction(top);reactions+=1;counts()
                toggle('Start reactions')
            toggle('Top reactions')
        toggle('Typed reaction action');typed=True
        identity()
        edge=cycle('Bubble edge: ','Inherit',['Inherit','Start','End'])
        geometry(bubble_end=False)
        toggle('Message end aligned');geometry(end=True,bubble_end=False);identity()
        edge=cycle('Bubble edge: ',edge,['Inherit','Start','End'])
        geometry(end=True,bubble_end=True)
        toggle('Message end aligned');geometry(bubble_end=True);identity()
        edge=cycle('Bubble edge: ',edge,['Inherit','Start','End'])
        # Footer changes leave the avatar/body row in place, measured relative to root.
        before=geometry()
        toggle('Expand message footer')
        after=geometry()
        assert all(abs(a-b)<1 for old,new in zip(before,after) for a,b in zip(old,new)), ('footer changed row geometry',before,after)
        toggle('Expand message footer')
        toggle('Compact message');geometry(compact=True);identity();toggle('Compact message')
        # Typed Ghost metadata, explicit override and arbitrary-view policy.
        while variant!='Ghost':variant=cycle('Bubble variant: ',variant,variants)
        hx=rect('Message header','AXStaticText')[0]-rect('Composed message')[0]
        inset=cycle('Message inset: ','Inherit',['Inherit','Yes','No'])
        assert abs(rect('Message header','AXStaticText')[0]-rect('Composed message')[0]-hx-12)<1
        inset=cycle('Message inset: ',inset,['Inherit','Yes','No'])
        assert abs(rect('Message header','AXStaticText')[0]-rect('Composed message')[0]-hx)<1
        inset=cycle('Message inset: ',inset,['Inherit','Yes','No'])
        toggle('Typed message bubble')
        assert abs(rect('Message header','AXStaticText')[0]-rect('Composed message')[0]-hx-12)<1
        identity();toggle('Typed message bubble')
        # Streaming Markdown/code changes native flow height, not editor identity/text.
        toggle('Message Markdown')
        mac.release(mac.wait_find(TITLE,'Composed message document','AXGroup'))
        focus_gallery_control(mac,'Message draft','AXTextField')
        for chunk in range(1,4):
            old_h=rect('Bubble payload')[3]
            mac.press(TITLE,'Append message chunk')
            mac.wait_text(TITLE,f'Stream chunks: {chunk}')
            deadline=time.monotonic()+10
            while rect('Bubble payload')[3] <= old_h+5:
                if time.monotonic()>deadline:raise RuntimeError('streamed Markdown did not grow')
                time.sleep(.05)
            identity();expect_focus(mac,'Message draft','AXTextField')
            geometry()
        toggle('Show message reactions')
        wait_absent(mac,'React to message','AXButton');identity(check_reaction=False)
        toggle('Show message reactions')
        replacement=mac.wait_find(TITLE,'React to message','AXButton')
        assert not equal(reaction,replacement)
        mac.release(reaction);reaction=replacement
        for control,label,role in [('Show message avatar','Message avatar','AXGroup'),('Show message header','Message header','AXStaticText'),('Show message footer','Message footer','AXStaticText')]:
            toggle(control);wait_absent(mac,label,role);identity();toggle(control)
        mac.press(TITLE,'Runtime & windows')
        wait_absent(mac,'Message draft','AXTextField')
        mac.press(TITLE,'Refresh resource counts')
        mac.wait_text(TITLE,'Registered source bytes: 0')
        mac.press(TITLE,'Presentation')
        replacement=mac.wait_find(TITLE,'Message body action','AXButton')
        assert not equal(body,replacement)
        mac.release(body);body=replacement
        counts();mac.wait_text(TITLE,'Stream chunks: 0')
        print(f'GALLERY_CHAT_COMPOSITION_OK: {cases} theme/variant/alignment cases; {paint_cases} GPU surfaces; {actions} body and {reactions} reaction actions; avatar/footer geometry, native editor retention, reaction clipping, Ghost insets, Markdown streaming and page retirement',flush=True)
    finally:
        mac.release(editor);mac.release(body);mac.release(reaction)
        temporary.cleanup()


def exercise_chat_list(mac, images):
    """Public managed rows: measured reaction spacing, anchor and streamed growth."""
    mac.press(TITLE,'Presentation')
    mac.wait_text(TITLE,'A conversation in motion')
    raise_gallery(mac)
    time.sleep(.2)  # Allow AppKit's asynchronous activation before the first wheel.
    focus_gallery_control(mac,'Transcript draft','AXTextField')
    # Native focus first reveals the distant card. Send subsequent reveal wheels
    # beside the nested viewport, so they reach the page instead of the rows.
    reveal_gallery_control(mac,'Managed conversation','AXList',scroll_fraction=.94)
    reveal_gallery_control(mac,'Transcript draft','AXTextField',scroll_fraction=.94)
    mac.key(0,flags=1<<20);mac.key(0)
    editor=mac.wait_find(TITLE,'Transcript draft','AXTextField')
    equal=mac.cf.CFEqual
    equal.restype,equal.argtypes=C.c_bool,[C.c_void_p,C.c_void_p]
    mouse=GalleryMouse(mac)
    retained=[]

    def rect(label,role='AXGroup'):
        node=mac.wait_find(TITLE,label,role)
        try:return element_rect(mac,node)
        finally:mac.release(node)

    def identity():
        current=mac.wait_find(TITLE,'Transcript draft','AXTextField')
        try:assert equal(current,editor),'Managed row work recreated the outside draft'
        finally:mac.release(current)
        expect_field(mac,TITLE,'Transcript draft','a')
        status=mac.wait_find(TITLE,'Managed chunks:',contains=True)
        try:
            values,_children=mac.node_values(status)
            try:
                match=re.search(r'retained rows: (\d+)/12', ' '.join(value or '' for value in values))
                assert match and 0<int(match[1])<=12, ('managed row budget',values)
            finally:
                for child in _children:mac.release(child)
        finally:mac.release(status)

    def append(chunk):
        mac.press(TITLE,'Append transcript chunk')
        mac.wait_text(TITLE,f'Managed chunks: {chunk} ·')

    try:
        mac.press(TITLE,'Transcript latest')
        mac.release(mac.wait_find(TITLE,'Managed streamed answer','AXGroup'))
        reveal_gallery_control(mac,'Managed conversation','AXList',scroll_fraction=.94)
        preceding=mac.wait_find(TITLE,'Managed message 99','AXGroup')
        retained.append(preceding)
        # Read retained AX objects directly while preparation/layout changes; a
        # whole-window traversal between samples could miss a brief rebound.
        samples=[]
        append_button=mac.wait_find(TITLE,'Append transcript chunk','AXButton')
        retained.append(append_button)
        for _ in range(3):
            samples.append((element_rect(mac,preceding)[1],element_rect(mac,editor)[1]))
            mac.perform(append_button,'AXPress')
            until=time.monotonic()+.45
            while time.monotonic()<until:
                samples.append((element_rect(mac,preceding)[1],element_rect(mac,editor)[1]))
                time.sleep(.01)
        mac.wait_text(TITLE,'Managed chunks: 3 ·')
        deltas=[b[0]-a[0] for a,b in zip(samples,samples[1:])]
        assert not [d for d in deltas if d>1],('stream moved preceding row down',samples)
        assert sum(d< -1 for d in deltas)>=2,('too few observed growth steps',samples)
        assert max(y for _,y in samples)-min(y for _,y in samples)<.5,('outside draft moved',samples)
        identity();expect_focus(mac,'Transcript draft','AXTextField')
        mac.press(TITLE,'Transcript history')
        row=mac.wait_find(TITLE,'Managed message 50','AXGroup')
        wait_absent(mac,'Managed streamed answer','AXGroup')
        action=mac.wait_find(TITLE,'React to row 51','AXButton')
        retained.extend([row,action])
        time.sleep(.15)
        before=element_rect(mac,row)
        for chunk in range(4,7):
            append(chunk)
            time.sleep(.1)
            after=element_rect(mac,row)
            assert all(abs(a-b)<.5 for a,b in zip(before,after)),('stream moved paused history',before,after)
            identity();expect_focus(mac,'Transcript draft','AXTextField')
        # Absolute controls fit within caller-reserved measured row padding.
        rx,ry,rw,rh=element_rect(mac,action)
        mx,my,mw,mh=rect('Managed message 51')
        nx,ny,nw,nh=rect('Managed message 52')
        assert ry+rh>my+mh and ry+rh+4<=ny,('reaction spacing',(ry,rh),(my,mh),ny)
        point=(rx+rw/2,ry+rh-3)
        mouse.check_owner(point)
        mouse.send(5,point);mouse.send(1,point);mouse.send(2,point)
        mac.wait_text(TITLE,'/12 · reactions: 1')
        gx,gy,gw,gh=rect('Managed conversation','AXList')
        point=(gx+gw/2,gy+gh/2)
        create=mac.cg.CGEventCreateScrollWheelEvent
        create.restype,create.argtypes=C.c_void_p,[C.c_void_p,C.c_uint,C.c_uint,C.c_int]
        locate=mac.cg.CGEventSetLocation
        locate.restype,locate.argtypes=None,[C.c_void_p,GalleryMouse.Point]
        for delta in [-1]*8+[1]*8:
            old_y=element_rect(mac,row)[1]
            old_viewport=rect('Managed conversation','AXList')
            mouse.check_owner(point);mouse.send(5,point)
            event=create(None,0,1,delta)
            assert event
            try:locate(event,GalleryMouse.Point(*point));mouse.post(0,event)
            finally:mac.release(event)
            time.sleep(.08)
            current=mac.wait_find(TITLE,'React to row 51','AXButton')
            try:assert equal(action,current),'small scroll replaced warm reaction control'
            finally:mac.release(current)
            new_y=element_rect(mac,row)[1]
            new_viewport=rect('Managed conversation','AXList')
            assert abs((new_y-old_y)-delta)<.75,('small scroll geometry',delta,old_y,new_y,old_viewport,new_viewport)
            assert all(abs(a-b)<.5 for a,b in zip(old_viewport,new_viewport)),('list wheel moved outer page',old_viewport,new_viewport)
        assert abs(element_rect(mac,row)[1]-before[1])<.75,'reverse scroll did not restore anchor'
        mac.press(TITLE,'Transcript latest')
        mac.wait_text(TITLE,'Update 6')
        identity()
        if images:
            reveal_gallery_control(mac,'Managed conversation','AXList',scroll_fraction=.94)
            screenshot(mac,images/'gallery-managed-chat.png',title=TITLE)
        mac.press(TITLE,'Runtime & windows')
        wait_absent(mac,'Transcript draft','AXTextField')
        mac.press(TITLE,'Refresh resource counts')
        mac.wait_text(TITLE,'Registered source bytes: 0')
        mac.press(TITLE,'Presentation')
        mac.wait_text(TITLE,'Managed chunks: 0 ·')
        replacement=mac.wait_find(TITLE,'Transcript draft','AXTextField')
        try:assert not equal(editor,replacement),'page retained old native draft owner'
        finally:mac.release(replacement)
        print(f'GALLERY_CHAT_LIST_OK: 100 logical / max12 managed rows; {len(samples)} stream samples, monotonic growth/stable draft; paused history anchor, outside-bubble reaction spacing/action, 16 one-pixel scrolls and warm identity; offscreen source/remount and page cleanup',flush=True)
    finally:
        mac.release(editor)
        for node in retained:mac.release(node)


def exercise_tags(mac, images):
    """Rich tags: real hover compositing, child keys, native paint and OS actions."""
    mac.press(TITLE,'Presentation')
    mac.wait_text(TITLE,'Small details, useful actions')
    action=mac.wait_find(TITLE,'Tag action','AXButton')
    equal=mac.cf.CFEqual
    equal.restype,equal.argtypes=C.c_bool,[C.c_void_p,C.c_void_p]
    mouse=GalleryMouse(mac)
    temporary=tempfile.TemporaryDirectory(prefix='gpuio-tag-')
    directory=images or Path(temporary.name)
    serial=actions=cases=0
    variant,size,hover_mode='Secondary','M','Default'
    variants=['Primary','Secondary','Danger','Success','Warning','Info','Custom']

    def toggle(label):
        activate(mac,mac.wait_find(TITLE,label,'AXCheckBox'))
        time.sleep(.08)

    def cycle(prefix,current,values):
        mac.press(TITLE,prefix+current)
        value=values[(values.index(current)+1)%len(values)]
        mac.release(mac.wait_find(TITLE,prefix+value,'AXButton'))
        return value

    def identity():
        node=mac.wait_find(TITLE,'Tag action','AXButton')
        try: assert equal(action,node), 'Tag reconfiguration remounted action'
        finally: mac.release(node)

    def root_rect():
        node=mac.wait_find(TITLE,'Tag preview','AXGroup')
        try: return element_rect(mac,node)
        finally: mac.release(node)

    def window_rect():
        node=mac.window(TITLE)
        try: return element_rect(mac,node)
        finally: mac.release(node)

    def reveal():
        # The nested action can be visible while the tag's bottom border is
        # clipped. Pixel checks need the complete tag, not just its focus leaf.
        reveal_gallery_control(mac,'Tag preview','AXGroup')
        wx,wy,ww,wh=window_rect()
        mouse.send(5,(wx+ww-25,wy+110))
        time.sleep(.08)

    def paint(theme,outline,opacity=1.,hover=False):
        nonlocal serial
        x,y,w,h=root_rect()
        wx,wy,ww,wh=window_rect()
        assert abs(w-280)<1 and y>=wy+160 and y+h<wy+wh-15, ('tag bounds',x,y,w,h)
        if hover:
            point=(x+w-5,y+h/2)
            mouse.check_owner(point)
            mouse.send(5,point)
            time.sleep(.15)
        path=directory/f'gallery-tag-paint-{serial:03d}.png'
        serial+=1
        screenshot(mac,path,title=TITLE)
        pixels=read_png(mac,path)
        def rgb(px,py):
            return pixels.rgb((px-wx)*pixels.width/ww,(py-wy)*pixels.height/wh)
        dark=theme=='Dark'
        parent=(25,33,44) if dark else (255,255,255)
        accent=(163,181,255) if dark else (64,88,183)
        bg,border={
            'Primary':(accent,accent), 'Info':(accent,accent),
            'Secondary':((39,46,59),(62,72,91)) if dark else ((240,242,246),(211,217,227)),
            'Danger':((255,160,175),(255,160,175)) if dark else ((183,52,75),(183,52,75)),
            'Success':((139,214,175),(139,214,175)) if dark else ((33,115,76),(33,115,76)),
            'Warning':((241,199,132),(241,199,132)) if dark else ((135,85,11),(135,85,11)),
            'Custom':((16,21,29),(137,221,201)) if dark else ((241,244,247),(9,110,91))}[variant]
        def blend(color): return tuple(round(c*opacity+b*(1-opacity)) for c,b in zip(color,parent))
        background=parent if outline else blend(bg)
        edge=tuple(round(c*opacity+b*(1-opacity)) for c,b in zip(border,background))
        close=lambda a,b:max(abs(x-y) for x,y in zip(a,b))<=5
        actual=rgb(x+w-5,y+h/2)
        assert close(actual,background), ('tag fill/hover',theme,variant,outline,opacity,actual,background)
        edges=[rgb(x+w/2,y+h-d) for d in [.25,.5,.75]]
        assert any(close(c,edge) for c in edges), ('tag border/hover',theme,variant,outline,opacity,edges,edge)
        return actual

    node=mac.find(TITLE,'Dark','AXButton')
    initial='Dark' if node else 'Light'
    if node: mac.release(node)
    try:
        for theme in [initial,'Light' if initial=='Dark' else 'Dark']:
            for _ in range(7):
                focus_gallery_control(mac,'Tag action','AXButton')
                for outline in [False,True]:
                    if outline: toggle('Tag outline')
                    reveal()
                    identity()
                    paint(theme,outline)
                    expect_focus(mac,'Tag action','AXButton')
                    mac.key(36)
                    actions+=1
                    mac.wait_text(TITLE,f'Tag actions: {actions}')
                    cases+=1
                toggle('Tag outline')
                variant=cycle('Tag variant: ',variant,variants)
            sizes={}
            for _ in range(4):
                reveal()
                sizes[size]=root_rect()[3]
                identity()
                size=cycle('Tag size: ',size,['XS','S','M','L'])
            assert abs(sizes['XS']-sizes['S'])<.1 and abs(sizes['M']-sizes['L'])<.1 and abs(sizes['M']-sizes['S']-4)<.2, sizes
            # Use a strongly contrasting palette for observable hover alpha.
            while variant!='Primary': variant=cycle('Tag variant: ',variant,variants)
            for mode,rest,active in [('Default',1.,.9),('Override',.65,.4),('Unset',.65,.65)]:
                assert hover_mode==mode
                reveal()
                before=paint(theme,False,rest)
                after=paint(theme,False,active,hover=True)
                if mode=='Unset': assert before==after, 'Unset hover changed base opacity'
                else: assert max(abs(a-b) for a,b in zip(before,after))>5, 'hover was not observable'
                identity()
                hover_mode=cycle('Tag hover: ',hover_mode,['Default','Override','Unset'])
            toggle('Round tag corners')
            toggle('Reverse tag children')
            reveal()
            identity()
            label=mac.wait_find(TITLE,'Ready · 京都','AXStaticText')
            try:
                assert element_rect(mac,action)[0] < element_rect(mac,label)[0], 'reverse did not change order'
            finally: mac.release(label)
            if images: screenshot(mac,images/f'gallery-tag-{theme.lower()}.png',title=TITLE)
            toggle('Reverse tag children')
            toggle('Round tag corners')
            # Return to the same variant for the next full matrix.
            while variant!='Secondary': variant=cycle('Tag variant: ',variant,variants)
            if theme==initial: mac.press(TITLE,theme)
        toggle('Tag label')
        wait_absent(mac,'Ready · 京都','AXStaticText')
        identity()
        focus_gallery_control(mac,'Tag action','AXButton')
        mac.key(49)
        actions+=1
        mac.wait_text(TITLE,f'Tag actions: {actions}')
        toggle('Tag action slot')
        wait_absent(mac,'Tag action','AXButton')
        root=mac.wait_find(TITLE,'Tag preview','AXGroup')
        children=mac.children(root,'AXChildren')
        try: assert not children, 'empty tag inserted placeholder accessibility nodes'
        finally:
            for child in children: mac.release(child)
            mac.release(root)
        toggle('Tag action slot')
        replacement=mac.wait_find(TITLE,'Tag action','AXButton')
        assert not equal(action,replacement), 'removed action retained native identity'
        mac.release(action)
        action=replacement
        toggle('Tag label')
        mac.press(TITLE,'Runtime & windows')
        wait_absent(mac,'Tag action','AXButton')
        mac.press(TITLE,'Refresh resource counts')
        mac.wait_text(TITLE,'Registered source bytes: 0')
        mac.press(TITLE,'Presentation')
        replacement=mac.wait_find(TITLE,'Tag action','AXButton')
        assert not equal(action,replacement), 'page retained native action'
        mac.release(action)
        action=replacement
        mac.wait_text(TITLE,f'Tag actions: {actions}')
        focus_gallery_control(mac,'Tag action','AXButton')
        mac.key(36)
        actions+=1
        mac.wait_text(TITLE,f'Tag actions: {actions}')
        print(f'GALLERY_TAG_OK: {cases} theme/variant/outline GPU cases, two size groups, native hover/override/unset pixels, reorder and rich-only/empty content, {actions} OS actions, retained focus/identity, slot/page retirement and state recovery',flush=True)
    finally:
        mac.release(action)
        temporary.cleanup()


def exercise_alerts(mac, images):
    """Rich Alert public controls, native geometry, OS activation and dismissal."""
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A clear next step')
    action = mac.wait_find(TITLE, 'Alert action', 'AXButton')
    close = mac.wait_find(TITLE, 'Dismiss alert', 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    mouse = GalleryMouse(mac)
    temporary = tempfile.TemporaryDirectory(prefix="gpuio-alert-")
    directory = images or Path(temporary.name)
    serial = 0
    actions = cases = 0
    variant, size, icon = 'Default', 'M', 'Default'
    title = 'A small interruption, a clear next step'
    message = 'Your workspace is safe. Review the details, then continue where you left off. 京都'

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.08)

    def identity():
        for label, retained in [('Alert action', action), ('Dismiss alert', close)]:
            node = mac.wait_find(TITLE, label, 'AXButton')
            try:
                assert equal(retained, node), f'{label} remounted during reconfiguration'
            finally:
                mac.release(node)

    def cycle(prefix, current, values):
        mac.press(TITLE, prefix + current)
        value = values[(values.index(current) + 1) % len(values)]
        mac.release(mac.wait_find(TITLE, prefix + value, 'AXButton'))
        return value

    def root_rect():
        root = mac.wait_find(TITLE, 'Alert preview', 'AXGroup')
        try:
            return element_rect(mac, root)
        finally:
            mac.release(root)

    def geometry(banner, compact=False):
        reveal_gallery_control(mac, 'Alert action', 'AXButton')
        x,y,w,h = root_rect()
        assert abs(w-(350 if compact else 560)) < 1, ('alert width',w)
        body = mac.wait_find(TITLE, message, 'AXStaticText')
        try:
            bx,by,bw,bh = element_rect(mac, body)
        finally:
            mac.release(body)
        ax,ay,aw,ah = element_rect(mac,action)
        cx,cy,cw,ch = element_rect(mac,close)
        assert x <= bx and bx+bw <= cx+1 and by >= y and by+bh <= y+h+1, ('body clipped/overlap', (x,y,w,h),(bx,by,bw,bh),(cx,cy,cw,ch))
        assert ay >= by+bh-1 and ax+aw <= cx+1 and ay+ah <= y+h+1, 'rich action escaped body'
        assert cx+cw <= x+w+1 and cy >= y, 'close outside root'
        if banner:
            absent(mac,title,'AXStaticText')
        else:
            title_node=mac.wait_find(TITLE,title,'AXStaticText')
            try:
                _,_,tw,th=element_rect(mac,title_node)
                assert th <= 22 and tw <= w, ('title failed single-line truncation',tw,th)
            finally:
                mac.release(title_node)
        return h

    def paint(theme, variant, refined=False):
        nonlocal serial
        x,y,w,h=root_rect()
        window=mac.window(TITLE)
        try:
            wx,wy,ww,wh=element_rect(mac,window)
        finally:
            mac.release(window)
        # The left padding and vertical edge contain no glyphs or controls.
        assert y+h/2 > wy+160 and y+h/2 < wy+wh-20
        path=directory/f'gallery-alert-paint-{serial:03d}.png'
        serial+=1
        screenshot(mac,path,title=TITLE)
        pixels=read_png(mac,path)
        def rgb(offset):
            return pixels.rgb((x+offset-wx)*pixels.width/ww,(y+h/2-wy)*pixels.height/wh)
        dark=theme=='Dark'
        parent=(25,33,44) if dark else (255,255,255)
        surface=(27,32,43) if dark else (255,255,255)
        border=(62,72,91) if dark else (211,217,227)
        accents={'Info':(163,181,255) if dark else (64,88,183),
                 'Success':(139,214,175) if dark else (33,115,76),
                 'Warning':(241,199,132) if dark else (135,85,11),
                 'Error':(255,160,175) if dark else (183,52,75)}
        def mix(ink,alpha,base):
            return tuple(round(a*alpha+b*(1-alpha)) for a,b in zip(ink,base))
        if variant=='Default':
            background=surface
        else:
            background=mix(accents[variant],10/255,parent)
            border=mix(accents[variant],77/255,background)
        if refined:
            border=(137,221,201) if dark else (9,110,91)
        close=lambda a,b:max(abs(x-y) for x,y in zip(a,b))<=5
        assert close(rgb(6),background), ('alert background tint',theme,variant,rgb(6),background)
        edge=[rgb(o) for o in [.25,.5,.75]]
        assert any(close(c,border) for c in edge), ('alert border tint',theme,variant,edge,border)

    def click_close():
        x,y,w,h = reveal_gallery_control(mac, 'Dismiss alert', 'AXButton')
        point=(x+w/2,y+h/2)
        mouse.check_owner(point)
        mouse.send(5,point)
        mouse.send(1,point)
        mouse.send(2,point)

    node=mac.find(TITLE,'Dark','AXButton')
    initial='Dark' if node else 'Light'
    if node: mac.release(node)
    try:
        for theme in [initial, 'Light' if initial=='Dark' else 'Dark']:
            for _ in range(5):
                focus_gallery_control(mac,'Alert action','AXButton')
                for banner in [False,True]:
                    if banner: toggle('Alert banner')
                    identity()
                    geometry(banner)
                    paint(theme,variant)
                    expect_focus(mac,'Alert action','AXButton')
                    mac.key(36)
                    actions+=1
                    mac.wait_text(TITLE,f'Alert actions: {actions}')
                    cases+=1
                toggle('Alert banner')
                variant=cycle('Alert variant: ',variant,['Default','Info','Success','Warning','Error'])
            heights={}
            for _ in range(4):
                heights[size]=geometry(False)
                identity()
                size=cycle('Alert size: ',size,['XS','S','M','L'])
            assert heights['XS'] < heights['S'] < heights['M'] < heights['L'], heights
            for _ in range(3):
                icon=cycle('Alert icon: ',icon,['Default','Custom','Hidden'])
                identity()
                geometry(False)
            toggle('Compact alert')
            compact_h=geometry(False,compact=True)
            toggle('Compact alert')
            assert compact_h > geometry(False), 'narrow body did not wrap/grow'
            toggle('Refine alert styles')
            geometry(False)
            paint(theme,variant,refined=True)
            identity()
            if images:
                screenshot(mac, images / f'gallery-alert-{theme.lower()}.png', title=TITLE)
            toggle('Refine alert styles')
            if theme==initial:
                mac.press(TITLE,theme)
        toggle('Alert title')
        absent(mac,title,'AXStaticText')
        identity()
        toggle('Alert title')
        mac.release(mac.wait_find(TITLE,title,'AXStaticText'))
        toggle('Disable alert close')
        identity()
        click_close()
        mac.wait_text(TITLE,'Alert dismissals: 0')
        identity()
        toggle('Disable alert close')
        # Removing only the close slot must keep the body action alive.
        toggle('Alert close control')
        wait_absent(mac,'Dismiss alert','AXButton')
        node=mac.wait_find(TITLE,'Alert action','AXButton')
        try: assert equal(node,action)
        finally: mac.release(node)
        toggle('Alert close control')
        replacement=mac.wait_find(TITLE,'Dismiss alert','AXButton')
        assert not equal(close,replacement), 'removed close retained native identity'
        mac.release(close)
        close=replacement
        focus_gallery_control(mac,'Dismiss alert','AXButton')
        mac.key(49)
        mac.wait_text(TITLE,'Alert dismissals: 1')
        wait_absent(mac,'Alert action','AXButton')
        absent(mac,title,'AXStaticText')
        absent(mac,'Alert preview','AXGroup')
        mac.press(TITLE,'Restore alert')
        replacement=mac.wait_find(TITLE,'Alert action','AXButton')
        assert not equal(action,replacement), 'hidden alert retained native action'
        mac.release(action)
        action=replacement
        replacement=mac.wait_find(TITLE,'Dismiss alert','AXButton')
        assert not equal(close,replacement), 'hidden alert retained close action'
        mac.release(close)
        close=replacement
        mac.wait_text(TITLE,f'Alert actions: {actions}')
        click_close()
        mac.wait_text(TITLE,'Alert dismissals: 2')
        wait_absent(mac,'Alert action','AXButton')
        mac.press(TITLE,'Restore alert')
        mac.release(mac.wait_find(TITLE,'Alert action','AXButton'))
        mac.press(TITLE,'Runtime & windows')
        wait_absent(mac,'Alert action','AXButton')
        mac.press(TITLE,'Refresh resource counts')
        mac.wait_text(TITLE,'Registered source bytes: 0')
        mac.press(TITLE,'Presentation')
        mac.wait_text(TITLE,f'Alert actions: {actions}')
        focus_gallery_control(mac,'Alert action','AXButton')
        mac.key(36)
        actions+=1
        mac.wait_text(TITLE,f'Alert actions: {actions}')
        print(f'GALLERY_ALERT_OK: {cases} theme/variant/banner cases, eight size layouts, custom/hidden icons, wrapping/styles, tint/border pixels, {actions} OS body actions, disabled close, keyboard/pointer dismissal, slot/hide/page retirement and retained caller state',flush=True)
    finally:
        mac.release(action)
        mac.release(close)
        temporary.cleanup()


def exercise_markers(mac, images):
    """Public composition, OS activation, retained controls and native glyph/pulse paint."""
    mac.press(TITLE, 'Motion & rhythm')
    mac.press(TITLE, 'Use full motion')
    mac.wait_text(TITLE, 'Motion preference: Full')
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'Signals that stay out of the way')
    action = mac.wait_find(TITLE, 'Marker action', 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    mouse = GalleryMouse(mac)
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-marker-')
    directory = images or Path(temporary.name)
    serial = actions = cases = 0
    variant, loading_style, icon = 'Plain', 'Spinner', 'None'

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.08)

    def identity():
        node = mac.wait_find(TITLE, 'Marker action', 'AXButton')
        try:
            assert equal(action, node), 'Marker reconfiguration remounted its action'
        finally:
            mac.release(node)

    def reveal():
        reveal_gallery_control(mac, 'Marker action', 'AXButton')
        window = mac.window(TITLE)
        try:
            wx,wy,ww,wh = element_rect(mac,window)
        finally:
            mac.release(window)
        mouse.send(5,(wx+ww-25,wy+110))
        time.sleep(.08)

    def capture(nodes):
        nonlocal serial
        window = mac.window(TITLE)
        try:
            wx,wy,ww,wh = element_rect(mac,window)
        finally:
            mac.release(window)
        rectangles = [element_rect(mac,node) for node in nodes]
        path = directory / f'gallery-marker-{serial:03d}.png'
        serial += 1
        screenshot(mac,path,title=TITLE)
        pixels = read_png(mac,path)
        result=[]
        for x,y,w,h in rectangles:
            assert wx<=x and wy<y and x+w<=wx+ww and y+h<wy+wh, ('marker sample clipped',x,y,w,h)
            result.append(((x,y,w,h),tuple(pixels.rgb((x-wx+(i+.5)*w/120)*pixels.width/ww,(y-wy+(j+.5)*h/24)*pixels.height/wh) for j in range(24) for i in range(120))))
        return result

    def difference(a,b):
        assert all(abs(x-y)<.2 for x,y in zip(a[0],b[0])), ('animation changed marker layout',a[0],b[0])
        return sum(max(abs(x-y) for x,y in zip(p,q))>5 for p,q in zip(a[1],b[1]))

    def paint_check(animated_label, static_labels, reveal_label=None):
        nodes=[mac.wait_find(TITLE,label,role) for label,role in ([animated_label] if animated_label else [])+static_labels]
        try:
            if reveal_label:
                reveal_gallery_control(mac,*reveal_label)
            else:
                reveal()
            before=capture(nodes)
            most=0
            deadline=time.monotonic()+(4 if animated_label else .55)
            while time.monotonic()<deadline:
                time.sleep(.17)
                after=capture(nodes)
                start=1 if animated_label else 0
                for i in range(start,len(nodes)):
                    assert difference(before[i],after[i])==0, 'Loading affected static rich/root content'
                if animated_label:
                    most=max(most,difference(before[0],after[0]))
                    if most>=8: break
            if animated_label:
                assert most>=8, ('No marker animation',animated_label,most)
                print('GALLERY_MARKER_PIXELS',animated_label[0],most,flush=True)
        finally:
            for node in nodes: mac.release(node)

    def variant_paint(theme, variant, refined=False):
        nonlocal serial
        reveal()
        root = mac.wait_find(TITLE,'Marker preview','AXGroup')
        window = mac.window(TITLE)
        try:
            x,y,w,h = element_rect(mac,root)
            wx,wy,ww,wh = element_rect(mac,window)
        finally:
            mac.release(root)
            mac.release(window)
        assert abs(w-532)<1 and h>=42, ('marker full-width/min-height',w,h)
        path=directory/f'gallery-marker-{serial:03d}.png'
        serial+=1
        screenshot(mac,path,title=TITLE)
        pixels=read_png(mac,path)
        def rgb(py):
            return pixels.rgb((x+w-5-wx)*pixels.width/ww,(py-wy)*pixels.height/wh)
        surface=(25,33,44) if theme=='Dark' else (255,255,255)
        border=(62,72,91) if theme=='Dark' else (211,217,227)
        accent=(137,221,201) if theme=='Dark' else (9,110,91)
        close=lambda a,b:max(abs(x-y) for x,y in zip(a,b))<=5
        center=[rgb(y+h/2+delta) for delta in [-.5,-.25,0,.25,.5]]
        bottom=[rgb(y+h-delta) for delta in [.25,.5,.75]]
        if variant=='Separator':
            assert any(close(c,accent if refined else border) for c in center), ('separator not painted',center)
        else:
            assert all(close(c,surface) for c in center), ('unexpected divider',variant,center)
        if variant=='Border':
            assert any(close(c,border) for c in bottom), ('bottom border not painted',bottom)
        else:
            assert all(close(c,surface) for c in bottom), ('unexpected bottom border',variant,bottom)

    def cycle(prefix,current,values):
        mac.press(TITLE,prefix+current)
        value=values[(values.index(current)+1)%len(values)]
        mac.release(mac.wait_find(TITLE,prefix+value,'AXButton'))
        return value

    theme_node=mac.find(TITLE,'Dark','AXButton')
    initial='Dark' if theme_node else 'Light'
    if theme_node: mac.release(theme_node)
    try:
        for theme in [initial,'Light' if initial=='Dark' else 'Dark']:
            for _ in range(3):
                variant_paint(theme,variant)
                focus_gallery_control(mac,'Marker action','AXButton')
                toggle('Marker busy')
                for _ in range(3):
                    identity()
                    if icon=='None': mac.release(mac.wait_find(TITLE,'Marker activity','AXProgressIndicator'))
                    else: absent(mac,'Marker activity','AXProgressIndicator')
                    expect_focus(mac,'Marker action','AXButton')
                    mac.key(36)
                    actions+=1
                    mac.wait_text(TITLE,f'Marker actions: {actions}')
                    icon=cycle('Marker icon: ',icon,['None','Custom','Empty'])
                    cases+=1
                toggle('Marker busy')
                absent(mac,'Marker activity','AXProgressIndicator')
                variant=cycle('Marker variant: ',variant,['Plain','Separator','Border'])
            loading_style=cycle('Marker loading: ',loading_style,['Spinner','Shimmer'])
            toggle('Marker busy')
            absent(mac,'Marker activity','AXProgressIndicator')
            paint_check(('Thinking · 京都','AXStaticText'),[('Marker action','AXButton'),('Steady','AXStaticText')])
            toggle('Marker typed text')
            paint_check(('Marker action','AXButton'),[('Steady','AXStaticText')])
            toggle('Refine marker styles')
            paint_check(('Marker action','AXButton'),[('Steady','AXStaticText')])
            variant=cycle('Marker variant: ',variant,['Plain','Separator','Border'])
            variant_paint(theme,variant,refined=True)
            variant=cycle('Marker variant: ',variant,['Plain','Separator','Border'])
            variant=cycle('Marker variant: ',variant,['Plain','Separator','Border'])
            toggle('Marker typed text')
            toggle('Empty marker text')
            paint_check(None,[('Marker action','AXButton'),('Steady','AXStaticText')])
            toggle('Empty marker text')
            toggle('Marker busy')
            paint_check(None,[('Thinking · 京都','AXStaticText'),('Marker action','AXButton'),('Steady','AXStaticText')])
            toggle('Compact marker')
            reveal()
            identity()
            focus_gallery_control(mac,'Marker action','AXButton')
            mac.key(49)
            actions+=1
            mac.wait_text(TITLE,f'Marker actions: {actions}')
            toggle('Compact marker')
            toggle('Refine marker styles')
            loading_style=cycle('Marker loading: ',loading_style,['Spinner','Shimmer'])
            if theme==initial:
                mac.press(TITLE,theme)
        # Reduced-motion branch is driven by the actual native application policy.
        mac.press(TITLE,'Motion & rhythm')
        mac.press(TITLE,'Use reduced motion')
        mac.wait_text(TITLE,'Motion preference: Reduced')
        mac.press(TITLE,'Presentation')
        mac.wait_text(TITLE,'Signals that stay out of the way')
        restored=mac.wait_find(TITLE,'Marker action','AXButton')
        assert not equal(action,restored), 'Page teardown retained the native action'
        mac.release(action)
        action=restored
        loading_style=cycle('Marker loading: ',loading_style,['Spinner','Shimmer'])
        toggle('Marker busy')
        paint_check(None,[('Thinking · 京都','AXStaticText'),('Marker action','AXButton'),('Steady','AXStaticText')])
        toggle('Marker typed text')
        paint_check(None,[('Marker action','AXButton'),('Steady','AXStaticText')])
        identity()
        mac.press(TITLE,'Runtime & windows')
        mac.press(TITLE,'Refresh resource counts')
        mac.wait_text(TITLE,'Registered source bytes: 0')
        absent(mac,'Marker action','AXButton')
        mac.press(TITLE,'Motion & rhythm')
        mac.press(TITLE,'Use full motion')
        mac.wait_text(TITLE,'Motion preference: Full')
        mac.press(TITLE,'Presentation')
        replacement=mac.wait_find(TITLE,'Marker action','AXButton')
        assert not equal(action,replacement), 'Completed page retained the native action'
        mac.release(action)
        action=replacement
        paint_check(('Marker action','AXButton'),[('Steady','AXStaticText')])
        toggle('Marker busy')
        toggle('Marker typed text')
        loading_style=cycle('Marker loading: ',loading_style,['Spinner','Shimmer'])
        mac.wait_text(TITLE,f'Marker actions: {actions}')
        # Removing the rich slot intentionally retires its action; the surviving
        # typed text still shimmers with the same content owner and layout policy.
        toggle('Marker rich content')
        absent(mac,'Marker action','AXButton')
        loading_style=cycle('Marker loading: ',loading_style,['Spinner','Shimmer'])
        toggle('Marker busy')
        paint_check(('Thinking · 京都','AXStaticText'),[('Steady','AXStaticText')],reveal_label=('Thinking · 京都','AXStaticText'))
        toggle('Marker busy')
        toggle('Marker rich content')
        replacement=mac.wait_find(TITLE,'Marker action','AXButton')
        assert not equal(action,replacement), 'Removed rich slot retained native action'
        mac.release(action)
        action=replacement
        loading_style=cycle('Marker loading: ',loading_style,['Spinner','Shimmer'])
        reveal()
        focus_gallery_control(mac,'Marker action','AXButton')
        mac.key(36)
        actions+=1
        mac.wait_text(TITLE,f'Marker actions: {actions}')
        print(f'GALLERY_MARKER_OK: {cases} theme/variant/icon cases, {actions} OS Return/Space actions, retained identity/focus, typed and empty-text policy, text-only/mixed/rich/static GPU paint, divider/border pixels, opacity refinement, compact layout, native reduced/full recovery and page/slot retirement',flush=True)
    finally:
        mac.release(action)
        temporary.cleanup()


def exercise_shimmer(mac, images):
    """Normal public OCaml application: native keys/AX and captured glyph pixels."""
    mac.press(TITLE, 'Motion & rhythm')
    mac.press(TITLE, 'Use full motion')
    mac.wait_text(TITLE, 'Motion preference: Full')
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A little light, in motion')
    raise_gallery(mac)
    revision = 1
    source = lambda: f'Connecting ideas · {revision}\nAé世界 · é · 👩‍💻'
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    original = mac.wait_find(TITLE, source(), 'AXStaticText')
    clipboard_env = dict(os.environ, LANG='en_US.UTF-8', LC_ALL='en_US.UTF-8')
    saved = subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=clipboard_env).stdout
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-shimmer-captures-')
    directory = images or Path(temporary.name)
    capture_index = 0

    def identity():
        node = mac.wait_find(TITLE, source(), 'AXStaticText')
        try:
            assert equal(original, node), 'Shimmer update replaced the native text identity'
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def copy_source():
        focus_gallery_control(mac, source(), 'AXStaticText')
        mac.key(0, flags=1 << 20)
        mac.key(8, flags=1 << 20)
        deadline = time.monotonic() + 5
        while True:
            actual = subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=clipboard_env).stdout.decode('utf-8')
            if actual == source():
                break
            assert time.monotonic() < deadline, 'Shimmer copy differs from its logical source'
            time.sleep(.025)
        mac.key(124)

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.08)

    def reveal():
        # Keep the keyboard-targeted controls and source in the same viewport.
        reveal_gallery_control(mac, 'One sweep', 'AXCheckBox')
        reveal_gallery_control(mac, source(), 'AXStaticText')

    def capture():
        nonlocal capture_index
        # Avoid a full AX tree walk per animation sample: its latency can alias
        # the two-second sweep. Identity is checked separately around controls.
        bounds = element_rect(mac, original)
        window = mac.window(TITLE)
        try:
            wx, wy, ww, wh = element_rect(mac, window)
        finally:
            mac.release(window)
        x, y, w, h = bounds
        assert wx <= x and wy < y and x+w <= wx+ww and y+h < wy+wh, ('shimmer clipped in capture', bounds)
        path = directory / f'gallery-shimmer-{capture_index:03d}.png'
        capture_index += 1
        screenshot(mac, path, title=TITLE)
        pixels = read_png(mac, path)
        # A dense fixed grid within the actual text node excludes buttons/cursors.
        samples = tuple(pixels.rgb((x-wx+(ix+.5)*w/180)*pixels.width/ww,
                                   (y-wy+(iy+.5)*h/48)*pixels.height/wh)
                        for iy in range(48) for ix in range(180))
        return bounds, samples

    def changed(before, after):
        assert all(abs(a-b) < .1 for a,b in zip(before[0],after[0])), 'Shimmer moved or resized text'
        return sum(max(abs(a-b) for a,b in zip(p,q)) > 5 for p,q in zip(before[1],after[1]))

    def static():
        time.sleep(.12)
        first = capture()
        time.sleep(.25)
        second = capture()
        assert changed(first,second) == 0, 'Paused/disabled/reduced/finished text is still animating'
        return second

    def animated(base):
        deadline = time.monotonic()+4
        most_changed = 0
        timings = []
        started = time.monotonic()
        while time.monotonic()<deadline:
            time.sleep(.12)
            most_changed = max(most_changed,changed(base,capture()))
            timings.append(round(time.monotonic()-started,3))
            if most_changed >= 8:
                print('GALLERY_SHIMMER_PIXELS',most_changed,'sample_seconds=',timings,flush=True)
                return
        raise AssertionError(f'No captured glyph animation: {most_changed} changed samples at {timings}')

    theme_node = mac.find(TITLE,'Dark','AXButton')
    initial, alternate = ('Dark','Light') if theme_node else ('Light','Dark')
    if theme_node:
        mac.release(theme_node)
    cases = 0
    try:
        for theme in (initial,alternate):
            for width in ('wide','compact'):
                reveal()
                baseline = static()
                copy_source()
                assert all(abs(a-b)<.1 for a,b in zip(baseline[0],identity())), 'Copy changed text geometry'
                for reverse in (False,True):
                    mac.press(TITLE,'Start text shimmer')
                    mac.release(mac.wait_find(TITLE,'Pause text shimmer','AXButton'))
                    animated(baseline)
                    identity()
                    copy_source()
                    mac.press(TITLE,'Pause text shimmer')
                    mac.release(mac.wait_find(TITLE,'Start text shimmer','AXButton'))
                    assert changed(baseline,static()) == 0, 'Stopping did not restore ordinary text'
                    toggle('Reverse shimmer')
                    cases += 1
                mac.press(TITLE,'Shimmer width: '+width)
                mac.release(mac.wait_find(TITLE,'Shimmer width: '+('compact' if width=='wide' else 'wide'),'AXButton'))
            mac.press(TITLE,theme)
            mac.release(mac.wait_find(TITLE,alternate if theme==initial else initial,'AXButton'))

        reveal()
        baseline = static()
        mac.press(TITLE,'Start text shimmer')
        animated(baseline)
        # Real Space toggles effect clear on the focused public checkbox.
        focus_gallery_control(mac,'Text shimmer effect','AXCheckBox')
        mac.key(49)
        time.sleep(.12)
        assert changed(baseline,static()) == 0
        identity()
        mac.key(49)
        animated(baseline)
        mac.press(TITLE,'Pause text shimmer')
        toggle('One sweep')
        baseline = static()
        mac.press(TITLE,'Start text shimmer')
        animated(baseline)
        time.sleep(2.2)
        assert changed(baseline,static()) == 0, 'One-shot did not finish at ordinary text'
        mac.press(TITLE,'Refresh shimmer status')
        revision += 1
        mac.wait_text(TITLE,source())
        identity()
        copy_source()
        time.sleep(2.2)
        static()
        mac.press(TITLE,'Pause text shimmer')
        toggle('One sweep')

        # Playback is retained in Bonsai while page departure retires native text.
        mac.press(TITLE,'Start text shimmer')
        retired_source = source()
        mac.press(TITLE,'Motion & rhythm')
        absent(mac,retired_source,'AXStaticText')
        mac.press(TITLE,'Use reduced motion')
        mac.wait_text(TITLE,'Motion preference: Reduced')
        mac.press(TITLE,'Presentation')
        mac.wait_text(TITLE,source())
        replacement = mac.wait_find(TITLE,source(),'AXStaticText')
        assert not equal(original,replacement), 'Page departure retained the old native text node'
        mac.release(original)
        original = replacement
        reveal()
        static()
        copy_source()
        mac.press(TITLE,'Motion & rhythm')
        mac.press(TITLE,'Use full motion')
        mac.press(TITLE,'Presentation')
        mac.release(original)
        original = mac.wait_find(TITLE,source(),'AXStaticText')
        reveal()
        mac.press(TITLE,'Pause text shimmer')
        baseline = static()
        mac.press(TITLE,'Start text shimmer')
        animated(baseline)
        mac.press(TITLE,'Pause text shimmer')
        static()
        print(f'GALLERY_SHIMMER_OK: {cases} theme/width/direction cases, native identity/geometry, '
              'real keyboard Unicode copy and Space clear, glyph captures, pause/one-shot/source refresh, '
              'reduced motion and page retirement/remount',flush=True)
    finally:
        mac.release(original)
        subprocess.run(['/usr/bin/pbcopy'], input=saved, check=True, env=clipboard_env)
        temporary.cleanup()


def exercise_labels(mac, images):
    """Public label data -> Bonsai diff -> native foreground/selection/AX."""
    primary = 'İstanbul · Élan · agent'
    secondary = 'agent notes · 世界'
    mac.wait_text(TITLE, 'Labels that read naturally')
    raise_gallery(mac)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    original = mac.wait_find(TITLE, primary + ' ' + secondary, 'AXStaticText')
    clipboard_env = dict(os.environ, LANG='en_US.UTF-8', LC_ALL='en_US.UTF-8')
    saved_clipboard = subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=clipboard_env).stdout
    dark_button = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if dark_button else ('Light', 'Dark')
    if dark_button:
        mac.release(dark_button)

    def reveal(label):
        return reveal_gallery_control(mac, label, 'AXStaticText')

    def copy(label):
        focus_gallery_control(mac, label, 'AXStaticText')
        mac.key(0, flags=1 << 20)
        mac.key(8, flags=1 << 20)
        deadline = time.monotonic() + 5
        while True:
            actual = subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=clipboard_env).stdout.decode('utf-8')
            if actual == label:
                break
            assert time.monotonic() < deadline, 'Label clipboard did not match its display source'
            time.sleep(.025)
        mac.key(124)  # collapse selection before screenshots

    def native_identity(label):
        node = mac.wait_find(TITLE, label, 'AXStaticText')
        try:
            assert equal(original, node), 'Label configuration replaced native text identity'
        finally:
            mac.release(node)

    cases = 0
    try:
        for appearance in (current, alternate):
            for width in ('compact', 'wide'):
                for enabled in (True, False):
                    source = primary + (' ' + secondary if enabled else '')
                    for mode in ('plain', 'prefix i', 'all agent'):
                        mac.press(TITLE, 'Label: ' + mode)
                        native_identity(source)
                        _, _, _, height = reveal(source)
                        if enabled:
                            assert (height > 40 if width == 'compact' else height < 40), (width, height)
                        copy(source)
                        activate(mac, mac.wait_find(TITLE, 'Mask label', 'AXCheckBox'))
                        masked = '•' * len(source)
                        native_identity(masked)
                        _, _, _, height = reveal(masked)
                        if enabled:
                            assert (height > 40 if width == 'compact' else height < 40), (width, height)
                        # Original complete labels must disappear from native AX.
                        for unmasked in (primary, primary + ' ' + secondary):
                            old = mac.find(TITLE, unmasked, 'AXStaticText')
                            if old:
                                mac.release(old)
                                raise AssertionError('Masked label exposed its original AX source')
                        copy(masked)
                        if images and width == 'compact' and enabled and mode == 'all agent':
                            screenshot(mac, images / f'gallery-label-{appearance.lower()}-masked.png', title=TITLE)
                        activate(mac, mac.wait_find(TITLE, 'Mask label', 'AXCheckBox'))
                        mac.wait_text(TITLE, source)
                        if images and width == 'compact' and enabled and mode == 'all agent':
                            reveal(source)
                            screenshot(mac, images / f'gallery-label-{appearance.lower()}.png', title=TITLE)
                        cases += 2
                    activate(mac, mac.wait_find(TITLE, 'Label secondary text', 'AXCheckBox'))
                mac.press(TITLE, 'Label width: ' + width)
                mac.release(mac.wait_find(TITLE, 'Label width: ' + ('wide' if width == 'compact' else 'compact'), 'AXButton'))
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
    finally:
        mac.release(original)
        subprocess.run(['/usr/bin/pbcopy'], input=saved_clipboard, check=True, env=clipboard_env)
    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Refresh resource counts')
    wait_for_resource_cleanup(mac)
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, primary + ' ' + secondary)
    print(f'GALLERY_LABEL_OK: {cases} theme/width/secondary/match/mask cases, real keyboard '
          'copy of visible-only source, native text identity, AX secrecy and page teardown', flush=True)


def exercise_badges(mac, images):
    """Badges preserve their content's native input owner and borrow icon assets."""
    mac.release(mac.wait_find(TITLE, 'Open inbox', 'AXButton'))
    raise_gallery(mac)
    mouse = GalleryMouse(mac)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    body = mac.wait_find(TITLE, 'Open inbox', 'AXButton')
    clicks = 0

    def rect(label, role):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def image_labels():
        result = []
        def visit(node):
            values, children = mac.node_values(node)
            try:
                if values[0] == 'AXImage':
                    result.append(values[1] or values[2] or values[3])
                assert '99+' not in values[1:] and '9+' not in values[1:], values
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        group = mac.wait_find(TITLE, 'Badged inbox', 'AXGroup')
        try:
            visit(group)
        finally:
            mac.release(group)
        return result

    def reveal():
        window = mac.window(TITLE)
        try:
            wx, wy, ww, wh = element_rect(mac, window)
        finally:
            mac.release(window)
        point = (wx + ww * .75, wy + wh * .65)
        create = mac.cg.CGEventCreateScrollWheelEvent
        create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
        locate = mac.cg.CGEventSetLocation
        locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]
        for _ in range(12):
            _, y, _, h = rect('Open inbox', 'AXButton')
            if y >= wy + 150 and y + h <= wy + wh - 24:
                return
            mouse.send(5, point)
            mouse.check_owner(point)
            event = create(None, 0, 1, C.c_int(-90 if y + h > wy + wh - 24 else 90))
            assert event
            try:
                locate(event, GalleryMouse.Point(*point))
                mouse.post(0, event)
            finally:
                mac.release(event)
            time.sleep(.1)
        raise RuntimeError('Badge content did not become visible')

    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    try:
        for appearance in (current, alternate):
            for size, next_size in [('medium', 'large'), ('large', 'small'), ('small', 'medium')]:
                for mode, label in [('Unread: 0', None), ('Unread: 7', '7 unread messages'),
                                    ('Unread: 150', '150 unread messages'),
                                    ('Activity dot', 'Inbox has new activity'),
                                    ('Verified icon', 'Verified inbox'), ('Decorative icon', None)]:
                    mac.press(TITLE, mode)
                    expected = [label] if label else []
                    deadline = time.monotonic() + 10
                    while image_labels() != expected:
                        assert time.monotonic() < deadline, (mode, image_labels())
                        time.sleep(.025)
                    reveal()
                    other = mac.wait_find(TITLE, 'Open inbox', 'AXButton')
                    try:
                        assert equal(body, other), 'Badge changes replaced the native content'
                        x, y, w, h = element_rect(mac, other)
                    finally:
                        mac.release(other)
                    assert abs(w - 170) <= 1 and abs(h - 48) <= 1, (mode, x, y, w, h)
                    if label:
                        bx, by, bw, bh = rect(label, 'AXImage')
                        if mode == 'Verified icon':
                            assert abs(bx + bw - (x + w - 3)) <= 1, (bx, bw, x, w)
                            assert abs(by + bh - (y + h - 3)) <= 1, (by, bh, y, h)
                        else:
                            assert abs(bx + bw - (x + w)) <= 1 and abs(by - y) <= 1, (bx, by, bw, x, y, w)
                        point = (bx + bw / 2, by + bh / 2)
                    else:
                        point = (x + w / 2, y + h / 2)
                    # Real pointer input lands on the painted badge itself.
                    mouse.check_owner(point)
                    mouse.send(5, point)
                    mouse.send(1, point)
                    mouse.send(2, point)
                    clicks += 1
                    mac.wait_text(TITLE, f'Inbox opens: {clicks}')
                    focus_gallery_control(mac, 'Open inbox', 'AXButton')
                    mac.key(36)
                    clicks += 1
                    mac.wait_text(TITLE, f'Inbox opens: {clicks}')
                    mac.key(48)
                    expect_focus(mac, f'Badge size: {size}')
                    if mode == 'Unread: 150':
                        mac.press(TITLE, 'Badge cap: 99')
                        mac.release(mac.wait_find(TITLE, 'Badge cap: 9', 'AXButton'))
                        assert image_labels() == expected
                        mac.press(TITLE, 'Badge cap: 9')
                        mac.release(mac.wait_find(TITLE, 'Badge cap: 99', 'AXButton'))
                    if images and size == 'medium' and mode in ('Unread: 150', 'Verified icon'):
                        screenshot(mac, images / f'gallery-badge-{appearance.lower()}-{mode.split()[0].rstrip(":").lower()}.png', title=TITLE)
                mac.press(TITLE, f'Badge size: {size}')
                mac.release(mac.wait_find(TITLE, f'Badge size: {next_size}', 'AXButton'))
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
    finally:
        mac.release(body)
    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Refresh resource counts')
    wait_for_resource_cleanup(mac)
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    mac.press(TITLE, 'Presentation')
    mac.release(mac.wait_find(TITLE, 'Open inbox', 'AXButton'))
    print('GALLERY_OVERLAY_BADGE_OK: 36 kind/size/theme cases, uncapped labels and zero/decorative '
          'omission, anchors, 72 real pointer/Return activations, one content focus stop, '
          'native identity and scoped asset cleanup/remount', flush=True)


def exercise_status_regions(mac, images):
    """Verify native geometry and actions of the public three-region composition."""
    mac.wait_text(TITLE, 'A little context goes a long way')
    present = [True, True, True]
    labels = ['Leading status', 'Center action', 'Trailing status']
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    clicks = 0
    retained = None
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]

    def rect(label, role):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    try:
        for appearance in (current, alternate):
            for size, next_size in [('Comfortable', 'Large'), ('Large', 'Compact'),
                                    ('Compact', 'Comfortable')]:
                for wanted in [(True, True, True), (False, True, True),
                               (False, True, False), (True, True, False),
                               (True, False, False), (True, False, True),
                               (False, False, True), (False, False, False)]:
                    for i, enabled in enumerate(wanted):
                        if present[i] != enabled:
                            activate(mac, mac.wait_find(TITLE, labels[i], 'AXCheckBox'))
                    present = list(wanted)
                    leading, center, trailing = wanted
                    for enabled, label in [(leading, 'Workspace ready'), (trailing, 'UTF-8')]:
                        if enabled:
                            mac.wait_text(TITLE, label)
                        else:
                            wait_absent(mac, label, 'AXStaticText')
                    if not center:
                        wait_absent(mac, 'Sync workspace', 'AXButton')
                        if retained:
                            mac.release(retained)
                            retained = None
                        continue
                    button = mac.wait_find(TITLE, 'Sync workspace', 'AXButton')
                    if retained:
                        assert equal(retained, button), 'Changing ends replaced the center control'
                    else:
                        retained = mac.retain(button)
                    mac.release(button)
                    deadline = time.monotonic() + 10
                    while True:
                        bx, by, bw, bh = rect('Workspace status bar', 'AXGroup')
                        x, y, w, h = rect('Sync workspace', 'AXButton')
                        start, end = bx + 8, bx + bw - 8
                        if leading:
                            lx, _, lw, _ = rect('Workspace ready', 'AXStaticText')
                            start = lx + lw + 12
                        if trailing:
                            rx, _, _, _ = rect('UTF-8', 'AXStaticText')
                            end = rx - 12
                        expected = ((start + end - w) / 2 if leading and trailing
                                    else end - w if leading else start)
                        if (abs(x - expected) <= 2 and w > 20 and h > 20
                                and y >= by and y + h <= by + bh + 1):
                            break
                        assert time.monotonic() < deadline, (wanted, (bx, by, bw, bh),
                                                             (x, y, w, h), expected)
                        time.sleep(.025)
                    focus_gallery_control(mac, 'Sync workspace', 'AXButton')
                    mac.key(36)
                    clicks += 1
                    mac.wait_text(TITLE, f'Status actions: {clicks}')
                    expect_focus(mac, 'Sync workspace')
                    if images and size == 'Comfortable' and leading and trailing:
                        screenshot(mac, images / f'gallery-status-{appearance.lower()}.png', title=TITLE)
                mac.press(TITLE, size)
                mac.release(mac.wait_find(TITLE, next_size, 'AXButton'))
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, alternate if appearance == current else current, 'AXButton'))
        for label in labels:
            activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        mac.release(mac.wait_find(TITLE, 'Sync workspace', 'AXButton'))
    finally:
        if retained:
            mac.release(retained)
    print('GALLERY_STATUS_REGIONS_OK: 48 theme/size/slot combinations; remaining-space '
          'alignment, native control identity, real Return activation, focus and absent semantics', flush=True)


def within(mac, group, label, role):
    def visit(node):
        if (mac.text(node, 'AXRole') == role
                and label in (mac.text(node, 'AXTitle'), mac.text(node, 'AXDescription'))):
            return mac.retain(node)
        children = mac.children(node)
        try:
            for child in children:
                found = visit(child)
                if found:
                    return found
        finally:
            for child in children:
                mac.release(child)
        return None
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        root = mac.find(TITLE, group)
        if root:
            try:
                found = visit(root)
                if found:
                    return found
            finally:
                mac.release(root)
        time.sleep(.03)
    mac.dump(TITLE)
    raise RuntimeError(f'{label} not found within {group}')


def activate(mac, node):
    try:
        mac.perform(node, 'AXPress')
    finally:
        mac.release(node)


def open_picker(mac, trigger, cancel):
    node = mac.wait_find(TITLE, trigger, 'AXButton')
    try:
        mac.set(node, 'AXFocused', mac.true)
        mac.perform(node, 'AXPress')
    finally:
        mac.release(node)
    mac.release(mac.wait_find(TITLE, cancel, 'AXButton'))


def expect_focus(mac, trigger, role="AXButton", *, title=TITLE, focused=True):
    get = mac.cf.CFBooleanGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        node = mac.wait_find(title, trigger, role)
        value = mac.attr(node, 'AXFocused')
        try:
            if value and bool(get(value)) == focused:
                return
        finally:
            if value:
                mac.release(value)
            mac.release(node)
        time.sleep(.03)
    raise RuntimeError(f'{title}: expected {trigger} AXFocused={focused}')


def expect_popup_expanded(mac, label, expected, *, role="AXButton"):
    get = mac.cf.CFBooleanGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        node = mac.wait_find(TITLE, label, role)
        value = mac.attr(node, 'AXExpanded')
        try:
            if value and bool(get(value)) == expected:
                return
        finally:
            if value:
                mac.release(value)
            mac.release(node)
        time.sleep(.03)
    raise RuntimeError(f'{label}: expected AXExpanded={expected}')


def exercise_pickers(mac, images):
    mac.press(TITLE, 'Dates & colors')
    from gallery_calendar_content import exercise as exercise_calendar_content
    exercise_calendar_content(mac)
    reveal_gallery_control(mac, 'Choose appointment', 'AXButton')
    mac.wait_text(TITLE, 'Appointment: 2026-09-14')
    open_picker(mac, 'Choose appointment', 'Cancel appointment')
    expect_popup_expanded(mac, 'Choose appointment', True)
    activate(mac, within(mac, 'Preview appointment', 'September 17, 2026', 'AXCheckBox'))
    mac.wait_text(TITLE, 'Appointment: 2026-09-14')
    mac.press(TITLE, 'Cancel appointment')
    expect_focus(mac, 'Choose appointment')
    expect_popup_expanded(mac, 'Choose appointment', False)
    mac.wait_text(TITLE, 'Appointment: 2026-09-14')
    open_picker(mac, 'Choose appointment', 'Cancel appointment')
    expect_popup_expanded(mac, 'Choose appointment', True)
    activate(mac, within(mac, 'Preview appointment', 'September 18, 2026', 'AXCheckBox'))
    mac.press(TITLE, 'Apply appointment')
    mac.wait_text(TITLE, 'Appointment: 2026-09-18')
    expect_focus(mac, 'Choose appointment')
    expect_popup_expanded(mac, 'Choose appointment', False)
    open_picker(mac, 'Choose accent', 'Cancel accent')
    expect_popup_expanded(mac, 'Choose accent', True)
    hex_field = within(mac, 'Preview accent', 'Hex color', 'AXTextField')
    try:
        original_hex = mac.text(hex_field, 'AXValue')
        activate(mac, within(mac, 'Preview accent', 'HSLA', 'AXRadioButton'))
        mac.release(within(mac, 'Preview accent', 'Hue', 'AXSlider'))
        assert mac.text(hex_field, 'AXValue') == original_hex
        activate(mac, within(mac, 'Preview accent', 'Palette', 'AXRadioButton'))
        assert mac.text(hex_field, 'AXValue') == original_hex
    finally:
        mac.release(hex_field)
    mac.wait_text(TITLE, 'Accent: #89DDC9')
    activate(mac, within(mac, 'Preview accent', 'Iris', 'AXRadioButton'))
    mac.wait_text(TITLE, 'Accent: #89DDC9')
    mac.press(TITLE, 'Cancel accent')
    expect_focus(mac, 'Choose accent')
    expect_popup_expanded(mac, 'Choose accent', False)
    open_picker(mac, 'Choose accent', 'Cancel accent')
    expect_popup_expanded(mac, 'Choose accent', True)
    activate(mac, within(mac, 'Preview accent', 'Coral', 'AXRadioButton'))
    mac.press(TITLE, 'Apply accent')
    mac.wait_text(TITLE, 'Accent: #F6A89D')
    expect_focus(mac, 'Choose accent')
    expect_popup_expanded(mac, 'Choose accent', False)
    if images:
        screenshot(mac, images / 'gallery-pickers.png', title=TITLE)
    open_picker(mac, 'Choose accent', 'Cancel accent')
    expect_popup_expanded(mac, 'Choose accent', True)
    mac.key(53)
    expect_focus(mac, 'Choose accent')
    expect_popup_expanded(mac, 'Choose accent', False)
    mac.wait_text(TITLE, 'Accent: #F6A89D')
    mac.press(TITLE, 'Clear appointment')
    mac.wait_text(TITLE, 'Appointment: No date selected')
    mac.press(TITLE, 'Clear accent')
    mac.wait_text(TITLE, 'Accent: No color')


def exercise_overlays(mac, images):
    mac.press(TITLE, 'Overlays & help')
    open_picker(mac, 'Open dialog', 'Close dialog')
    original = mac.wait_find(TITLE, 'Preview dialog', 'AXWindow')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    try:
        for _ in range(2):
            mac.press(TITLE, 'Change backdrop')
            current = mac.wait_find(TITLE, 'Preview dialog', 'AXWindow')
            modal = mac.attr(current, 'AXModal')
            try:
                assert equal(original, current), 'Backdrop update replaced dialog'
                assert modal and boolean(modal), 'Dialog must expose AXModal'
            finally:
                if modal:
                    mac.release(modal)
                mac.release(current)
    finally:
        mac.release(original)
    mac.key(53)
    expect_focus(mac, 'Open dialog')
    mac.press(TITLE, 'Animate opening')
    open_picker(mac, 'Open dialog', 'Close dialog')
    mac.press(TITLE, 'Close dialog')
    expect_focus(mac, 'Open dialog')
    mac.press(TITLE, 'Animate opening')
    open_picker(mac, 'Open drawer', 'Close drawer')
    mac.press(TITLE, 'Close drawer')
    expect_focus(mac, 'Open drawer')
    open_picker(mac, 'Review confirmation', 'Keep preview')
    mac.press(TITLE, 'Keep preview')
    mac.wait_text(TITLE, 'Nothing has been changed.')
    open_picker(mac, 'Review confirmation', 'Keep preview')
    mac.press(TITLE, 'Confirm reset')
    mac.wait_text(TITLE, 'Preview reset confirmed.')
    expect_focus(mac, 'Review confirmation')
    open_picker(mac, 'Show details', 'Done with details')
    if images:
        screenshot(mac, images / 'gallery-overlays.png', title=TITLE)
    mac.press(TITLE, 'Done with details')
    expect_focus(mac, 'Show details')

    # Animated managed help replaces only accepted managed content. Physical
    # animation smoothness still needs visual review; AX presence is not paint.
    tips = [
        ('Focus for a tip', 'Tooltips also appear when their trigger receives keyboard focus.'),
        ('Streaming help', 'Responses can stream while the native interface stays responsive.'),
        ('Keyboard help', 'Move between these triggers to preview a native tooltip switch.'),
    ]
    previous_text = None
    for trigger, text in tips:
        focus_gallery_control(mac, trigger, 'AXButton')
        mac.wait_text(TITLE, text)
        if previous_text:
            wait_absent(mac, previous_text, None)
        previous_text = text
    mac.key(53)
    wait_absent(mac, previous_text, None)


def exercise_navigation(mac, images):
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
    from gallery_disclosure import exercise as exercise_disclosure
    exercise_disclosure(mac)
    exercise_pagination(mac)
    mac.press(TITLE, 'Next')
    mac.wait_text(TITLE, 'Preview page 43 of 120')
    mac.press(TITLE, 'Last')
    mac.wait_text(TITLE, 'Preview page 120 of 120')
    if images:
        screenshot(mac, images / 'gallery-navigation.png', title=TITLE)


def exercise_pagination(mac):
    first, second = 'Pages 2 to 58', 'Pages 62 to 119'
    reveal_gallery_control(mac, first, 'AXButton')
    mac.wait_text(TITLE, 'Preview page 60 of 120')
    expect_popup_expanded(mac, first, False)
    expect_popup_expanded(mac, second, False)
    # Semantic activation deliberately does not focus the trigger first.
    mac.press(TITLE, second)
    mac.wait_text(TITLE, 'Choose a page from 62 to 119')
    expect_focus(mac, 'Page number', 'AXTextField')
    expect_popup_expanded(mac, first, False)
    expect_popup_expanded(mac, second, True)
    mac.press(TITLE, 'Cancel')
    expect_focus(mac, second)
    expect_popup_expanded(mac, second, False)
    open_picker(mac, first, 'Cancel')
    expect_focus(mac, 'Page number', 'AXTextField')
    expect_popup_expanded(mac, first, True)
    expect_popup_expanded(mac, second, False)
    # The native numeric field receives opening focus. Enter commits the draft,
    # while the explicit Go action requests navigation through the Eio controller.
    mac.key(0, flags=1 << 20)  # Command-A.
    mac.key(21)  # 4.
    mac.key(19)  # 2.
    mac.key(36)
    mac.wait_text(TITLE, 'Preview page 60 of 120')
    expect_enabled(mac, 'Go to page', True)
    mac.press(TITLE, 'Go to page')
    mac.wait_text(TITLE, 'Preview page 42 of 120')
    expect_popup_expanded(mac, 'Pages 2 to 40', False)
    expect_popup_expanded(mac, 'Pages 44 to 119', False)


def exercise_feedback(mac, images):
    mac.press(TITLE, 'Commands & feedback')
    mac.wait_text(TITLE, 'Ready to begin')
    mac.press(TITLE, 'Advance preview')
    mac.wait_text(TITLE, 'Finding the right pieces')
    activate(mac, mac.wait_find(TITLE, 'Enable preview command', 'AXCheckBox'))
    expect_enabled(mac, 'Advance preview', False)
    activate(mac, mac.wait_find(TITLE, 'Enable preview command', 'AXCheckBox'))
    expect_enabled(mac, 'Advance preview', True)
    draft = mac.wait_find(TITLE, 'Command preview draft', 'AXTextField')
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.set(draft, 'AXFocused', mac.true)
        expect_focus(mac, 'Command preview draft', 'AXTextField')
        mac.key(40, 1 << 20)  # Command-K.
    finally:
        mac.release(draft)
    mac.wait_text(TITLE, 'Halfway there')
    mac.key(35, (1 << 20) | (1 << 17))  # Command-Shift-P.
    mac.wait_text(TITLE, 'Preview commands')
    mac.key(36)  # Native chooser selects the first enabled command.
    mac.wait_text(TITLE, 'Everything is in place')
    mac.press(TITLE, 'Preview actions')
    activate(mac, mac.wait_find(TITLE, 'Save preview', 'AXMenuItem'))
    mac.wait_text(TITLE, 'Notification visible')
    mac.press(TITLE, 'Dismiss saved preview')
    mac.wait_text(TITLE, 'No pending notification')
    activate(mac, mac.wait_find(TITLE, 'Preview actions', 'AXMenuItem'))
    mac.release(mac.wait_find(TITLE, 'Editing', 'AXMenuItem'))
    mac.key(53)
    mac.press(TITLE, 'Save preview')
    mac.wait_text(TITLE, 'Notification visible')
    mac.wait_text(TITLE, 'Your preview is ready to share.')
    if images:
        screenshot(mac, images / 'gallery-feedback.png', title=TITLE)
    mac.press(TITLE, 'Dismiss saved preview')
    mac.wait_text(TITLE, 'No pending notification')
    mac.press(TITLE, 'Save preview')
    mac.wait_text(TITLE, 'Notification visible')
    # Native active-time expiry: keep focus and pointer outside the toast.
    draft = mac.wait_find(TITLE, 'Command preview draft', 'AXTextField')
    try:
        mac.set(draft, 'AXFocused', mac.true)
    finally:
        mac.release(draft)
    mac.wait_text(TITLE, 'No pending notification')
    mac.press(TITLE, 'Save preview')
    mac.wait_text(TITLE, 'Notification visible')
    mac.press(TITLE, 'Presentation')
    mac.press(TITLE, 'Commands & feedback')
    mac.wait_text(TITLE, 'Everything is in place')
    mac.wait_text(TITLE, 'No pending notification')


def expect_enabled(mac, label, expected, role='AXButton'):
    get = mac.cf.CFBooleanGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        node = mac.wait_find(TITLE, label, role)
        value = mac.attr(node, 'AXEnabled')
        try:
            if value and bool(get(value)) == expected:
                return
        finally:
            if value:
                mac.release(value)
            mac.release(node)
        time.sleep(.03)
    raise RuntimeError(f'{label} did not publish enabled={expected}')


def absent(mac, label, role):
    node = mac.find(TITLE, label, role)
    if node:
        mac.release(node)
        raise RuntimeError(f'Hidden content is still accessible: {label}')


def type_a(mac, label):
    window = mac.window(TITLE)
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    field = mac.wait_find(TITLE, label, 'AXTextField')
    try:
        mac.set(field, 'AXFocused', mac.true)
        expect_focus(mac, label, 'AXTextField')
        mac.key(0, 1 << 20)
        mac.key(0)
    finally:
        mac.release(field)
    expect_field(mac, TITLE, label, 'a')


def exercise_journeys(mac, images):
    mac.press(TITLE, 'Carousels & journeys')
    mac.wait_text(TITLE, 'Current idea: Imagine')
    type_a(mac, 'Carousel idea')
    mac.press(TITLE, 'Next')
    mac.wait_text(TITLE, 'Current idea: Shape')
    absent(mac, 'Carousel idea', 'AXTextField')
    mac.press(TITLE, 'Previous')
    mac.wait_text(TITLE, 'Current idea: Imagine')
    expect_field(mac, TITLE, 'Carousel idea', 'a')
    mac.press(TITLE, 'Use vertical slides')
    mac.press(TITLE, 'Next')
    mac.wait_text(TITLE, 'Current idea: Shape')
    mac.press(TITLE, 'Previous')
    mac.wait_text(TITLE, 'Current idea: Imagine')
    expect_field(mac, TITLE, 'Carousel idea', 'a')
    mac.press(TITLE, 'Use horizontal slides')
    type_a(mac, 'Journey note')
    mac.press(TITLE, 'Continue journey')
    mac.wait_text(TITLE, 'Current journey: Shape')
    absent(mac, 'Journey note', 'AXTextField')
    mac.press(TITLE, 'Go back')
    mac.wait_text(TITLE, 'Current journey: Imagine')
    expect_field(mac, TITLE, 'Journey note', 'a')
    activate(mac, mac.wait_find(TITLE, 'Observatory', 'AXLink'))
    mac.wait_text(TITLE, 'Destination: Observatory')
    mac.press(TITLE, 'Collapse sidebar')
    mac.release(mac.wait_find(TITLE, 'Expand sidebar', 'AXButton'))
    absent(mac, 'Observatory', 'AXLink')
    mac.press(TITLE, 'Expand sidebar')
    mac.release(mac.wait_find(TITLE, 'Observatory', 'AXLink'))
    mac.press(TITLE, 'Use offcanvas sidebar')
    mac.press(TITLE, 'Collapse sidebar')
    mac.release(mac.wait_find(TITLE, 'Expand sidebar', 'AXButton'))
    absent(mac, 'Projects', 'AXLink')
    mac.press(TITLE, 'Expand sidebar')
    mac.release(mac.wait_find(TITLE, 'Observatory', 'AXLink'))
    mac.wait_text(TITLE, 'Destination: Observatory')
    if images:
        screenshot(mac, images / 'gallery-journeys.png', title=TITLE)
    activate(mac, mac.wait_find(TITLE, 'Auto-advance slides', 'AXCheckBox'))
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A little context goes a long way')
    absent(mac, 'Carousel idea', 'AXTextField')
    absent(mac, 'Journey note', 'AXTextField')


def tree_counts(mac, root):
    roles = {}
    def visit(node):
        values, children = mac.node_values(node)
        roles[values[0]] = roles.get(values[0], 0) + 1
        try:
            for child in children:
                visit(child)
        finally:
            for child in children:
                mac.release(child)
    try:
        visit(root)
    finally:
        mac.release(root)
    return roles


def exercise_collections(mac, images):
    mac.press(TITLE, 'Lists, trees & tables')
    mac.wait_text(TITLE, 'Entry 0999')
    mac.wait_text(TITLE, 'Following new messages.')
    mac.press(TITLE, 'Grow latest response')
    mac.wait_text(TITLE, '1000 messages · Latest response: 1 / 8 extra lines')
    mac.wait_text(TITLE, 'Another useful detail.')
    mac.press(TITLE, 'New message')
    mac.wait_text(TITLE, 'Entry 1000')
    mac.wait_text(TITLE, '1001 messages · Latest response: 0 / 8 extra lines')
    mac.press(TITLE, 'First entry')
    mac.wait_text(TITLE, 'Entry 0000')
    mac.wait_text(TITLE, 'Reading history.')
    mac.release(mac.wait_find(TITLE, 'Follow latest', 'AXButton'))
    if images:
        screenshot(mac, images / 'gallery-message-follow.png', title=TITLE)
    mac.press(TITLE, 'Follow latest')
    mac.wait_text(TITLE, 'Following new messages.')
    absent(mac, 'Follow latest', 'AXButton')
    mac.press(TITLE, 'First entry')
    mac.wait_text(TITLE, 'Entry 0000')
    mac.wait_text(TITLE, 'Reading history.')
    mac.press(TITLE, 'Earlier history')
    mac.wait_text(TITLE, '1002 messages · Latest response: 0 / 8 extra lines')
    mac.wait_text(TITLE, 'Entry 0000')
    mac.wait_text(TITLE, 'Reading history.')
    mac.press(TITLE, 'New message')
    mac.wait_text(TITLE, '1003 messages · Latest response: 0 / 8 extra lines')
    mac.wait_text(TITLE, 'Entry 0000')
    mac.wait_text(TITLE, 'Reading history.')
    mac.press(TITLE, 'Jump to latest')
    mac.wait_text(TITLE, 'Entry 1001')
    mac.wait_text(TITLE, 'Following new messages.')
    mac.press(TITLE, 'Grow latest response')
    mac.wait_text(TITLE, '1003 messages · Latest response: 1 / 8 extra lines')
    mac.wait_text(TITLE, 'Another useful detail.')
    mac.press(TITLE, 'Reset latest response')
    mac.wait_text(TITLE, '1003 messages · Latest response: 0 / 8 extra lines')
    mac.press(TITLE, 'Outline tree')
    mac.release(mac.wait_find(TITLE, 'Field notes', 'AXRow', search_files=True))
    mac.press(TITLE, 'Reveal observatory')
    activate(mac, mac.wait_find(TITLE, 'Observatory study', 'AXRow', search_files=True))
    mac.wait_text(TITLE, 'Selected outline: observatory')
    counts = tree_counts(mac, mac.wait_find(TITLE, 'Preview outline', 'AXOutline'))
    if not 1 <= counts.get('AXRow', 0) <= 16:
        raise RuntimeError(f'Outline escaped its row budget: {counts}')
    mac.press(TITLE, 'Result table')
    # Virtual cells mount only after the table enters the page viewport.
    reveal_gallery_control(mac, 'Preview results', 'AXTable')
    mac.release(mac.wait_find(TITLE, '0000', 'AXCell', search_files=True))
    mac.press(TITLE, 'Select last result')
    mac.wait_text(TITLE, 'Table selection: Cell 999 / detail')
    mac.release(mac.wait_find(TITLE, 'Finding 0999 · 日本語 · 👨‍👩‍👧‍👦', 'AXCell', search_files=True))
    counts = tree_counts(mac, mac.wait_find(TITLE, 'Preview results', 'AXTable'))
    if not (1 <= counts.get('AXRow', 0) <= 25 and 3 <= counts.get('AXCell', 0) <= 75):
        raise RuntimeError(f'Table escaped its row/cell budget: {counts}')
    if images:
        screenshot(mac, images / 'gallery-collections.png', title=TITLE)
    mac.press(TITLE, 'First result')
    mac.release(mac.wait_find(TITLE, '0000', 'AXCell', search_files=True))
    mac.press(TITLE, 'Outline tree')
    mac.wait_text(TITLE, 'Selected outline: observatory')
    mac.press(TITLE, 'Message list')
    mac.wait_text(TITLE, 'Entry 1001')
    mac.wait_text(TITLE, '1003 messages · Latest response: 0 / 8 extra lines')
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A little context goes a long way')
    absent(mac, 'Preview results', 'AXTable')
    absent(mac, 'Preview outline', 'AXOutline')


def exercise_highlighting(mac, images):
    mac.press(TITLE, 'Find & highlight')
    mac.wait_text(TITLE, '7 matches · selected 1')
    expect_field(mac, TITLE, 'Find in preview', 'ideas')
    mac.press(TITLE, 'Next match')
    mac.wait_text(TITLE, '7 matches · selected 2')
    mac.press(TITLE, 'Previous match')
    mac.wait_text(TITLE, '7 matches · selected 1')
    mac.press(TITLE, 'Previous match')
    mac.wait_text(TITLE, '7 matches · selected 7')
    mac.press(TITLE, 'Next match')
    mac.wait_text(TITLE, '7 matches · selected 1')
    if images:
        screenshot(mac, images / 'gallery-highlighting-dark.png', title=TITLE)

    def search(text):
        # Real native accessibility editing; this does not replace the editor's
        # OCaml model or synthesize a highlight observation.
        mac.field(TITLE, 'Find in preview', 'AXTextField', text=text)
        expect_field(mac, TITLE, 'Find in preview', text)

    search('IDEAS')
    mac.wait_text(TITLE, '7 matches · selected 1')
    activate(mac, mac.wait_find(TITLE, 'Match case', 'AXCheckBox'))
    mac.wait_text(TITLE, 'No matches')
    activate(mac, mac.wait_find(TITLE, 'Match case', 'AXCheckBox'))
    mac.wait_text(TITLE, '7 matches · selected 1')
    search('idea')
    mac.wait_text(TITLE, '7 matches · selected 1')
    activate(mac, mac.wait_find(TITLE, 'Whole words', 'AXCheckBox'))
    mac.wait_text(TITLE, 'No matches')
    search('ideas')
    mac.wait_text(TITLE, '7 matches · selected 1')
    activate(mac, mac.wait_find(TITLE, 'Whole words', 'AXCheckBox'))
    activate(mac, mac.wait_find(TITLE, 'Enable search', 'AXCheckBox'))
    mac.wait_text(TITLE, 'Search paused')
    expect_field(mac, TITLE, 'Find in preview', 'ideas')
    activate(mac, mac.wait_find(TITLE, 'Enable search', 'AXCheckBox'))
    mac.wait_text(TITLE, '7 matches · selected 1')
    search('')
    mac.wait_text(TITLE, 'Enter a word to search')
    search('x' * 4097)
    mac.wait_text(TITLE, 'Choose a search of at most 4,096 UTF-8 bytes, without NUL')
    search('ideas')
    mac.wait_text(TITLE, '7 matches · selected 1')
    mac.press(TITLE, 'Add a paragraph')
    mac.wait_text(TITLE, '9 matches · selected 1')
    mac.press(TITLE, 'Add a paragraph')
    mac.wait_text(TITLE, '9 matches · selected 1')
    # The document's native collapse control changes searchable presentation.
    mac.press(TITLE, 'Previous match')
    mac.wait_text(TITLE, '9 matches · selected 9')
    mac.press(TITLE, 'Collapse')
    mac.wait_text(TITLE, '4 matches · selected 4')
    mac.press(TITLE, 'Previous match')
    mac.wait_text(TITLE, '4 matches · selected 3')
    mac.press(TITLE, 'Next match')
    mac.wait_text(TITLE, '4 matches · selected 4')
    mac.press(TITLE, 'Next match')
    mac.wait_text(TITLE, '4 matches · selected 1')
    mac.press(TITLE, 'Expand')
    mac.wait_text(TITLE, '9 matches · selected 1')
    dark_button = mac.find(TITLE, 'Dark', 'AXButton')
    current_theme = 'Dark' if dark_button else 'Light'
    if dark_button:
        mac.release(dark_button)
    mac.press(TITLE, current_theme)
    alternate_theme = 'Light' if current_theme == 'Dark' else 'Dark'
    mac.release(mac.wait_find(TITLE, alternate_theme, 'AXButton'))
    # A theme update rotates the paired highlight configuration asynchronously.
    # Do not accept the old frame's count before that update has settled.
    deadline, stable_since = time.monotonic() + 10, None
    while time.monotonic() < deadline:
        node = mac.find(TITLE, '9 matches · selected 1')
        if node:
            mac.release(node)
            if stable_since is None:
                stable_since = time.monotonic()
            if time.monotonic() - stable_since >= .2:
                break
        else:
            stable_since = None
        time.sleep(.025)
    else:
        raise RuntimeError('Highlight count did not settle after theme change')
    expect_field(mac, TITLE, 'Find in preview', 'ideas')
    if images:
        screenshot(mac, images / 'gallery-highlighting-alternate.png', title=TITLE)
    mac.press(TITLE, alternate_theme)
    mac.press(TITLE, 'Presentation')
    wait_absent(mac, 'Find in preview', 'AXTextField')
    wait_absent(mac, 'Searchable notebook', 'AXGroup')
    mac.press(TITLE, 'Find & highlight')
    expect_field(mac, TITLE, 'Find in preview', 'ideas')
    mac.wait_text(TITLE, '7 matches · selected 1')


def document_structure(mac):
    """Read the actual painted Markdown hierarchy through external macOS AX."""
    def descendants(node, role):
        result = []
        if mac.text(node, 'AXRole') == role:
            result.append(mac.retain(node))
        children = mac.children(node)
        try:
            for child in children:
                result.extend(descendants(child, role))
        finally:
            for child in children:
                mac.release(child)
        return result

    def number(node, attribute):
        get = mac.cf.CFNumberGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        number_type = mac.cf.CFNumberGetTypeID
        number_type.restype, number_type.argtypes = C.c_ulong, []
        value, result = mac.attr(node, attribute), C.c_longlong()
        try:
            assert value and mac.type_id(value) == number_type(), attribute
            assert value and get(value, 4, C.byref(result)), attribute
            return result.value
        finally:
            if value:
                mac.release(value)

    def index_range(node, attribute):
        class Range(C.Structure):
            _fields_ = [('location', C.c_long), ('length', C.c_long)]
        get = mac.ax.AXValueGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        value, result = mac.attr(node, attribute), Range()
        try:
            assert value and get(value, 4, C.byref(result)), attribute
            return result.location, result.length
        finally:
            if value:
                mac.release(value)

    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    root = mac.wait_find(TITLE, 'Markdown preview', 'AXGroup')
    headings, tables, rows, all_cells, headers, row_headers = [], [], [], [], [], []
    header_group = None
    try:
        headings = descendants(root, 'AXHeading')
        assert len(headings) == 2, len(headings)
        # Disabled frontmatter leaves a level-two Setext metadata heading.
        assert [number(heading, 'AXValue') for heading in headings] == [2, 1]
        tables = descendants(root, 'AXTable')
        assert len(tables) == 1, len(tables)
        table = tables[0]
        assert number(table, 'AXRowCount') == 3
        assert number(table, 'AXColumnCount') == 2
        rows = mac.children(table, 'AXRows')
        assert len(rows) == 3, len(rows)
        headers = mac.children(table, 'AXColumnHeaderUIElements')
        row_headers = mac.children(table, 'AXRowHeaderUIElements')
        assert len(headers) == 2 and not row_headers, (len(headers), len(row_headers))
        header_group = mac.attr(table, 'AXHeader')
        assert header_group and equal(header_group, rows[0]), 'Table header must be its painted first row'
        expected = [('Idea', 'Next step'), ('Native text', 'Keep 世界 readable'),
                    ('Small details', 'Review together')]
        for row_index, row in enumerate(rows):
            assert number(row, 'AXIndex') == row_index
            assert not any(equal(row, prior) for prior in rows[:row_index])
            cells = descendants(row, 'AXCell')
            all_cells.extend(cells)
            assert len(cells) == 2, (row_index, len(cells))
            for column_index, cell in enumerate(cells):
                assert index_range(cell, 'AXRowIndexRange') == (row_index, 1)
                assert index_range(cell, 'AXColumnIndexRange') == (column_index, 1)
                if row_index == 0:
                    assert equal(headers[column_index], cell), 'Header relation duplicated or reordered a cell'
                labels = descendants(cell, 'AXStaticText')
                try:
                    text = ''.join(mac.text(label, 'AXValue') or mac.text(label, 'AXTitle') or ''
                                   for label in labels)
                    assert text == expected[row_index][column_index], (row_index, column_index, text)
                finally:
                    for label in labels:
                        mac.release(label)
        assert all(not equal(cell, prior) for i, cell in enumerate(all_cells)
                   for prior in all_cells[:i]), 'Cells from different rows share identity'
        print('GALLERY_DOCUMENT_STRUCTURE_OK: heading level, table counts and header relationships, distinct rows/cells, '
              'zero-based indices and Unicode reading order', flush=True)
    finally:
        if header_group:
            mac.release(header_group)
        for node in [*headings, *tables, *rows, *all_cells, *headers, *row_headers, root]:
            mac.release(node)


def reveal_document_control(mac, label="Copy code", role="AXButton"):
    # First reveal the nested viewport through its outer page. AX reports its
    # full layout rectangle even when a parent clips it. Target the card padding
    # just left of the document so the inner scroller cannot consume this wheel.
    body = mac.wait_find(TITLE, 'Document content', 'AXGroup')
    window = mac.window(TITLE)
    try:
        x, _, _, _ = element_rect(mac, body)
        wx, _, ww, _ = element_rect(mac, window)
    finally:
        mac.release(body)
        mac.release(window)
    fraction = (x - wx - 10) / ww
    assert 0 < fraction < 1, ('Document outer scroll target', fraction)
    reveal_gallery_control(mac, 'Document content', 'AXGroup', scroll_fraction=fraction)
    # Scroll the actual native viewport until the requested block/control mounts.
    # A baseline code fence can exist before later appended content is visible.
    raise_gallery(mac)
    mouse = GalleryMouse(mac)
    create = mac.cg.CGEventCreateScrollWheelEvent
    create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
    locate = mac.cg.CGEventSetLocation
    locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]
    previous = None
    for _ in range(20):
        # AX can include mounted overscan controls that are still clipped. A
        # keyboard handler is usable only after its element actually paints.
        current = mouse.bounds('Document content')
        x, y, width, height = current
        assert width > 0 and height > 0, current
        amount = -80
        control = mac.find(TITLE, label, role)
        if control:
            try:
                tx, ty, tw, th = element_rect(mac, control)
            finally:
                mac.release(control)
            if tw <= 0 or th <= 0:
                time.sleep(.1)
                continue
            if ty >= y - 1 and ty + th <= y + height + 1:
                return
            distance = y - ty if ty < y else ty + th - (y + height)
            amount = round(min(200, max(40, distance + 8))) * (1 if ty < y else -1)
            print('DOCUMENT_CONTROL_CLIPPED', label, (tx, ty, tw, th), current, flush=True)
        # Target the actual body, following any outer-page or document reflow.
        point = (x + width / 2, y + height / 2)
        if current != previous:
            print('DOCUMENT_SCROLL_TARGET', current, 'point', point, flush=True)
            mouse.send(5, point)
            time.sleep(.1)
            previous = current
        mouse.check_owner(point)
        event = create(None, 0, 1, C.c_int(amount))
        assert event
        try:
            locate(event, GalleryMouse.Point(*point))
            mouse.post(0, event)
        finally:
            mac.release(event)
        time.sleep(.1)
    print('DOCUMENT_SCROLL_FINAL_BODY', mouse.bounds('Document content'), flush=True)
    raise RuntimeError(f'{label!r} did not become accessible after native document scrolling')


def document_reading_order(mac):
    observed = []
    def visit(node):
        values, children = mac.node_values(node)
        try:
            if values[0] in ('AXStaticText', 'AXLink', 'AXImage'):
                observed.append((values[0], next((v for v in values[1:] if v), '')))
            for child in children:
                visit(child)
        finally:
            for child in children:
                mac.release(child)
    root = mac.wait_find(TITLE, 'Document content', 'AXGroup')
    try:
        visit(root)
    finally:
        mac.release(root)
    return observed


def document_link_reading_order(mac):
    """A mixed-font link and image alternative each occupy one source position."""
    observed = document_reading_order(mac)
    start = observed.index(('AXStaticText', 'Before '))
    assert observed[start:start + 5] == [
        ('AXStaticText', 'Before '), ('AXLink', 'Read the design notes'),
        ('AXStaticText', ' and '), ('AXLink', '世界 guide'),
        ('AXStaticText', ' after.'),
    ], observed
    assert not any('[Image: 世界 guide]' == text for _, text in observed), observed


def exercise_document_links(mac, images):
    mac.press(TITLE, 'Markdown & code')
    mac.wait_text(TITLE, 'Markdown preview')
    mac.wait_text(TITLE, 'A place for ideas')
    mac.wait_text(TITLE, '世界 · 👨‍👩‍👧‍👦')
    mac.wait_text(TITLE, 'Explore a direction')
    mac.wait_text(TITLE, 'Read the design notes')
    roles = tree_counts(mac, mac.wait_find(TITLE, 'Markdown preview', 'AXGroup'))
    # Frontmatter detection defaults to Disabled: the metadata followed by ---
    # is ordinary Markdown (a Setext heading), alongside the explicit title.
    assert roles.get('AXHeading') == 2 and roles.get('AXList') == 1, roles
    assert roles.get('AXLink') == 2, roles
    document_link_reading_order(mac)
    document_structure(mac)
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    if images:
        screenshot(mac, images / f'gallery-document-table-{current.lower()}.png', title=TITLE)
    mac.press(TITLE, current)
    mac.release(mac.wait_find(TITLE, alternate, 'AXButton'))
    document_structure(mac)
    if images:
        screenshot(mac, images / f'gallery-document-table-{alternate.lower()}.png', title=TITLE)
    mac.press(TITLE, alternate)
    mac.release(mac.wait_find(TITLE, current, 'AXButton'))
    document_structure(mac)
    for label in ['Read the design notes', '世界 guide']:
        link = mac.wait_find(TITLE, label, 'AXLink')
        try:
            mac.set(link, 'AXFocused', mac.true)
            expect_focus(mac, label, 'AXLink')
            mac.wait_text(TITLE, 'Ready to explore')  # Focusing does not navigate.
        finally:
            mac.release(link)
    mac.key(48, 1 << 17)
    expect_focus(mac, 'Read the design notes', 'AXLink')
    mac.key(53)
    expect_focus(mac, 'Document content', 'AXGroup')
    link = mac.wait_find(TITLE, 'Read the design notes', 'AXLink')
    try:
        mac.perform(link, 'AXPress')
    finally:
        mac.release(link)
    mac.wait_text(TITLE, 'Link requested: gpuio-preview:notes')
    link = mac.wait_find(TITLE, '世界 guide', 'AXLink')
    try:
        mac.perform(link, 'AXPress')
    finally:
        mac.release(link)
    mac.wait_text(TITLE, 'Link requested: gpuio-preview:unicode')
    stale_link = mac.wait_find(TITLE, 'Read the design notes', 'AXLink')
    try:
        mac.press(TITLE, 'Collapse')
        mac.release(mac.wait_find(TITLE, 'Expand', 'AXButton'))
        wait_absent(mac, 'Read the design notes', 'AXLink')
        expand = mac.wait_find(TITLE, 'Expand', 'AXButton')
        try:
            mac.set(expand, 'AXFocused', mac.true)
        finally:
            mac.release(expand)
        attribute = mac.string('AXFocused')
        try:
            # A retained OS target must not steal focus from collapsed content.
            mac.set_attr(stale_link, attribute, mac.true)
        finally:
            mac.release(attribute)
        press = mac.string('AXPress')
        try:
            # An OS-retained reference may report success for an asynchronous
            # action; either way it must not navigate the collapsed document.
            mac.action(stale_link, press)
        finally:
            mac.release(press)
        time.sleep(.1)
        expect_focus(mac, 'Expand')
        mac.wait_text(TITLE, 'Link requested: gpuio-preview:unicode')
        mac.press(TITLE, 'Expand')
        mac.release(mac.wait_find(TITLE, 'Read the design notes', 'AXLink'))
        document_structure(mac)
    finally:
        mac.release(stale_link)
    mac.press(TITLE, 'Append a finding')
    mac.wait_text(TITLE, 'Appended findings: 1 / 6')
    reveal_document_control(mac)
    mac.release(mac.wait_find(TITLE, 'Copy code', 'AXButton'))
    reveal_document_control(mac, 'Finding 1', 'AXStaticText')
    mac.wait_text(TITLE, 'Finding 1')
    for finding in range(2, 7):
        mac.press(TITLE, 'Append a finding')
        mac.wait_text(TITLE, f'Appended findings: {finding} / 6')
    focus = mac.wait_find(TITLE, 'Copy source', 'AXButton')
    try:
        mac.set(focus, 'AXFocused', mac.true)
    finally:
        mac.release(focus)
    expect_focus(mac, 'Copy source')
    mac.key(48)  # Tab from toolbar to the document's native focus owner.
    expect_focus(mac, 'Document content', 'AXGroup')
    for destination in ('notes', 'unicode', *(f'finding-{i}' for i in range(1, 7))):
        mac.key(48)
        label = {'notes': 'Read the design notes', 'unicode': '世界 guide'}.get(
            destination, 'Explore ' + destination.replace('-', ' '))
        expect_focus(mac, label, 'AXLink')
        mac.key(36)
        mac.wait_text(TITLE, 'Link requested: gpuio-preview:' + destination)
    def bounds(node):
        values = []
        get = mac.ax.AXValueGetValue
        get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        for attribute, kind in [('AXPosition', 1), ('AXSize', 2)]:
            value = mac.attr(node, attribute)
            point = (C.c_double * 2)()
            try:
                assert value and get(value, kind, C.byref(point)), attribute
                values.extend(point)
            finally:
                if value:
                    mac.release(value)
        return values
    content = mac.wait_find(TITLE, 'Document content', 'AXGroup')
    link = mac.wait_find(TITLE, 'Explore finding 6', 'AXLink')
    try:
        x, y, width, height = bounds(content)
        lx, ly, lw, lh = bounds(link)
        assert x <= lx and lx + lw <= x + width + 2, (bounds(content), bounds(link))
        assert y <= ly and ly + lh <= y + height + 2, (bounds(content), bounds(link))
    finally:
        mac.release(content)
        mac.release(link)
    if images:
        screenshot(mac, images / 'gallery-document-keyboard-link.png', title=TITLE)
    mac.key(48, 1 << 17)  # Shift-Tab returns to the preceding logical link.
    mac.key(36)
    mac.wait_text(TITLE, 'Link requested: gpuio-preview:finding-5')
    mac.key(53)  # Escape clears link focus without leaving the document.
    expect_focus(mac, 'Document content', 'AXGroup')
    link = mac.wait_find(TITLE, 'Explore finding 6', 'AXLink')
    try:
        mac.set(link, 'AXFocused', mac.true)
        expect_focus(mac, 'Explore finding 6', 'AXLink')
        mac.wait_text(TITLE, 'Link requested: gpuio-preview:finding-5')
        mac.key(36)
        mac.wait_text(TITLE, 'Link requested: gpuio-preview:finding-6')
    finally:
        mac.release(link)
    mac.key(53)
    print('GALLERY_DOCUMENT_LINKS_OK: rich text/code and safe-image alternative, '
          'source reading order, direct AX focus without activation, AX press, '
          'Tab/Enter/reverse/Escape, retained collapse guard and far reveal', flush=True)


def exercise_document_images(mac, images):
    mac.press(TITLE, 'Markdown & code')
    mac.press(TITLE, 'Image alternatives')
    mac.wait_text(TITLE, 'Image alternatives preview')
    for name in ['Prism 世界', 'Reference 世界']:
        mac.release(mac.wait_find(TITLE, name, 'AXImage'))
    observed = document_reading_order(mac)
    assert observed == [
        ('AXStaticText', 'Before '), ('AXImage', 'Prism 世界'),
        ('AXStaticText', ' after.'),
        ('AXStaticText', 'Before '), ('AXLink', 'Linked 世界'),
        ('AXStaticText', ' after.'),
        ('AXStaticText', 'Decoration '), ('AXStaticText', ' stays quiet.'),
        ('AXStaticText', 'Unnamed '), ('AXLink', 'gpuio-preview:unnamed'),
        ('AXStaticText', ' link.'),
        ('AXStaticText', 'Missing '), ('AXStaticText', '[Image: Unavailable 世界]'),
        ('AXStaticText', ' stays readable.'), ('AXImage', 'Reference 世界'),
    ], observed
    for label, destination in [('Linked 世界', 'prism'), ('gpuio-preview:unnamed', 'unnamed')]:
        link = mac.wait_find(TITLE, label, 'AXLink')
        try:
            mac.set(link, 'AXFocused', mac.true)
            expect_focus(mac, label, 'AXLink')
            mac.perform(link, 'AXPress')
            mac.wait_text(TITLE, 'Link requested: gpuio-preview:' + destination)
        finally:
            mac.release(link)
    if images:
        screenshot(mac, images / 'gallery-document-images.png', title=TITLE)
    mac.press(TITLE, 'Collapse')
    wait_absent(mac, 'Prism 世界', 'AXImage')
    wait_absent(mac, 'Linked 世界', 'AXLink')
    mac.press(TITLE, 'Expand')
    mac.release(mac.wait_find(TITLE, 'Prism 世界', 'AXImage'))
    mac.press(TITLE, 'Presentation')
    wait_absent(mac, 'Prism 世界', 'AXImage')
    mac.press(TITLE, 'Markdown & code')
    mac.press(TITLE, 'Image alternatives')
    mac.release(mac.wait_find(TITLE, 'Reference 世界', 'AXImage'))
    mac.press(TITLE, 'Markdown')
    mac.wait_text(TITLE, 'Markdown preview')
    print('GALLERY_DOCUMENT_IMAGES_OK: decoded/reference images, single linked alternatives, '
          'decorative omission, safe placeholder, AX focus/activation, collapse and remount', flush=True)


def assert_reset_document_source(mac):
    # The baseline now contains a code fence, so Copy code remains legitimate.
    # Verify source reset directly rather than relying on viewport virtualization.
    env = dict(os.environ, LANG='en_US.UTF-8', LC_ALL='en_US.UTF-8')
    saved = subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=env).stdout
    try:
        subprocess.run(['/usr/bin/pbcopy'], input=b'GPUIO source pending', check=True, env=env)
        mac.press(TITLE, 'Copy source')
        deadline = time.monotonic() + 5
        while True:
            source = subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=env).stdout.decode('utf-8')
            if '# A place for ideas' in source:
                assert 'let next_step = "Explore"' in source, source
                assert '## Finding ' not in source, source
                return
            assert time.monotonic() < deadline, source
            time.sleep(.05)
    finally:
        subprocess.run(['/usr/bin/pbcopy'], input=saved, check=True, env=env)


def exercise_documents(mac, images):
    exercise_document_images(mac, images)
    exercise_document_links(mac, images)
    mac.press(TITLE, 'Code')
    mac.wait_text(TITLE, 'Code preview')
    editor = mac.wait_find(TITLE, 'Code preview', 'AXTextArea')
    try:
        value = mac.text(editor, 'AXValue')
        assert value and 'let greeting name =' in value and '世界' in value, value
        settable = C.c_bool()
        check = mac.ax.AXUIElementIsAttributeSettable
        check.restype = C.c_int
        check.argtypes = [C.c_void_p, C.c_void_p, C.POINTER(C.c_bool)]
        attribute = mac.string('AXValue')
        try:
            assert check(editor, attribute, C.byref(settable)) == 0
            assert not settable.value, 'Read-only document advertises AX value replacement'
        finally:
            mac.release(attribute)
        mac.set(editor, 'AXFocused', mac.true)
        expect_focus(mac, 'Code preview', 'AXTextArea')
        mac.key(0, 1 << 20)  # Command-A: select the native document page.
        mac.key(51)  # Backspace must preserve the selected read-only text.
        mac.key(0)  # Typing must also preserve it.
        time.sleep(0.1)
        assert mac.text(editor, 'AXValue') == value
    finally:
        mac.release(editor)
    mac.release(mac.wait_find(TITLE, 'Copy source', 'AXButton'))
    activate(mac, mac.wait_find(TITLE, 'Highlight let', 'AXCheckBox'))
    mac.press(TITLE, 'Diff')
    mac.wait_text(TITLE, 'Diff preview')
    editor = mac.wait_find(TITLE, 'Diff preview', 'AXTextArea')
    try:
        value = mac.text(editor, 'AXValue')
        assert value and '-let greeting' in value and '+let greeting' in value, value
    finally:
        mac.release(editor)
    exercise_diff_controls(mac, images)
    mac.press(TITLE, 'Collapse')
    mac.release(mac.wait_find(TITLE, 'Expand', 'AXButton'))
    wait_absent(mac, 'Diff preview', 'AXTextArea')
    mac.press(TITLE, 'Expand')
    mac.release(mac.wait_find(TITLE, 'Collapse', 'AXButton'))
    mac.release(mac.wait_find(TITLE, 'Diff preview', 'AXTextArea'))
    mac.press(TITLE, 'Markdown')
    reveal_document_control(mac)
    mac.release(mac.wait_find(TITLE, 'Copy code', 'AXButton'))
    if images:
        screenshot(mac, images / 'gallery-documents.png', title=TITLE)
    mac.press(TITLE, 'Reset document')
    mac.wait_text(TITLE, 'Document reset')
    assert_reset_document_source(mac)
    wait_absent(mac, 'Finding 1', 'AXStaticText')
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Markdown preview', 'AXGroup')
        mac.press(TITLE, 'Markdown & code')
        mac.wait_text(TITLE, 'Markdown preview')
        document_structure(mac)
        assert_reset_document_source(mac)
        mac.press(TITLE, 'Append a finding')
        mac.wait_text(TITLE, 'Appended findings: 1 / 6')
        reveal_document_control(mac)
        mac.release(mac.wait_find(TITLE, 'Copy code', 'AXButton'))
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A little context goes a long way')


def exercise_diff_controls(mac, images):
    def source(*, present=(), missing=()):
        deadline = time.monotonic() + 10
        value = None
        while time.monotonic() < deadline:
            value = mac.field(TITLE, 'Diff preview', 'AXTextArea')
            if value is not None and all(s in value for s in present) and all(s not in value for s in missing):
                return value
            time.sleep(.025)
        raise RuntimeError(f'Diff source mismatch: present={present!r}, missing={missing!r}, value={value!r}')

    mac.release(mac.wait_find(TITLE, 'Collapse file greeting.ml', 'AXButton'))
    mac.release(mac.wait_find(TITLE, 'Show more diff lines', 'AXButton'))
    mac.press(TITLE, 'Collapse file greeting.ml')
    mac.wait_text(TITLE, 'Collapsed greeting.ml')
    source(present=('--- a/greeting.ml', 'settings.json'), missing=('-let greeting',))
    mac.press(TITLE, 'Expand file greeting.ml')
    mac.wait_text(TITLE, 'Expanded greeting.ml')
    source(present=('-let greeting', '+let greeting'))
    mac.press(TITLE, 'Show more diff lines')
    mac.wait_text(TITLE, 'Showing up to 8 changed and context lines')
    full = source(present=('"language": "世界"',))
    wait_absent(mac, 'Show more diff lines', 'AXButton')
    activate(mac, mac.wait_find(TITLE, 'Emphasize changed words', 'AXCheckBox'))
    expect_field(mac, TITLE, 'Diff preview', full, role='AXTextArea')

    activate(mac, mac.wait_find(TITLE, 'Application controls expansion', 'AXCheckBox'))
    mac.release(mac.wait_find(TITLE, 'Show more diff lines', 'AXButton'))
    mac.press(TITLE, 'Collapse file settings.json')
    mac.wait_text(TITLE, 'Collapsed settings.json')
    source(present=('--- a/settings.json',), missing=('"theme"',))
    mac.press(TITLE, 'Expand file settings.json')
    mac.wait_text(TITLE, 'Expanded settings.json')
    mac.press(TITLE, 'Show more diff lines')
    mac.wait_text(TITLE, 'Showing up to 8 changed and context lines')
    expect_field(mac, TITLE, 'Diff preview', full, role='AXTextArea')

    focus_gallery_control(mac, 'Diff preview', 'AXTextArea')
    mac.key(126, 1 << 20)  # Command-Up: canonical first source row.
    for _ in range(3):
        mac.key(125)  # Down to the removed OCaml line.
    mac.key(36)
    mac.wait_text(TITLE, 'Selected greeting.ml · old 1 → new — · let greeting = "Hello"')
    mac.press(TITLE, 'Append a file')
    mac.wait_text(TITLE, 'Appended files: 1 / 3')
    mac.press(TITLE, 'Show more diff lines')
    mac.wait_text(TITLE, 'Showing up to 11 changed and context lines')
    expanded = source(present=('worker-1.rs', '+fn main()', 'println!'))
    cycle_preview_appearance(mac, 'Appended files: 1 / 3')
    expect_field(mac, TITLE, 'Diff preview', expanded, role='AXTextArea')
    if images:
        screenshot(mac, images / 'gallery-diff-controls.png', title=TITLE)
    mac.press(TITLE, 'Reset diff')
    mac.wait_text(TITLE, 'Diff reset')
    source(present=('-let greeting',), missing=('worker-1.rs', '"language"'))
    mac.release(mac.wait_find(TITLE, 'Show more diff lines', 'AXButton'))
    print('GALLERY_DIFF_OK: managed/controlled collapse and preview, public queued file/show-more/line events, '
          'native keyboard activation, word-toggle source preservation, streamed file append, theme/scale retention and generation reset', flush=True)


def wait_absent(mac, label, role):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        node = mac.find(TITLE, label, role)
        if not node:
            return
        mac.release(node)
        time.sleep(.03)
    raise RuntimeError(f'Removed content is still accessible: {label}')






def select_gallery_appearance(mac, desired):
    assert desired in ('Light', 'Dark')
    current = mac.find(TITLE, desired, 'AXButton')
    if current:
        mac.release(current)
        return
    # The shell labels its toggle with the current mode, not the destination.
    mac.press(TITLE, 'Dark' if desired == 'Light' else 'Light')
    mac.release(mac.wait_find(TITLE, desired, 'AXButton'))


def cycle_preview_appearance(mac, expected):
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    mac.press(TITLE, current)
    mac.release(mac.wait_find(TITLE, alternate, 'AXButton'))
    mac.wait_text(TITLE, expected)
    for size, next_size in [('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')]:
        mac.press(TITLE, size)
        mac.release(mac.wait_find(TITLE, next_size, 'AXButton'))
        mac.wait_text(TITLE, expected)
    mac.press(TITLE, alternate)
    mac.release(mac.wait_find(TITLE, current, 'AXButton'))
    mac.wait_text(TITLE, expected)


def element_rect(mac, node):
    result = []
    get = mac.ax.AXValueGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
    for attribute, kind in [('AXPosition', 1), ('AXSize', 2)]:
        value = mac.attr(node, attribute)
        pair = (C.c_double * 2)()
        try:
            assert value and get(value, kind, C.byref(pair)), attribute
            result.extend(pair)
        finally:
            if value:
                mac.release(value)
    return result


def canvas_shape_rect(mac):
    node = mac.wait_find(TITLE, 'Orbit', 'AXStaticText')
    try:
        return element_rect(mac, node)
    finally:
        mac.release(node)



def drag_canvas_orbit(mac):
    class Point(C.Structure):
        _fields_ = [('x', C.c_double), ('y', C.c_double)]
    window = mac.window(TITLE)
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    expect_enabled(mac, 'Orbit', True, 'AXStaticText')
    x, y, width, height = canvas_shape_rect(mac)
    start = Point(x + width/2, y + height/2)
    finish = Point(start.x + 24, start.y + 16)
    system = mac.ax.AXUIElementCreateSystemWide
    system.restype, system.argtypes = C.c_void_p, []
    hit_test = mac.ax.AXUIElementCopyElementAtPosition
    hit_test.restype, hit_test.argtypes = C.c_int, [C.c_void_p, C.c_float, C.c_float, C.POINTER(C.c_void_p)]
    get_pid = mac.ax.AXUIElementGetPid
    get_pid.restype, get_pid.argtypes = C.c_int, [C.c_void_p, C.POINTER(C.c_int)]
    root, hit, owner = system(), C.c_void_p(), C.c_int()
    try:
        assert (not hit_test(root, start.x, start.y, C.byref(hit)) and hit.value
                and not get_pid(hit, C.byref(owner)) and owner.value == mac.pid), 'Canvas pointer target is occluded'
    finally:
        if hit.value:
            mac.release(hit)
        mac.release(root)
    create = mac.cg.CGEventCreateMouseEvent
    create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, Point, C.c_int]
    post = mac.cg.CGEventPost
    post.restype, post.argtypes = None, [C.c_int, C.c_void_p]
    def send(kind, point):
        event = create(None, kind, point, 0)
        assert event, 'Cannot create canvas mouse event'
        try:
            post(0, event)
        finally:
            mac.release(event)
    try:
        send(1, start)
        time.sleep(.05)
        send(6, finish)
        time.sleep(.05)
    finally:
        send(2, finish)
    mac.wait_text(TITLE, 'Selected: Orbit · x 134 · y 146')
    after = canvas_shape_rect(mac)
    assert abs(after[0]-x-24) < 1 and abs(after[1]-y-16) < 1, ((x, y), after)


def exercise_canvas(mac, images):
    mac.press(TITLE, 'Canvas & drawing')
    expect_enabled(mac, 'Orbit', True, 'AXStaticText')
    window = mac.window(TITLE)
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    original = canvas_shape_rect(mac)
    node = mac.wait_find(TITLE, 'Orbit', 'AXStaticText')
    try:
        mac.set(node, 'AXFocused', mac.true)
        expect_focus(mac, 'Orbit', 'AXStaticText')
    finally:
        mac.release(node)
    mac.wait_text(TITLE, 'Selected: Orbit · x 110 · y 130')
    for key, x, y in [(124, 111, 130), (124, 112, 130), (125, 112, 131)]:
        window = mac.window(TITLE)
        try:
            mac.set(mac.app, 'AXFrontmost', mac.true)
            mac.perform(window, 'AXRaise')
        finally:
            mac.release(window)
        expect_enabled(mac, 'Orbit', True, 'AXStaticText')
        # Publication must preserve native focus; do not refocus the object to
        # conceal a focus loss during the source echo.
        expect_focus(mac, 'Orbit', 'AXStaticText')
        mac.key(key, 1 << 17)  # Shift + arrow: commit a native world-space move.
        mac.wait_text(TITLE, f'Selected: Orbit · x {x} · y {y}')
    moved = canvas_shape_rect(mac)
    assert abs(moved[0] - original[0] - 2) < 1, (original, moved)
    assert abs(moved[1] - original[1] - 1) < 1, (original, moved)
    expect_enabled(mac, 'Activate Orbit', True)
    mac.press(TITLE, 'Activate Orbit')
    mac.wait_text(TITLE, 'Last activation: Orbit')
    mac.press(TITLE, 'Zoom to 125%')
    mac.wait_text(TITLE, 'Canvas zoom: 125%')
    zoomed = canvas_shape_rect(mac)
    assert abs(zoomed[2] - original[2] * 1.25) < 1, (original, zoomed)
    mac.press(TITLE, 'Reset canvas view')
    mac.wait_text(TITLE, 'Canvas zoom: 100%')
    activate(mac, mac.wait_find(TITLE, 'Disable canvas input', 'AXCheckBox'))
    expect_enabled(mac, 'Orbit', False, 'AXStaticText')
    mac.press(TITLE, 'Select Prism')  # Explicit commands still work while input is disabled.
    mac.wait_text(TITLE, 'Selected: Prism · x 290 · y 100')
    activate(mac, mac.wait_find(TITLE, 'Disable canvas input', 'AXCheckBox'))
    expect_enabled(mac, 'Orbit', True, 'AXStaticText')
    mac.press(TITLE, 'Hide canvas')
    mac.release(mac.wait_find(TITLE, 'Show canvas', 'AXButton'))
    absent(mac, 'Orbit', 'AXStaticText')
    mac.press(TITLE, 'Show canvas')
    expect_enabled(mac, 'Orbit', True, 'AXStaticText')
    mac.press(TITLE, 'Select Orbit')
    mac.wait_text(TITLE, 'Selected: Orbit · x 112 · y 131')
    mac.press(TITLE, 'Reset canvas scene')
    mac.wait_text(TITLE, 'Canvas reset')
    deadline = time.monotonic() + 10
    while True:
        reset = canvas_shape_rect(mac)
        if abs(reset[0] - original[0]) < .5 and abs(reset[1] - original[1]) < .5:
            break
        assert time.monotonic() < deadline, (original, reset)
        time.sleep(.03)
    mac.press(TITLE, 'Select Orbit')
    mac.wait_text(TITLE, 'Selected: Orbit · x 110 · y 130')
    drag_canvas_orbit(mac)
    cycle_preview_appearance(mac, 'Selected: Orbit · x 134 · y 146')
    if images:
        screenshot(mac, images / 'gallery-canvas.png', title=TITLE)
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Orbit', 'AXStaticText')
        mac.press(TITLE, 'Canvas & drawing')
        expect_enabled(mac, 'Orbit', True, 'AXStaticText')
        mac.wait_text(TITLE, 'No shape activated yet')
        mac.press(TITLE, 'Select Tile')
        mac.wait_text(TITLE, 'Selected: Tile · x 470 · y 160')
    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Refresh resource counts')
    wait_for_resource_cleanup(mac)
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    print('GALLERY_CANVAS_OK: OS keyboard/pointer movement, source echo, native geometry/zoom, '
          'disabled commands, retained hide/show, reset and scoped cleanup', flush=True)


def exercise_assets(mac, images):
    mac.press(TITLE, 'Images & icons')
    mac.wait_text(TITLE, 'Image ready:')
    for label in ['Gallery image', 'Gradient thumbnail', 'Check mark']:
        mac.release(mac.wait_find(TITLE, label, 'AXImage'))
    roles = tree_counts(mac, mac.window(TITLE))
    assert roles.get('AXImage') == 3, roles  # Button decoration has no second image target.
    mac.press(TITLE, 'Show raster gradient')
    mac.wait_text(TITLE, 'Image ready: 96 × 48 pixels · 1 frame(s)')
    for fit in ['Cover', 'Fill', 'Scale down', 'Intrinsic size', 'Contain']:
        mac.press(TITLE, fit)
        mac.wait_text(TITLE, 'Image fit: ' + fit)
        mac.wait_text(TITLE, 'Image ready: 96 × 48 pixels · 1 frame(s)')
    cycle_preview_appearance(mac, 'Image ready: 96 × 48 pixels · 1 frame(s)')
    mac.press(TITLE, 'Simulate decode failure')
    mac.wait_text(TITLE, 'Image failed: Invalid_data')
    mac.press(TITLE, 'Restore image')
    mac.wait_text(TITLE, 'Image ready: 96 × 48 pixels · 1 frame(s)')
    window = mac.window(TITLE)
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    button = mac.wait_find(TITLE, 'Approve sample', 'AXButton')
    try:
        mac.set(button, 'AXFocused', mac.true)
        expect_focus(mac, 'Approve sample')
        mac.key(49)
    finally:
        mac.release(button)
    mac.wait_text(TITLE, 'Sample approvals: 1')
    mac.press(TITLE, 'Approve sample')
    mac.wait_text(TITLE, 'Sample approvals: 2')
    mac.press(TITLE, 'Show vector landscape')
    mac.wait_text(TITLE, 'Image ready: 480 × 240 pixels · 1 frame(s)')
    if images:
        screenshot(mac, images / 'gallery-assets.png', title=TITLE)
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Gallery image', 'AXImage')
        mac.press(TITLE, 'Images & icons')
        mac.wait_text(TITLE, 'Image ready:')
        mac.wait_text(TITLE, 'Sample approvals: 2')
    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Refresh resource counts')
    wait_for_resource_cleanup(mac)
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    print('GALLERY_ASSETS_OK: SVG/raster readiness, fit controls, native decode failure/recovery, '
          'icon semantics, OS activation and scoped cleanup', flush=True)


def exercise_charts(mac, images):
    mac.press(TITLE, 'Charts & data')
    cases = [
        ('Line', 48, 48, 'Atlas · x 0 · value 30'),
        ('Area', 24, 24, 'Active capacity · x 0 · value 30'),
        ('Bar', 24, 24, 'Completed evaluations · x 0 · value 30'),
        ('Pie', 4, 4, 'Reasoning · 44'),
        ('Radar', 10, 10, 'Atlas · Quality · 88 / 100'),
        ('Candlestick', 24, 24, 'Session 1 · close 34'),
        ('Sankey', 4, 8, 'Incoming → Reasoning · 65'),
        ('Mixed layers', 72, 72, 'Capacity · x 0 · value 30'),
    ]
    for family, plotted, originals, first in cases:
        mac.press(TITLE, family)
        mac.wait_text(TITLE, f'Ready: {family} · {plotted} source values')
        focus_gallery_control(mac, 'Chart preview: ' + family, 'AXGroup')
        mac.key(115)  # Home previews without selecting.
        mac.wait_text(TITLE, 'Select a chart value to inspect it.')
        mac.key(36)
        mac.wait_text(TITLE, 'Selected: ' + first)
        if family == 'Line':
            mac.press(TITLE, 'Update chart samples')
            mac.wait_text(TITLE, 'Selected: Atlas · x 0 · value 33')
        if family == 'Pie':
            mac.press(TITLE, 'Update chart samples')  # Constant sample: no publication needed.
            mac.wait_text(TITLE, 'Ready: Pie · 4 source values')
            mac.wait_text(TITLE, 'Selected: Reasoning · 44')
        mac.press(TITLE, 'View data')
        table = mac.wait_find(TITLE, 'Chart preview: ' + family + ' · original data', 'AXTable')
        try:
            get = mac.cf.CFNumberGetValue
            get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
            value = mac.attr(table, 'AXRowCount')
            count = C.c_longlong()
            try:
                assert value and get(value, 4, C.byref(count)) and count.value == originals, (family, count.value)
            finally:
                if value:
                    mac.release(value)
            rows = mac.children(table, 'AXRows')
            try:
                assert 0 < len(rows) <= 10, (family, len(rows))
            finally:
                for row in rows:
                    mac.release(row)
        finally:
            mac.release(table)
        mac.key(119)
        mac.release(mac.wait_find(TITLE, f'Row {originals}:', 'AXRow', contains=True, search_files=True))
        mac.press(TITLE, 'Back to chart')
        mac.release(mac.wait_find(TITLE, 'View data', 'AXButton'))
    activate(mac, mac.wait_find(TITLE, 'Horizontal axes', 'AXCheckBox'))
    mac.wait_text(TITLE, 'Ready: Mixed layers · 72 source values')
    activate(mac, mac.wait_find(TITLE, 'Disable chart input', 'AXCheckBox'))
    expect_enabled(mac, 'Chart preview: Mixed layers', False, 'AXGroup')
    activate(mac, mac.wait_find(TITLE, 'Disable chart input', 'AXCheckBox'))
    expect_enabled(mac, 'Chart preview: Mixed layers', True, 'AXGroup')
    # Restore theme/size after exercising both. Choices and scoped data stay intact.
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, next_label = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    mac.press(TITLE, current)
    mac.release(mac.wait_find(TITLE, next_label, 'AXButton'))
    mac.wait_text(TITLE, 'Selected: Capacity · x 0 · value 30')
    for size, next_size in [('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')]:
        mac.press(TITLE, size)
        mac.release(mac.wait_find(TITLE, next_size, 'AXButton'))
        mac.wait_text(TITLE, 'Selected: Capacity · x 0 · value 30')
    if images:
        screenshot(mac, images / 'gallery-charts.png', title=TITLE)
    mac.press(TITLE, next_label)
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Chart preview: Mixed layers', 'AXGroup')
        mac.press(TITLE, 'Charts & data')
        mac.wait_text(TITLE, 'Ready: Mixed layers · 72 source values')
        mac.wait_text(TITLE, 'Select a chart value to inspect it.')
    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Refresh resource counts')
    wait_for_resource_cleanup(mac)
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    print('GALLERY_CHARTS_OK: seven families plus mixed layers, native keyboard selection, '
          'data updates, bounded original-data pages, styles and scope cleanup', flush=True)


class GalleryMouse:
    class Point(C.Structure):
        _fields_ = [('x', C.c_double), ('y', C.c_double)]

    def __init__(self, mac):
        self.mac = mac
        self.create = mac.cg.CGEventCreateMouseEvent
        self.create.restype, self.create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, self.Point, C.c_int]
        self.post = mac.cg.CGEventPost
        self.post.restype, self.post.argtypes = None, [C.c_int, C.c_void_p]

    def send(self, kind, point):
        event = self.create(None, kind, self.Point(*point), 0)
        assert event, 'Cannot create gallery mouse event'
        try:
            if kind in (1, 2, 3, 4, 25, 26):
                # Match real native down/up click-count metadata. A generated up
                # can otherwise carry zero, which is not a valid typed click.
                set_integer = self.mac.cg.CGEventSetIntegerValueField
                set_integer.restype, set_integer.argtypes = None, [C.c_void_p, C.c_int, C.c_longlong]
                set_integer(event, 1, 1)  # kCGMouseEventClickState.
            self.post(0, event)
        finally:
            self.mac.release(event)

    def bounds(self, label):
        node = self.mac.wait_find(TITLE, label, 'AXGroup')
        try:
            bounds = element_rect(self.mac, node)
        finally:
            self.mac.release(node)
        self.check_owner((bounds[0] + bounds[2]/2, bounds[1] + bounds[3]/2))
        return bounds

    def check_owner(self, point):
        mac = self.mac
        system = mac.ax.AXUIElementCreateSystemWide
        system.restype, system.argtypes = C.c_void_p, []
        hit_test = mac.ax.AXUIElementCopyElementAtPosition
        hit_test.restype, hit_test.argtypes = C.c_int, [C.c_void_p, C.c_float, C.c_float, C.POINTER(C.c_void_p)]
        get_pid = mac.ax.AXUIElementGetPid
        get_pid.restype, get_pid.argtypes = C.c_int, [C.c_void_p, C.POINTER(C.c_int)]
        root, hit, owner = system(), C.c_void_p(), C.c_int()
        try:
            hit_status = hit_test(root, *point, C.byref(hit))
            pid_status = get_pid(hit, C.byref(owner)) if not hit_status and hit.value else None
            assert (not hit_status and hit.value and pid_status == 0
                    and owner.value == mac.pid), (
                        'Pointer target is occluded', point,
                        {'expected_pid': mac.pid, 'actual_pid': owner.value,
                         'hit_status': hit_status, 'pid_status': pid_status})
            subrole = mac.text(hit, 'AXSubrole')
            assert subrole not in ('AXCloseButton', 'AXMinimizeButton', 'AXZoomButton'), (
                'Content pointer target hit a window control', point, subrole)
        finally:
            if hit.value:
                mac.release(hit)
            mac.release(root)

    def transfer(self, *, cancel=False):
        sx, sy, sw, sh = self.bounds('Idea transfer source')
        tx, ty, tw, th = self.bounds('Idea transfer inbox')
        start, finish = (sx+sw/2, sy+sh/2), (tx+tw/2, ty+th/2)
        try:
            self.send(5, start)
            self.send(1, start)
            time.sleep(.05)
            for step in range(1, 13):
                point = tuple(a+(b-a)*step/12 for a, b in zip(start, finish))
                self.send(6, point)
                time.sleep(.02)
            if cancel:
                self.mac.key(53)
                self.mac.wait_text(TITLE, 'Transfer cancelled: Escape')
        finally:
            self.send(2, finish)


def exercise_observations(mac, images, *, second_title=SECOND):
    mac.press(TITLE, 'Input observations')
    mac.wait_text(TITLE, 'Every interaction has a story')
    expect_field(mac, TITLE, 'Observation draft', 'Small ideas grow here.')
    raise_gallery(mac)
    mouse = GalleryMouse(mac)
    x, y, width, height = mouse.bounds('Input observation surface')
    point = (x + 40, y + height - 22)
    mouse.check_owner(point)
    mouse.send(5, point)
    mouse.send(1, point)
    mouse.send(2, point)
    mac.wait_text(TITLE, 'Clicks: 1')
    mac.wait_text(TITLE, 'Focus: surface')
    mac.key(0)  # A raw key while the region itself owns focus.
    mac.wait_text(TITLE, 'Key: a')
    mac.key(48)
    expect_focus(mac, 'Observation draft', 'AXTextField')
    mac.key(48, flags=1 << 17)
    mac.wait_text(TITLE, 'Focus: surface')
    field = mac.wait_find(TITLE, 'Observation draft', 'AXTextField')
    try:
        mac.set(field, 'AXFocused', mac.true)
        mac.key(0, flags=1 << 20)
        mac.key(0)
    finally:
        mac.release(field)
    expect_field(mac, TITLE, 'Observation draft', 'a')
    mac.key(105)  # F13 is unbound in the retained editor, reaches capture.
    mac.wait_text(TITLE, 'Key: f13')
    mac.press(TITLE, 'Use bubble listeners')
    expect_field(mac, TITLE, 'Observation draft', 'a')
    mac.press(TITLE, 'Use capture listeners')
    expect_field(mac, TITLE, 'Observation draft', 'a')
    mac.press(TITLE, 'Disable observations')
    mac.wait_text(TITLE, 'Observations paused')
    mouse.send(1, point)
    mouse.send(2, point)
    mac.wait_text(TITLE, 'Clicks: 1')
    mac.press(TITLE, 'Enable observations')
    mac.release(mac.wait_find(TITLE, 'Disable observations', 'AXButton'))
    mouse.send(5, point)
    mouse.send(1, point)
    mouse.send(2, point)
    mac.wait_text(TITLE, 'Clicks: 2')
    expect_field(mac, TITLE, 'Observation draft', 'a')
    front = mac.wait_find(TITLE, 'Save idea', 'AXButton')
    try:
        fx, fy, fw, fh = element_rect(mac, front)
    finally:
        mac.release(front)
    front_point = (fx + fw/2, fy + fh/2)
    mouse.check_owner(front_point)
    mouse.send(5, front_point)
    mouse.send(1, front_point)
    mouse.send(2, front_point)
    mac.wait_text(TITLE, 'Floating action: 1')
    mac.wait_text(TITLE, 'Down: 2')
    mac.wait_text(TITLE, 'Clicks: 2')
    # The combined run preserves shell appearance across preceding sections.
    # Exercise both themes and restore the current one, rather than assuming
    # the fresh-process default used by a focused observations run.
    theme = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if theme else ('Light', 'Dark')
    if theme:
        mac.release(theme)
    if images:
        screenshot(mac, images / f'gallery-observations-{current.lower()}.png', title=TITLE)
    mac.press(TITLE, current)
    mac.release(mac.wait_find(TITLE, alternate, 'AXButton'))
    expect_field(mac, TITLE, 'Observation draft', 'a')
    if images:
        screenshot(mac, images / f'gallery-observations-{alternate.lower()}.png', title=TITLE)
    mac.press(TITLE, alternate)
    mac.release(mac.wait_find(TITLE, current, 'AXButton'))
    mac.press(TITLE, 'New window')
    mac.wait_text(second_title, 'A little context goes a long way')
    mac.press(second_title, 'Input observations')
    expect_field(mac, second_title, 'Observation draft', 'Small ideas grow here.')
    expect_field(mac, TITLE, 'Observation draft', 'a')
    mac.wait_text(second_title, 'Clicks: 0')
    mac.close(second_title)
    raise_gallery(mac)
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A little context goes a long way')
    field = mac.find(TITLE, 'Observation draft', 'AXTextField')
    if field:
        mac.release(field)
        raise RuntimeError('Departed observation editor remains accessible')
    mac.press(TITLE, 'Input observations')
    expect_field(mac, TITLE, 'Observation draft', 'Small ideas grow here.')
    mac.wait_text(TITLE, 'Clicks: 0')
    print('GALLERY_OBSERVATIONS_OK: actual pointer/focus/raw keys, retained native editing, '
          'configuration updates, disabled routing, themes and page teardown', flush=True)


def exercise_input(mac, images):
    mac.press(TITLE, 'Input & transfers')
    mac.wait_text(TITLE, 'Panel width: 180 logical pixels')
    raise_gallery(mac)
    mouse = GalleryMouse(mac)
    x, y, width, height = mouse.bounds('Panel drag track')
    start, finish = (x+180, y+height/2), (x+260, y-8)
    mouse.check_owner(finish)
    try:
        mouse.send(5, start)
        mouse.send(1, start)
        mac.wait_text(TITLE, 'Pointer: Started')
        mouse.send(6, finish)  # Capture continues above the region.
        mac.wait_text(TITLE, 'Pointer: Moved')
        mac.wait_text(TITLE, 'Panel width: 260 logical pixels')
    finally:
        mouse.send(2, finish)
    mac.wait_text(TITLE, 'Pointer: Released')
    panel = mac.wait_find(TITLE, 'Sized panel', 'AXGroup')
    try:
        assert abs(element_rect(mac, panel)[2] - 260) < 1
    finally:
        mac.release(panel)
    mac.press(TITLE, 'Reset panel')
    mac.wait_text(TITLE, 'Panel width: 180 logical pixels')
    try:
        mouse.send(5, start)
        mouse.send(1, start)
        mac.wait_text(TITLE, 'Pointer: Started')
        mac.key(53)
        mac.wait_text(TITLE, 'Pointer: (Cancelled Escape)')
    finally:
        mouse.send(2, start)
    mac.wait_text(TITLE, 'Pointer: (Cancelled Escape)')
    try:
        mouse.send(5, start)
        mouse.send(1, start)
        mac.wait_text(TITLE, 'Pointer: Started')
        mac.press(TITLE, 'Disable pointer input')
        mac.wait_text(TITLE, 'Pointer: (Cancelled Disabled)')
    finally:
        mouse.send(2, start)
    mac.release(mac.wait_find(TITLE, 'Enable pointer input', 'AXButton'))
    try:
        mouse.send(1, start)
        mouse.send(6, finish)
    finally:
        mouse.send(2, finish)
    time.sleep(.15)
    mac.wait_text(TITLE, 'Panel width: 180 logical pixels')
    focus_gallery_control(mac, 'Widen panel', 'AXButton')
    mac.key(49)
    mac.wait_text(TITLE, 'Panel width: 200 logical pixels')
    mac.press(TITLE, 'Enable pointer input')
    mouse.transfer()
    mac.wait_text(TITLE, 'Transfer delivered')
    mac.wait_text(TITLE, 'Received text · 33 UTF-8 bytes')
    mac.wait_text(TITLE, 'Received items: 1')
    mouse.transfer(cancel=True)
    mac.wait_text(TITLE, 'Received items: 1')
    mac.press(TITLE, 'Use card payload')
    mouse.transfer()
    mac.wait_text(TITLE, 'Received org.gpuio.gallery.card/v1 · 7 bytes')
    mac.wait_text(TITLE, 'Received items: 2')
    mac.press(TITLE, 'Reject cards')
    expect_enabled(mac, 'Receive with keyboard or click', False)
    mouse.transfer()
    mac.wait_text(TITLE, 'Transfer ended without an accepted drop')
    mac.wait_text(TITLE, 'Received items: 2')
    mac.press(TITLE, 'Accept cards')
    mac.press(TITLE, 'Disable transfers')
    expect_enabled(mac, 'Receive with keyboard or click', False)
    mouse.transfer()
    time.sleep(.15)
    mac.wait_text(TITLE, 'Received items: 2')
    mac.press(TITLE, 'Enable transfers')
    focus_gallery_control(mac, 'Receive with keyboard or click', 'AXButton')
    mac.key(49)
    mac.wait_text(TITLE, 'Received items: 3')
    cycle_preview_appearance(mac, 'Received items: 3')
    mac.wait_text(TITLE, 'Panel width: 200 logical pixels')
    if images:
        screenshot(mac, images / 'gallery-input.png', title=TITLE)
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Panel drag track', 'AXGroup')
        absent(mac, 'Idea transfer source', 'AXGroup')
        mac.press(TITLE, 'Input & transfers')
        mac.wait_text(TITLE, 'Pointer: Ready to drag')
        mac.wait_text(TITLE, 'No active transfer')
        mac.wait_text(TITLE, 'Received items: 3')
        mac.wait_text(TITLE, 'Panel width: 200 logical pixels')
    x, y, width, height = mouse.bounds('Panel drag track')
    held = (x+180, y+height/2)
    try:
        mouse.send(5, held)
        mouse.send(1, held)
        mac.wait_text(TITLE, 'Pointer: Started')
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Panel drag track', 'AXGroup')
    finally:
        mouse.send(2, held)
    mac.press(TITLE, 'Input & transfers')
    mac.wait_text(TITLE, 'Pointer: Ready to drag')
    mac.wait_text(TITLE, 'Panel width: 180 logical pixels')
    try:
        mouse.send(5, held)
        mouse.send(1, held)
        mac.wait_text(TITLE, 'Pointer: Started')
    finally:
        mouse.send(2, held)
    mac.wait_text(TITLE, 'Pointer: Released')
    print('GALLERY_INPUT_OK: captured OS pointer outside bounds, cancellation/disabled '
          'input, keyboard alternatives, text/custom transfers and rejection, themes/sizes, teardown', flush=True)


def verify_input_transfers(output):
    sources = defaultdict(list)
    targets = defaultdict(list)
    for line in output.splitlines():
        match = re.search(r'GALLERY_TRANSFER_(SOURCE|TARGET) gesture=(\d+) phase=(\w+)', line)
        if match:
            (sources if match[1] == 'SOURCE' else targets)[int(match[2])].append(match[3])
    ids = sorted(sources)
    assert len(ids) == 4, sources
    assert [sources[identity] for identity in ids] == [
        ['started', 'delivered'], ['started', 'cancelled'],
        ['started', 'delivered'], ['started', 'unconfirmed']], sources
    dropped = [identity for identity, phases in targets.items() for phase in phases if phase == 'dropped']
    assert sorted(dropped) == [ids[0], ids[2]], targets
    assert all(identity in sources for identity in targets), (sources, targets)
    assert all('entered' in targets[identity] for identity in dropped), targets
    print('GALLERY_TRANSFER_IDENTITIES_OK: four unique gestures, two matching native drops; '
          'cancelled/rejected/disabled attempts do not deliver', flush=True)


def exercise_extensions(mac, images):
    mac.press(TITLE, 'Native extensions')
    mac.wait_text(TITLE, 'Native component mounted')
    mac.wait_text(TITLE, 'Observed counter: 7')
    mac.press(TITLE, 'Increment counter, current value 7')
    mac.wait_text(TITLE, 'Observed counter: 8')
    window = mac.window(TITLE)
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    node = mac.wait_find(TITLE, 'Increment counter, current value 8', 'AXButton')
    try:
        mac.set(node, 'AXFocused', mac.true)
        expect_focus(mac, 'Increment counter, current value 8')
        mac.key(49)  # Space must activate exactly once.
    finally:
        mac.release(node)
    mac.wait_text(TITLE, 'Observed counter: 9')
    mac.press(TITLE, 'Use step 5')
    mac.wait_text(TITLE, 'Counter step: 5')
    node = mac.wait_find(TITLE, 'Increment counter, current value 9', 'AXButton')
    try:
        mac.set(node, 'AXFocused', mac.true)
        expect_focus(mac, 'Increment counter, current value 9')
        mac.key(36)
    finally:
        mac.release(node)
    mac.wait_text(TITLE, 'Observed counter: 14')
    mac.press(TITLE, 'Set property to 12')
    mac.release(mac.wait_find(TITLE, 'Increment counter, current value 12', 'AXButton'))
    mac.wait_text(TITLE, 'Observed counter: 12')
    mac.press(TITLE, 'Disable native input')
    expect_enabled(mac, 'Increment counter, current value 12', False)
    disabled = mac.wait_find(TITLE, 'Increment counter, current value 12', 'AXButton')
    try:
        try:
            mac.perform(disabled, 'AXPress')
        except RuntimeError:
            pass
        time.sleep(.15)
        mac.wait_text(TITLE, 'Observed counter: 12')
    finally:
        mac.release(disabled)
    mac.press(TITLE, 'Send command to 42')
    mac.wait_text(TITLE, 'Native command 1 completed')
    mac.wait_text(TITLE, 'Observed counter: 42')
    expect_enabled(mac, 'Increment counter, current value 42', False)
    mac.press(TITLE, 'Enable native input')
    expect_enabled(mac, 'Increment counter, current value 42', True)
    mac.press(TITLE, 'Set property to 12')
    retained = mac.wait_find(TITLE, 'Increment counter, current value 12', 'AXButton')
    try:
        mac.press(TITLE, 'Hide native counter')
        mac.release(mac.wait_find(TITLE, 'Show native counter', 'AXButton'))
        absent(mac, 'Increment counter, current value 12', 'AXButton')
        try:
            mac.perform(retained, 'AXPress')
        except RuntimeError:
            pass
        time.sleep(.15)
        mac.wait_text(TITLE, 'Observed counter: 12')
    finally:
        mac.release(retained)
    mac.press(TITLE, 'Send command to 42')
    mac.wait_text(TITLE, 'Native command 2 completed')
    mac.press(TITLE, 'Show native counter')
    mac.release(mac.wait_find(TITLE, 'Increment counter, current value 42', 'AXButton'))
    cycle_preview_appearance(mac, 'Observed counter: 42')
    if images:
        screenshot(mac, images / 'gallery-extensions.png', title=TITLE)
    retained = mac.wait_find(TITLE, 'Increment counter, current value 42', 'AXButton')
    try:
        mac.press(TITLE, 'Reset native instance')
        mac.wait_text(TITLE, 'Instance generation: 2')
        mac.wait_text(TITLE, 'Native component mounted')
        mac.wait_text(TITLE, 'Observed counter: 7')
        # The logical accessible button survives a native generation reset.
        # A retained AX reference resolves its current label/action, not an old
        # native closure. Verify activation uses the replacement's value of 7.
        labels = (mac.text(retained, 'AXTitle'), mac.text(retained, 'AXDescription'))
        print('EXTENSION_RESET_AX_LABELS', labels, flush=True)
        assert 'Increment counter, current value 7' in labels, labels
        mac.perform(retained, 'AXPress')
        mac.wait_text(TITLE, 'Observed counter: 12')
    finally:
        mac.release(retained)
    for generation in range(3, 6):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Increment counter, current value 12', 'AXButton')
        mac.press(TITLE, 'Native extensions')
        mac.wait_text(TITLE, f'Instance generation: {generation}')
        mac.wait_text(TITLE, 'Native component mounted')
        mac.wait_text(TITLE, 'Counter step: 5')
        mac.release(mac.wait_find(TITLE, 'Increment counter, current value 12', 'AXButton'))
    print('GALLERY_EXTENSIONS_OK: native AX/OS keyboard, property updates, sequenced '
          'commands while disabled/hidden, hidden AX fencing, current reset actions, themes/sizes and remount', flush=True)


def verify_extension_lifetimes(output):
    lifetimes = defaultdict(Counter)
    commands = []
    for line in output.splitlines():
        match = re.search(r'COUNTER_LIFETIME id=(\d+) phase=(\w+)', line)
        if match:
            lifetimes[int(match[1])][match[2]] += 1
        match = re.search(r'COUNTER_COMMAND id=(\d+) value=(\d+)', line)
        if match:
            commands.append((int(match[1]), int(match[2])))
    expected = Counter(mount=1, unmount=1, component_drop=1, value_drop=1)
    assert len(lifetimes) == 5 and all(value == expected for value in lifetimes.values()), lifetimes
    first = min(lifetimes)
    assert commands == [(first, 42), (first, 42)], commands
    print('GALLERY_EXTENSION_LIFETIMES_OK: five exact mount/unmount/component/value lifetimes; '
          'two commands, no replay during render/theme/resize/remount', flush=True)


def exercise_responsive(mac, images):
    mac.press(TITLE, 'Responsive layouts')
    mac.wait_text(TITLE, 'Painted layout: Compact · 400 × 300 · observation 1')
    expect_field(mac, TITLE, 'Compact layout draft', 'Compact ideas stay here.')
    absent(mac, 'Wide layout draft', 'AXTextField')
    absent(mac, 'Short layout draft', 'AXTextField')
    type_a(mac, 'Compact layout draft')
    compact = mac.wait_find(TITLE, 'Compact layout draft', 'AXTextField')
    try:
        before = element_rect(mac, compact)
        mac.press(TITLE, 'Compact saves: 0')
        mac.wait_text(TITLE, 'Compact saves: 1')
        mac.press(TITLE, 'Width 479')
        mac.wait_text(TITLE, 'Offered size: 479 × 300 logical pixels')
        # Selection observations carry the size at the last branch change.
        # A same-branch resize must neither emit again nor replace the editor.
        time.sleep(.15)
        mac.wait_text(TITLE, 'Painted layout: Compact · 400 × 300 · observation 1')
        after = element_rect(mac, compact)
        assert abs(after[2] - before[2] - 79) < 1, (before, after)
        expect_field(mac, TITLE, 'Compact layout draft', 'a')
        mac.press(TITLE, 'Width 480')
        mac.wait_text(TITLE, 'Painted layout: Wide · 480 × 300 · observation 2')
        absent(mac, 'Compact layout draft', 'AXTextField')
        # A retained accessibility reference must not activate the hidden editor.
        try:
            mac.set(compact, 'AXFocused', mac.true)
        except RuntimeError:
            pass
        time.sleep(.1)
        active = mac.attr(mac.app, 'AXFocusedUIElement')
        if active:
            try:
                assert 'Compact layout draft' not in (mac.text(active, 'AXTitle'),
                                                     mac.text(active, 'AXDescription'))
            finally:
                mac.release(active)
    finally:
        mac.release(compact)
    type_a(mac, 'Wide layout draft')
    mac.press(TITLE, 'Wide saves: 0')
    mac.wait_text(TITLE, 'Wide saves: 1')
    mac.press(TITLE, 'Height 200')
    mac.wait_text(TITLE, 'Painted layout: Short · 480 × 200 · observation 3')
    absent(mac, 'Wide layout draft', 'AXTextField')
    type_a(mac, 'Short layout draft')
    mac.press(TITLE, 'Short saves: 0')
    mac.wait_text(TITLE, 'Short saves: 1')
    mac.press(TITLE, 'Width 600')
    mac.wait_text(TITLE, 'Offered size: 600 × 200 logical pixels')
    time.sleep(.15)
    mac.wait_text(TITLE, 'Painted layout: Short · 480 × 200 · observation 3')
    expect_field(mac, TITLE, 'Short layout draft', 'a')
    mac.press(TITLE, 'Height 230')
    mac.wait_text(TITLE, 'Painted layout: Wide · 600 × 230 · observation 4')
    expect_field(mac, TITLE, 'Wide layout draft', 'a')
    mac.release(mac.wait_find(TITLE, 'Wide saves: 1', 'AXButton'))
    if images:
        screenshot(mac, images / 'gallery-responsive-wide.png', title=TITLE)
    cycle_preview_appearance(mac, 'Painted layout: Wide · 600 × 230 · observation 4')
    expect_field(mac, TITLE, 'Wide layout draft', 'a')
    mac.press(TITLE, 'Width 400')
    mac.wait_text(TITLE, 'Painted layout: Compact · 400 × 230 · observation 5')
    expect_field(mac, TITLE, 'Compact layout draft', 'a')
    mac.release(mac.wait_find(TITLE, 'Compact saves: 1', 'AXButton'))
    if images:
        screenshot(mac, images / 'gallery-responsive.png', title=TITLE)
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Compact layout draft', 'AXTextField')
        mac.press(TITLE, 'Responsive layouts')
        expect_field(mac, TITLE, 'Compact layout draft', 'Compact ideas stay here.')
        mac.release(mac.wait_find(TITLE, 'Compact saves: 1', 'AXButton'))
    mac.press(TITLE, 'Width 600')
    expect_field(mac, TITLE, 'Wide layout draft', 'Wide ideas stay here.')
    mac.press(TITLE, 'Height 200')
    expect_field(mac, TITLE, 'Short layout draft', 'Short ideas stay here.')
    mac.release(mac.wait_find(TITLE, 'Short saves: 1', 'AXButton'))
    print('GALLERY_RESPONSIVE_OK: native half-open boundaries and first-match priority, '
          'silent same-branch resize, retained drafts/counts, hidden AX/focus fencing, '
          'theme/size and fresh native editors on revisit', flush=True)


def raise_gallery(mac):
    window = mac.window(TITLE)
    assert window, 'Gallery window must exist before foreground rendering checks'
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)


def focus_gallery_control(mac, label, role):
    # Foregrounding and the GPUI activation observation are asynchronous. This
    # establishes focus before input; retention assertions still use expect_focus
    # without requesting focus again.
    raise_gallery(mac)
    get = mac.cf.CFBooleanGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
    deadline, requests = time.monotonic() + 10, 0
    while time.monotonic() < deadline:
        node = mac.wait_find(TITLE, label, role)
        try:
            mac.set(node, 'AXFocused', mac.true)
            requests += 1
            time.sleep(.04)
            value = mac.attr(node, 'AXFocused')
            try:
                if value and get(value):
                    print('GALLERY_FOCUS_READY', label, 'requests=', requests, flush=True)
                    return
            finally:
                if value:
                    mac.release(value)
        finally:
            mac.release(node)
    raise RuntimeError(f'Initial focus request did not settle for {label}')


def motion_width(mac, node):
    get = mac.ax.AXValueGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
    value = mac.attr(node, 'AXSize')
    size = (C.c_double * 2)()
    try:
        assert value and get(value, 2, C.byref(size)), 'animation AXSize'
        return size[0]
    finally:
        if value:
            mac.release(value)


def motion_samples(mac, label, seconds, after_press=None):
    node = mac.wait_find(TITLE, label, 'AXGroup')
    try:
        if after_press:
            mac.press(TITLE, after_press)
        samples = []
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            samples.append(motion_width(mac, node))
            time.sleep(.015)
        return samples
    finally:
        mac.release(node)


def exercise_motion(mac, images, second_title=SECOND):
    mac.press(TITLE, 'Motion & rhythm')
    raise_gallery(mac)
    mac.press(TITLE, 'Use full motion')
    mac.wait_text(TITLE, 'Motion preference: Full')
    opening = motion_samples(mac, 'Resize sample', 1.3, 'Expand preview')
    assert abs(opening[-1] - 310) < 1, opening
    assert any(98 < width < 308 for width in opening), opening
    # Resolve native elements before starting the interruption interval. A full
    # AX tree traversal between the two presses can outlast the animation.
    resize = mac.wait_find(TITLE, 'Resize sample', 'AXGroup')
    toggle = mac.wait_find(TITLE, 'Contract preview', 'AXButton')
    def sample_resize(seconds):
        samples = []
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            samples.append(motion_width(mac, resize))
            time.sleep(.015)
        return samples
    try:
        mac.perform(toggle, 'AXPress')
        early = sample_resize(.25)
        mac.perform(toggle, 'AXPress')
        reopening = sample_resize(1.3)
    finally:
        mac.release(toggle)
        mac.release(resize)
    print('GALLERY_MOTION_INTERRUPTION', 'before=', early[-1],
          'after_min=', min(reopening), 'end=', reopening[-1], flush=True)
    assert 98 < early[-1] < 308, early
    assert abs(reopening[-1] - 310) < 1, reopening
    assert min(reopening) > 98, reopening  # No jump to the declared narrow endpoint.
    started = motion_samples(mac, 'Sequence sample', .2, 'Replay sequence')
    paused = motion_samples(mac, 'Sequence sample', .4, 'Pause sequence')
    # The first sample can precede application of the asynchronous pause intent.
    assert max(started) - min(started) > 2, started
    assert max(paused[-12:]) - min(paused[-12:]) < 1, paused
    assert abs(paused[-1] - 120) > 2, paused  # A running stage, not the final endpoint.
    motion_samples(mac, 'Sequence sample', .12, 'Resume sequence')
    mac.press(TITLE, 'Cancel sequence')
    mac.wait_text(TITLE, 'Cancelled: Requested')
    mac.press(TITLE, 'Replay sequence')
    mac.wait_text(TITLE, 'Finished')
    assert abs(motion_samples(mac, 'Sequence sample', .1)[-1] - 120) < 1
    reverse = motion_samples(mac, 'Sequence sample', 3.5, 'Reverse sequence')
    assert abs(reverse[-1] - 64) < 1, reverse
    mac.press(TITLE, 'Use reduced motion')
    mac.wait_text(TITLE, 'Motion preference: Reduced')
    reduced = motion_samples(mac, 'Resize sample', .4, 'Contract preview')
    assert abs(reduced[-1] - 96) < 1, reduced
    assert all(min(abs(width-96), abs(width-310)) < 1 for width in reduced), reduced
    raise_gallery(mac)
    mac.press(TITLE, 'Replay sequence')
    mac.wait_text(TITLE, 'Stage 1 reduced · Stage 2 reduced · Stage 3 reduced · Finished')
    mac.press(TITLE, 'New window')
    mac.press(second_title, 'Motion & rhythm')
    mac.wait_text(second_title, 'Motion preference: Reduced')
    mac.press(second_title, 'Use full motion')
    mac.wait_text(TITLE, 'Motion preference: Full')
    mac.close(second_title)
    raise_gallery(mac)
    mac.press(TITLE, 'Start shared motion')
    first = motion_samples(mac, 'Shared member 1', .35)
    assert max(first) - min(first) > 3, first
    mac.press(TITLE, 'Join a second member')
    one = mac.wait_find(TITLE, 'Shared member 1', 'AXGroup')
    two = mac.wait_find(TITLE, 'Shared member 2', 'AXGroup')
    try:
        pairs = [(motion_width(mac, one), motion_width(mac, two)) for _ in range(8)]
        assert all(abs(a-b) < 12 for a, b in pairs), pairs
    finally:
        mac.release(one)
        mac.release(two)
    mac.press(TITLE, 'Use reduced motion')
    still = motion_samples(mac, 'Shared member 1', .4)
    assert all(abs(width-56) < 1 for width in still[-12:]), still
    if images:
        screenshot(mac, images / 'gallery-motion.png', title=TITLE)
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A little context goes a long way')
    absent(mac, 'Shared member 1', 'AXGroup')
    absent(mac, 'Sequence sample', 'AXGroup')
    mac.press(TITLE, 'Motion & rhythm')
    mac.wait_text(TITLE, 'Shared motion is stopped.')
    mac.wait_text(TITLE, 'Ready to play')
    absent(mac, 'Shared member 2', 'AXGroup')
    button = mac.wait_find(TITLE, 'Use system motion', 'AXButton')
    try:
        mac.set(button, 'AXFocused', mac.true)
        expect_focus(mac, 'Use system motion')
        mac.key(49)  # Space through the actual OS keyboard route.
    finally:
        mac.release(button)
    mac.wait_text(TITLE, 'Motion preference: System')
    print('GALLERY_MOTION_OK: native intermediate geometry, interruption, paused spring '
          'sequence, cancellation/reverse, reduced endpoints, shared phase and departure', flush=True)


def exercise_attachments(mac, images):
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'From queued to ready, with room for the details.')
    opened = saved = cases = 0
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    original = mac.wait_find(TITLE, 'Open Aurora attachment', 'AXButton')
    save_original = mac.wait_find(TITLE, 'Save attachment', 'AXButton')
    mouse = GalleryMouse(mac)

    def counter():
        mac.wait_text(TITLE, f'Attachment opened: {opened} · saved: {saved}')

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.12)

    def click(point):
        mouse.check_owner(point)
        mouse.send(5, point)
        mouse.send(1, point)
        mouse.send(2, point)

    def rect(label, role='AXButton'):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def identity():
        for label, previous in [('Open Aurora attachment', original), ('Save attachment', save_original)]:
            node = mac.wait_find(TITLE, label, 'AXButton')
            try:
                assert equal(previous, node), f'{label} replaced during restyle'
            finally:
                mac.release(node)

    theme_node = mac.find(TITLE, 'Dark', 'AXButton')
    theme = 'Dark' if theme_node else 'Light'
    if theme_node:
        mac.release(theme_node)
    try:
        for appearance in (theme, 'Light' if theme == 'Dark' else 'Dark'):
            for axis in ('horizontal', 'vertical'):
                for status, next_status in [('Complete','Pending'), ('Pending','Uploading'),
                                            ('Uploading','Processing'), ('Processing','Failed'), ('Failed','Complete')]:
                    reveal_gallery_control(mac, 'Open Aurora attachment', 'AXButton')
                    identity()
                    cx, cy, cw, ch = rect('Attachment preview card', 'AXGroup')
                    tx, ty, tw, th = rect('Open Aurora attachment')
                    assert abs(cw-(440 if axis=='horizontal' else 180)) < 1, (axis,cw)
                    assert abs(cx-tx) <= 2 and abs(cy-ty) <= 2, ('trigger origin',(cx,cy),(tx,ty))
                    assert abs(cw-tw) <= 4 and abs(ch-th) <= 4, ('trigger coverage',(cw,ch),(tw,th))
                    focus_gallery_control(mac, 'Open Aurora attachment', 'AXButton')
                    mac.key(36)
                    opened += 1
                    counter()
                    sx, sy, sw, sh = rect('Save attachment')
                    click((sx+sw/2, sy+sh/2))
                    saved += 1
                    counter()
                    dx, dy, dw, dh = rect('Unavailable attachment action')
                    assert dx-sx-sw > 6, ('action gap', (sx,sw,dx))
                    click(((sx+sw+dx)/2, sy+sh/2))
                    click((dx+dw/2, dy+dh/2))
                    time.sleep(.12)
                    counter()
                    cases += 1
                    mac.press(TITLE, 'Attachment status: '+status)
                    mac.release(mac.wait_find(TITLE, 'Attachment status: '+next_status, 'AXButton'))
                mac.press(TITLE, 'Attachment layout: '+axis)
                mac.release(mac.wait_find(TITLE, 'Attachment layout: '+('vertical' if axis=='horizontal' else 'horizontal'), 'AXButton'))
            mac.press(TITLE, appearance)
            mac.release(mac.wait_find(TITLE, 'Light' if appearance=='Dark' else 'Dark', 'AXButton'))
        toggle('Attachment image')
        mac.wait_text(TITLE, 'Attachment image: ready')
        toggle('Simulate attachment decode failure')
        mac.wait_text(TITLE, 'Attachment image: failed')
        identity()
        toggle('Simulate attachment decode failure')
        mac.wait_text(TITLE, 'Attachment image: ready')
        for size, extent, next_size in [('M',40,'L'), ('L',48,'56'), ('56',56,'XS'), ('XS',28,'S'), ('S',32,'M')]:
            reveal_gallery_control(mac, 'Open Aurora attachment', 'AXButton')
            _, _, w, h = rect('Attachment landscape', 'AXImage')
            assert abs(w-extent) < 1 and abs(h-extent) < 1, (size,w,h,extent)
            identity()
            mac.press(TITLE, 'Attachment size: '+size)
            mac.release(mac.wait_find(TITLE, 'Attachment size: '+next_size, 'AXButton'))
        mac.press(TITLE, 'Attachment layout: horizontal')
        mac.release(mac.wait_find(TITLE, 'Attachment layout: vertical', 'AXButton'))
        _, _, w, h = rect('Attachment landscape', 'AXImage')
        assert abs(w-h) < 1 and abs(w-162) < 1, ('vertical square',(w,h))
        toggle('Refine attachment style')
        identity()
        toggle('Refine attachment style')
        toggle('Disable attachment')
        reveal_gallery_control(mac, 'Open Aurora attachment', 'AXButton')
        x,y,w,h = rect('Open Aurora attachment')
        click((x+16,y+h-16))
        time.sleep(.12)
        counter()
        toggle('Disable attachment')
        reveal_gallery_control(mac, 'Open Aurora attachment', 'AXButton')
        x,y,w,h = rect('Open Aurora attachment')
        click((x+16,y+h-16))
        opened += 1
        counter()
        mac.press(TITLE, 'Open Aurora attachment')
        opened += 1
        counter()
        focus_gallery_control(mac, 'Open Aurora attachment', 'AXButton')
        mac.key(49)
        opened += 1
        counter()
        toggle('Attachment content')
        absent(mac, 'Aurora · 京都.png', 'AXStaticText')
        identity()
        toggle('Attachment content')
        mac.wait_text(TITLE, 'Aurora · 京都.png')
        toggle('Attachment media')
        absent(mac, 'Attachment landscape', 'AXImage')
        identity()
        toggle('Attachment media')
        mac.release(mac.wait_find(TITLE, 'Attachment landscape', 'AXImage'))
        if images:
            reveal_gallery_control(mac, 'Open Aurora attachment', 'AXButton')
            screenshot(mac, images / 'gallery-attachment.png', title=TITLE)
        toggle('Attachment actions')
        absent(mac, 'Save attachment', 'AXButton')
        toggle('Attachment actions')
        new_save = mac.wait_find(TITLE, 'Save attachment', 'AXButton')
        try:
            assert not equal(save_original, new_save), 'Removed action owner survived'
        finally:
            mac.release(new_save)
        mac.press(TITLE, 'Runtime & windows')
        mac.press(TITLE, 'Refresh resource counts')
        wait_for_resource_cleanup(mac)
        mac.wait_text(TITLE, 'Registered source bytes: 0')
        absent(mac, 'Open Aurora attachment', 'AXButton')
        absent(mac, 'Attachment landscape', 'AXImage')
        mac.press(TITLE, 'Presentation')
        restored = mac.wait_find(TITLE, 'Open Aurora attachment', 'AXButton')
        try:
            assert not equal(original, restored), 'Page departure retained trigger owner'
        finally:
            mac.release(restored)
        counter()
        print(f'GALLERY_ATTACHMENT_OK: {cases} theme/axis/status cases, 21 OS keyboard activations plus card pointer/AX activation, {saved} pointer actions, shielded gaps/disabled controls, decode failure/recovery, five decoded-image sizes and square layout, styles/slots/identity/disabled recovery/page teardown', flush=True)
    finally:
        mac.release(original)
        mac.release(save_original)


def exercise_attachment_paint(mac, images):
    """Public attachment pixels, native motion preference and real nested scrolling."""
    mac.press(TITLE, 'Motion & rhythm')
    mac.press(TITLE, 'Use full motion')
    mac.wait_text(TITLE, 'Motion preference: Full')
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'From queued to ready, with room for the details.')
    status = 'Complete'
    statuses = ('Complete', 'Pending', 'Uploading', 'Processing', 'Failed')
    mouse = GalleryMouse(mac)
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-attachment-paint-')
    directory = images or Path(temporary.name)
    serial = 0
    title = mac.wait_find(TITLE, 'Aurora · 京都.png', 'AXStaticText')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))
        time.sleep(.12)

    def set_status(value):
        nonlocal status
        while status != value:
            mac.press(TITLE, 'Attachment status: '+status)
            status = statuses[(statuses.index(status)+1) % len(statuses)]
            mac.release(mac.wait_find(TITLE, 'Attachment status: '+status, 'AXButton'))
        node = mac.wait_find(TITLE, 'Aurora · 京都.png', 'AXStaticText')
        try:
            assert equal(title,node), 'Status replaced title identity'
        finally:
            mac.release(node)
        time.sleep(.08)

    def rect(label, role='AXButton'):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def reveal():
        reveal_gallery_control(mac, 'Open Aurora attachment', 'AXButton')
        # Keep hover/pressed/focus decoration out of the color assertions.
        wx,wy,ww,wh = rect('Attachment preview group', 'AXGroup')
        mouse.send(5, (wx+ww+12,wy+5))
        time.sleep(.1)

    def capture():
        nonlocal serial
        window = mac.window(TITLE)
        try:
            wx,wy,ww,wh = element_rect(mac, window)
        finally:
            mac.release(window)
        path = directory / f'attachment-paint-{serial:03d}.png'
        serial += 1
        screenshot(mac, path, title=TITLE)
        pixels = read_png(mac, path)
        def rgb(x,y):
            assert wx <= x < wx+ww and wy <= y < wy+wh, ('sample outside window',x,y)
            return pixels.rgb((x-wx)*pixels.width/ww, (y-wy)*pixels.height/wh)
        return rgb

    def title_sample():
        # Reuse the retained AX reference. Walking the whole gallery for each
        # sample can alias the sweep period, especially in installed consumers.
        x,y,w,h = element_rect(mac,title)
        rgb = capture()
        return (x,y,w,h), tuple(rgb(x+(i+.5)*w/180,y+(j+.5)*h/28)
                               for j in range(28) for i in range(180))

    def difference(a,b):
        assert all(abs(x-y)<.2 for x,y in zip(a[0],b[0])), ('title geometry changed',a[0],b[0])
        return sum(max(abs(x-y) for x,y in zip(p,q))>5 for p,q in zip(a[1],b[1]))

    def still():
        time.sleep(.15)
        a=title_sample()
        time.sleep(.3)
        b=title_sample()
        assert difference(a,b)==0, 'Static/reduced title changed pixels'
        return b

    def moving(baseline):
        most=0
        started=time.monotonic()
        deadline=started+5
        timings=[]
        while time.monotonic()<deadline:
            time.sleep(.12)
            most=max(most,difference(baseline,title_sample()))
            timings.append(round(time.monotonic()-started,3))
            if most>=6:
                print('ATTACHMENT_GLYPH_ANIMATION',most,'samples=',timings,flush=True)
                return
        raise AssertionError(f'No attachment glyph motion: {most} at {timings}')

    def near(actual,expected,tolerance=5):
        assert max(abs(a-b) for a,b in zip(actual,expected))<=tolerance,(actual,expected)

    theme_node=mac.find(TITLE,'Dark','AXButton')
    initial='Dark' if theme_node else 'Light'
    if theme_node:
        mac.release(theme_node)
    try:
        mac.press(TITLE,'Attachment layout: horizontal')
        mac.release(mac.wait_find(TITLE,'Attachment layout: vertical','AXButton'))
        toggle('Attachment image')
        mac.wait_text(TITLE,'Attachment image: ready')
        for theme in (initial,'Light' if initial=='Dark' else 'Dark'):
            reveal()
            card=rect('Attachment preview card','AXGroup')
            media=rect('Attachment landscape','AXImage')
            overlay=rect('PNG','AXStaticText')
            tx,ty,tw,th=element_rect(mac,title)
            x,y,w,h=media
            ox,oy,ow,oh=overlay
            rgb=capture()
            baseline_image=rgb(x+w*.22,y+h*.3)
            # Pixel inside card padding supplies its actual composited surface.
            surface=rgb(card[0]+4,card[1]+card[3]*.6)
            glyph_points=[(ox+(i+.5)*ow/100,oy+(j+.5)*oh/28)
                          for j in range(28) for i in range(100)]
            palette=(234,240,247) if theme=='Dark' else (27,41,57)
            opaque=[(point,rgb(*point)) for point in glyph_points
                    if max(abs(a-b) for a,b in zip(rgb(*point),palette))<=5]
            assert len(opaque)>=8, ('no opaque overlay glyph pixels',theme,len(opaque))
            baseline=still()
            set_status('Uploading')
            moving(baseline)
            rgb=capture()
            raised=(39,46,59) if theme=='Dark' else (240,242,246)
            expected=tuple(round(.6*a+.4*b) for a,b in zip(baseline_image,raised))
            near(rgb(x+w*.22,y+h*.3),expected,8)
            for point,pixel in opaque:
                near(rgb(*point),pixel,5)
            set_status('Processing')
            moving(baseline)
            set_status('Failed')
            assert difference(baseline,still())==0,'Failure did not restore ordinary title'
            rgb=capture()
            near(rgb(x+w*.22,y+h*.3),expected,8)
            danger=(255,160,175) if theme=='Dark' else (183,52,75)
            description_color=tuple(round(.8*a+.2*b) for a,b in zip(danger,surface))
            dx,dy,dw,dh=rect('PNG image · 2.4 MB','AXStaticText')
            matches=sum(max(abs(a-b) for a,b in zip(
                rgb(dx+(i+.5)*dw/180,dy+(j+.5)*dh/28),description_color))<=8
                for j in range(28) for i in range(180))
            assert matches>=8,('failed description did not paint danger alpha',theme,matches,description_color)
            toggle('Attachment image')
            rgb=capture()
            danger=(255,160,175) if theme=='Dark' else (183,52,75)
            failed_bg=tuple(round(.1*a+.9*b) for a,b in zip(danger,surface))
            near(rgb(x+w*.22,y+h*.3),failed_bg,5)
            toggle('Attachment image')
            mac.wait_text(TITLE,'Attachment image: ready')
            set_status('Complete')
            assert difference(baseline,still())==0,'Completion did not restore title'
            # Compare the painted straight top border, away from corners/actions.
            def top_line():
                rgb=capture()
                cx,cy,cw,ch=rect('Attachment preview card','AXGroup')
                return [rgb(cx+24+i*(cw-48)/160,cy+.5) for i in range(160)]
            solid=top_line()
            set_status('Pending')
            dashed=top_line()
            changed=sum(max(abs(a-b) for a,b in zip(p,q))>8 for p,q in zip(solid,dashed))
            assert 10<changed<150,('pending border has no visible dash/gap alternation',changed)
            set_status('Complete')
            mac.press(TITLE,theme)
            mac.release(mac.wait_find(TITLE,'Light' if theme=='Dark' else 'Dark','AXButton'))
        set_status('Uploading')
        mac.press(TITLE,'Motion & rhythm')
        mac.press(TITLE,'Use reduced motion')
        mac.wait_text(TITLE,'Motion preference: Reduced')
        mac.press(TITLE,'Presentation')
        replacement=mac.wait_find(TITLE,'Aurora · 京都.png','AXStaticText')
        mac.release(title)
        title=replacement
        reveal()
        reduced=still()
        set_status('Complete')
        assert difference(reduced,still())==0,'Reduced title differs from ordinary text'
        set_status('Processing')
        assert difference(reduced,still())==0,'Processing ignores reduced motion'
        mac.press(TITLE,'Motion & rhythm')
        mac.press(TITLE,'Use full motion')
        mac.wait_text(TITLE,'Motion preference: Full')
        mac.press(TITLE,'Presentation')
        replacement=mac.wait_find(TITLE,'Aurora · 京都.png','AXStaticText')
        mac.release(title)
        title=replacement
        reveal()
        set_status('Complete')
        baseline=still()
        set_status('Uploading')
        moving(baseline)
        set_status('Complete')
        toggle('More attachments')
        toggle('Constrain attachment group')
        reveal_gallery_control(mac,'Attachment preview group','AXGroup')
        group=rect('Attachment preview group','AXGroup')
        gx,gy,gw,gh=group
        assert abs(gw-280)<1,group
        before=rect('Open Notes attachment')
        assert before[0]>=gx+gw,'Third attachment should start clipped'
        first=mac.wait_find(TITLE,'Open Aurora attachment','AXButton')
        last=mac.wait_find(TITLE,'Open Notes attachment','AXButton')
        # The sixth-argument API avoids the arm64 variadic calling convention.
        # Declaring wheel2 as fixed on the variadic API silently emits zero X.
        create=mac.cg.CGEventCreateScrollWheelEvent2
        create.restype,create.argtypes=C.c_void_p,[C.c_void_p,C.c_uint,C.c_uint,C.c_int,C.c_int,C.c_int]
        delta=mac.cg.CGEventGetIntegerValueField
        delta.restype,delta.argtypes=C.c_longlong,[C.c_void_p,C.c_int]
        locate=mac.cg.CGEventSetLocation
        locate.restype,locate.argtypes=None,[C.c_void_p,GalleryMouse.Point]
        def scroll(dx,dy=0,point=None):
            point=point or (gx+gw*.6,gy+gh*.5)
            mouse.check_owner(point)
            mouse.send(5,point)
            event=create(None,0,2,dy,dx,0)
            assert event
            try:
                assert delta(event,97)==dx and delta(event,96)==dy, 'Wrong native scroll delta'
                locate(event,GalleryMouse.Point(*point))
                mouse.post(0,event)
            finally:
                mac.release(event)
            time.sleep(.15)
        try:
            sx,sy,sw,sh=rect('Save attachment')
            dx,dy,dw,dh=rect('Unavailable attachment action')
            gap=((sx+sw+dx)/2,sy+sh/2)
            start_x=rect('Open Aurora attachment')[0]
            scroll(-100,point=gap)
            assert rect('Open Aurora attachment')[0]<start_x-10, 'Action shield swallowed horizontal wheel'
            for _ in range(8):
                scroll(-100)
                after=rect('Open Notes attachment')
                if after[0]+after[2]<=gx+gw+1:
                    break
            assert gx<=after[0] and after[0]+after[2]<=gx+gw+1,('horizontal reveal',group,after)
            current=mac.wait_find(TITLE,'Open Notes attachment','AXButton')
            assert equal(last,current),'Scroll replaced attachment action'
            mac.release(current)
            x,y,w,h=after
            mouse.check_owner((x+w/2,y+h/2))
            mouse.send(1,(x+w/2,y+h/2))
            mouse.send(2,(x+w/2,y+h/2))
            mac.wait_text(TITLE,'Attachment opened: 1 · saved: 0')
            for _ in range(8):
                scroll(100)
            current=mac.wait_find(TITLE,'Open Aurora attachment','AXButton')
            assert equal(first,current),'Return scroll replaced first attachment'
            mac.release(current)
            restored=rect('Open Aurora attachment')
            assert abs(restored[0]-gx)<=2,('scroll reset',group,restored)
            sx,sy,sw,sh=rect('Save attachment')
            dx,dy,dw,dh=rect('Unavailable attachment action')
            scroll(0,-60,point=((sx+sw+dx)/2,sy+sh/2))
            after_vertical=rect('Attachment preview group','AXGroup')
            assert after_vertical[1]<gy-10, ('vertical wheel did not reach parent',group,after_vertical)
            assert abs(after_vertical[0]-gx)<1, 'Vertical wheel changed horizontal origin'
            mac.wait_text(TITLE,'Attachment opened: 1 · saved: 0')
            toggle('Constrain attachment group')
            toggle('More attachments')
        finally:
            mac.release(first)
            mac.release(last)
        mac.press(TITLE,'Runtime & windows')
        mac.press(TITLE,'Refresh resource counts')
        wait_for_resource_cleanup(mac)
        mac.wait_text(TITLE,'Registered source bytes: 0')
        print('GALLERY_ATTACHMENT_PAINT_OK: light/dark native glyph motion and static completion/failure; image alpha and undimmed overlay; no-image failure tint; pending dashed paint; reduced-motion stop/full-motion recovery; real horizontal scroll, revealed action and retained owners; released images/sources',flush=True)
    finally:
        mac.release(title)
        temporary.cleanup()


def exercise_aspect_ratio(mac, images):
    mac.press(TITLE, 'Styling details')
    mac.wait_text(TITLE, 'Proportions that follow your layout')
    reveal_gallery_control(mac, 'Kept 0', 'AXButton')
    original = mac.wait_find(TITLE, 'Aspect preview surface', 'AXGroup')
    button = mac.wait_find(TITLE, 'Kept 0', 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    count = 0
    cases = 0

    def geometry(ratio, fixed=False):
        frame = mac.wait_find(TITLE, 'Aspect preview frame', 'AXGroup')
        node = mac.wait_find(TITLE, 'Aspect preview surface', 'AXGroup')
        control = mac.wait_find(TITLE, f'Kept {count}', 'AXButton')
        try:
            assert equal(original, node), 'Restyle replaced aspect surface'
            assert equal(button, control), 'Restyle replaced child control'
            x, y, w, h = element_rect(mac, node)
            fx, fy, fw, fh = element_rect(mac, frame)
            expected_width = fw - 26  # 12px padding and 1px border on each side.
            expected_height = 80 if fixed else expected_width / ratio
            assert abs(w-expected_width) < 1 and abs(h-expected_height) < 1, ((w,h),(expected_width,expected_height))
            assert abs(x-fx-13) < 1 and abs(y-fy-13) < 1, ('padding', (x,y),(fx,fy))
        finally:
            mac.release(frame)
            mac.release(node)
            mac.release(control)

    def activate_child():
        nonlocal count
        expect_focus(mac, f'Kept {count}', 'AXButton')
        mac.key(36)
        count += 1
        mac.wait_text(TITLE, f'Kept {count}')

    theme_node = mac.find(TITLE, 'Dark', 'AXButton')
    themes = ('Dark','Light') if theme_node else ('Light','Dark')
    if theme_node:
        mac.release(theme_node)
    try:
        focus_gallery_control(mac, 'Kept 0', 'AXButton')
        for theme in themes:
            for width in ('compact','wide'):
                for label, ratio, next_label in (('Square',1.,'Landscape'),('Landscape',2.,'Portrait'),('Portrait',.5,'Square')):
                    geometry(ratio)
                    activate_child()
                    geometry(ratio)
                    cases += 1
                    mac.press(TITLE, 'Ratio: '+label)
                    mac.release(mac.wait_find(TITLE, 'Ratio: '+next_label, 'AXButton'))
                mac.press(TITLE, 'Frame: '+width)
                mac.release(mac.wait_find(TITLE, 'Frame: '+('wide' if width=='compact' else 'compact'), 'AXButton'))
            mac.press(TITLE, theme)
            mac.release(mac.wait_find(TITLE, 'Light' if theme=='Dark' else 'Dark', 'AXButton'))
        mac.press(TITLE, 'Ratio: Square')
        mac.release(mac.wait_find(TITLE, 'Ratio: Landscape', 'AXButton'))
        mac.press(TITLE, 'Ratio: Landscape')
        mac.release(mac.wait_find(TITLE, 'Ratio: Portrait', 'AXButton'))
        mac.press(TITLE, 'Height: automatic')
        mac.release(mac.wait_find(TITLE, 'Height: fixed', 'AXButton'))
        geometry(.5, fixed=True)
        activate_child()
        mac.press(TITLE, 'Height: fixed')
        mac.release(mac.wait_find(TITLE, 'Height: automatic', 'AXButton'))
        geometry(.5)
        mac.press(TITLE, 'Ratio: Portrait')
        mac.release(mac.wait_find(TITLE, 'Ratio: Square', 'AXButton'))
        geometry(1.)
        if images:
            reveal_gallery_control(mac, f'Kept {count}', 'AXButton')
            screenshot(mac, images / 'gallery-aspect-ratio.png', title=TITLE)
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Aspect preview surface', 'AXGroup')
        mac.press(TITLE, 'Styling details')
        restored = mac.wait_find(TITLE, f'Kept {count}', 'AXButton')
        try:
            assert not equal(button, restored), 'Page departure retained the old control'
        finally:
            mac.release(restored)
        print(f'GALLERY_ASPECT_RATIO_OK: {cases} theme/ratio/width cases, explicit height/reset, stable native surface/child identity and keyboard focus, {count} Return actions, page teardown with retained model', flush=True)
    finally:
        mac.release(original)
        mac.release(button)


def exercise_borders(mac, images):
    """AX-driven public style updates; no foreground keyboard/focus claim."""
    mac.press(TITLE, 'Styling details')
    label = 'Border preview surface'
    original = mac.wait_find(TITLE, label, 'AXGroup')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    button = mac.find(TITLE, 'Dark', 'AXButton')
    current, alternate = ('Dark', 'Light') if button else ('Light', 'Dark')
    if button:
        mac.release(button)
    cases = 0
    try:
        for theme in (current, alternate):
            for pattern in ('dashed', 'solid'):
                for width in (1, 4):
                    for corners in ('rounded', 'square'):
                        mac.wait_text(TITLE, f'Border: {pattern} · {width} px · {corners}')
                        node = mac.wait_find(TITLE, label, 'AXGroup')
                        try:
                            assert equal(original, node), 'Restyle replaced border preview identity'
                            _, _, w, h = element_rect(mac, node)
                            assert abs(w - 340) < 1 and abs(h - 90) < 1, (w, h)
                        finally:
                            mac.release(node)
                        cases += 1
                        mac.press(TITLE, 'Change border corners')
                    mac.press(TITLE, 'Change border weight')
                mac.press(TITLE, 'Change border pattern')
            mac.press(TITLE, theme)
            mac.release(mac.wait_find(TITLE, alternate if theme == current else current, 'AXButton'))
        mac.press(TITLE, 'Change border pattern')
        mac.press(TITLE, 'Change border weight')
        mac.press(TITLE, 'Change border corners')
        selected = 'Border: solid · 4 px · square'
        mac.wait_text(TITLE, selected)
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, label, 'AXGroup')
        mac.press(TITLE, 'Styling details')
        mac.wait_text(TITLE, selected)
        # Restore defaults so repeated use within the complete gallery remains
        # independent of which focused section was exercised first.
        for control in ('pattern', 'weight', 'corners'):
            mac.press(TITLE, f'Change border {control}')
        mac.wait_text(TITLE, 'Border: dashed · 1 px · rounded')
    finally:
        mac.release(original)
    print(f'GALLERY_BORDERS_OK: {cases} theme/pattern/width/radius cases, native AX identity and geometry, retained page state, unmount/remount; GPU pattern evidence is native_border_style', flush=True)


def exercise_styles(mac, images):
    mac.press(TITLE, 'Styling details')
    mac.wait_text(TITLE, 'Text preview width: 250')
    mac.wait_text(TITLE, 'Cursor 1 of 22: Arrow')
    source = '/workspace/projects/native-studio/src/main.ml'
    def text_widths(expected):
        widths = []
        def visit(node):
            if mac.text(node, 'AXRole') == 'AXStaticText' and mac.text(node, 'AXTitle') == source:
                widths.append(element_rect(mac, node)[2])
            children = mac.children(node)
            try:
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        root = mac.window(TITLE)
        assert root
        try:
            visit(root)
        finally:
            mac.release(root)
        assert len(widths) == 3 and all(abs(width-expected) < 1 for width in widths), widths
    text_widths(250)
    mac.wait_text(TITLE, "Interpolation: sRGB")
    mac.press(TITLE, "Change gradient interpolation")
    mac.wait_text(TITLE, "Interpolation: Oklab")
    if images:
        screenshot(mac, images / 'gallery-styles-wide.png', title=TITLE)
    mac.press(TITLE, 'Narrow text previews')
    mac.wait_text(TITLE, 'Text preview width: 140')
    text_widths(140)
    labels = ['Text', 'Pointer', 'Crosshair', 'Move', 'Not allowed', 'Horizontal resize',
              'Vertical resize', 'Grab', 'Grabbing', 'Vertical text', 'Column resize',
              'Row resize', 'Northwest–southeast resize', 'Northeast–southwest resize',
              'Left resize', 'Right resize', 'Up resize', 'Down resize', 'Alias', 'Copy',
              'Context menu', 'Arrow']
    for i, label in enumerate(labels):
        mac.press(TITLE, 'Next cursor')
        mac.wait_text(TITLE, f'Cursor {(i+1)%22+1} of 22: {label}')
    focus_gallery_control(mac, 'Next cursor', 'AXButton')
    mac.key(49)
    mac.wait_text(TITLE, 'Cursor 2 of 22: Text')
    cycle_preview_appearance(mac, 'Cursor 2 of 22: Text')
    text_widths(140)
    mac.wait_text(TITLE, "Interpolation: Oklab")
    mac.press(TITLE, "Change gradient interpolation")
    mac.wait_text(TITLE, "Interpolation: sRGB")
    mac.press(TITLE, "Change gradient interpolation")
    mac.wait_text(TITLE, "Interpolation: Oklab")
    if images:
        screenshot(mac, images / 'gallery-styles-narrow.png', title=TITLE)
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Cursor preview surface', 'AXGroup')
        mac.press(TITLE, 'Styling details')
        mac.wait_text(TITLE, 'Cursor 2 of 22: Text')
        mac.wait_text(TITLE, 'Text preview width: 140')
        mac.wait_text(TITLE, 'Interpolation: Oklab')
    exercise_borders(mac, images)
    exercise_aspect_ratio(mac, images)
    print('GALLERY_STYLES_OK: all22 cursor configurations, keyboard/theme/size/visit retention, '
          'sRGB/Oklab gradient restyle, three bounded text samples with complete accessible source; physical cursor artwork is not asserted', flush=True)


def exercise_desktop(mac, images, *, second_title=SECOND, bundled=False):
    mac.press(TITLE, 'Desktop services')
    mac.wait_text(TITLE, 'GPUIO Component Studio')
    mac.wait_text(TITLE, 'Received link: gpuio-studio://preview/startup')
    mac.press(TITLE, 'Check desktop support')
    mac.wait_text(TITLE, 'Desktop support: links, registration, activation, reveal, open, document metadata')
    if not bundled:
        mac.wait_text(TITLE, 'Notification support: Unavailable')
        mac.wait_text(TITLE, 'Notification access: Unavailable')
        mac.press(TITLE, 'Post preview notification')
        mac.wait_text(TITLE, 'Notification post: Unavailable')
        expect_enabled(mac, 'Replace preview notification', False)
        expect_enabled(mac, 'Dismiss preview notification', False)
        mac.press(TITLE, 'Register Studio links with the OS')
        mac.wait_text(TITLE, 'Link registration: Unavailable')
    mac.press(TITLE, 'Activate application')
    mac.wait_text(TITLE, 'Application activation: request accepted')
    mac.press(TITLE, 'Clear represented file')
    mac.wait_text(TITLE, 'Document metadata observed')
    mac.wait_text(TITLE, 'Represented file: none')
    expect_enabled(mac, 'Open represented file', False)
    expect_enabled(mac, 'Reveal represented file', False)
    mac.press(TITLE, 'Mark document edited')
    mac.wait_text(TITLE, 'Document edited: true')
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Post preview notification', 'AXButton')
        mac.press(TITLE, 'Desktop services')
        mac.wait_text(TITLE, 'Document edited: true')
        mac.wait_text(TITLE, 'Received link: gpuio-studio://preview/startup')
    cycle_preview_appearance(mac, 'Document edited: true')
    if images:
        screenshot(mac, images / 'gallery-desktop.png', title=TITLE)
    mac.press(TITLE, 'New window')
    mac.wait_text(second_title, 'A little context goes a long way')
    mac.press(second_title, 'Desktop services')
    mac.wait_text(second_title, 'Document edited: false')
    mac.wait_text(second_title, 'Received link: gpuio-studio://preview/startup')
    if not bundled:
        mac.wait_text(second_title, 'Notification post: Unavailable')
    mac.press(second_title, 'Mark document edited')
    mac.wait_text(second_title, 'Document edited: true')
    mac.press(TITLE, 'Mark document saved')
    mac.wait_text(TITLE, 'Document edited: false')
    mac.wait_text(second_title, 'Document edited: true')
    mac.close(second_title)
    raise_gallery(mac)
    mac.press(TITLE, 'Choose represented file')
    mac.release(mac.wait_find(TITLE, 'Cancel', 'AXButton'))
    mac.press(TITLE, 'Cancel')
    mac.wait_text(TITLE, 'Document selection cancelled')
    mac.wait_text(TITLE, 'Represented file: none')
    print('GALLERY_DESKTOP_OK: incoming startup link, shared application services, '
          'per-window observed metadata, explicit OS results, themes/sizes and teardown', flush=True)


def exercise_runtime(mac, images):
    mac.press(TITLE, 'Runtime & windows')
    mac.press(TITLE, 'Refresh resource counts')
    mac.wait_text(TITLE, 'Windows: 1')
    mac.wait_text(TITLE, 'Documents: 0')
    mac.wait_text(TITLE, 'Registered source bytes: 0')
    mac.press(TITLE, 'Observe this window')
    mac.wait_text(TITLE, 'Observed content:')
    if images:
        screenshot(mac, images / 'gallery-runtime.png', title=TITLE)
    mac.press(TITLE, 'Choose a file')
    mac.release(mac.wait_find(TITLE, 'Cancel', 'AXButton'))
    mac.press(TITLE, 'Cancel')
    mac.wait_text(TITLE, 'File selection cancelled')



def exercise_settings_composition(mac):
    """Actual US-layout dead-key composition; does not change the input source."""
    carbon = C.CDLL('/System/Library/Frameworks/Carbon.framework/Carbon')
    current = carbon.TISCopyCurrentKeyboardInputSource
    current.restype, current.argtypes = C.c_void_p, []
    property_ = carbon.TISGetInputSourceProperty
    property_.restype, property_.argtypes = C.c_void_p, [C.c_void_p, C.c_void_p]
    source = current()
    if not source:
        raise RuntimeError('No active macOS keyboard input source')
    try:
        name = C.c_void_p.in_dll(carbon, 'kTISPropertyInputSourceID').value
        value = property_(source, name)
        buffer = C.create_string_buffer(1024)
        if not value or not mac.get_string(value, buffer, len(buffer), 0x08000100):
            raise RuntimeError('Cannot read the active keyboard input source')
        source_id = buffer.value.decode()
        if source_id != 'com.apple.keylayout.US':
            raise RuntimeError(f'Settings dead-key check requires the US input source; current={source_id}. No input source was changed.')
    finally:
        mac.release(source)
    print('SETTINGS_COMPOSITION_INPUT_SOURCE', source_id, flush=True)
    mac.press(TITLE, 'Settings')
    label, role = 'Settings workspace name', 'AXTextField'
    help_ = 'Finish composing text before resetting this field.'
    def stored(text):
        mac.release(mac.wait_find(TITLE, f'Stored name: {text} · region: Americas · model: 000 · custom: 0', 'AXStaticText'))
    focus_gallery_control(mac, label, role)
    mac.key(0, flags=1 << 20)
    mac.key(0)
    expect_field(mac, TITLE, label, 'a')
    stored('a')
    original = mac.wait_find(TITLE, label, role)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    def composing(text, saved):
        mac.wait_text(TITLE, help_)
        expect_field(mac, TITLE, label, text)
        stored(saved)
        expect_focus(mac, label, role)
        current = mac.wait_find(TITLE, label, role)
        try:
            assert equal(original, current), 'Composition lost its native editor identity'
        finally:
            mac.release(current)
    try:
        mac.key(14, flags=1 << 19)  # Option+E starts the OS dead-key preedit.
        composing('a´', 'a')
        for before, after in [
            ('Narrow settings', 'Widen settings'),
            ('Widen settings', 'Narrow settings'),
            ('Outline groups', 'Filled groups'),
            ('Filled groups', 'Plain groups'),
            ('Plain groups', 'Card groups'),
            ('Card groups', 'Outline groups'),
            ('Medium fields', 'Large fields'),
            ('Large fields', 'Small fields'),
            ('Small fields', 'Medium fields'),
            ('Dark', 'Light'),
            ('Light', 'Dark'),
        ]:
            mac.press(TITLE, before)
            mac.release(mac.wait_find(TITLE, after, 'AXButton'))
            composing('a´', 'a')
        mac.press(TITLE, 'Reset workspace-name')
        mac.wait_text(TITLE, 'Name reset: Composing')
        composing('a´', 'a')
        # One native reset fails while a separate Boolean reset can succeed.
        activate(mac, mac.wait_find(TITLE, 'Settings notifications', 'AXCheckBox'))
        expect_enabled(mac, 'Reset notifications', True)
        composing('a´', 'a')
        mac.press(TITLE, 'Reset entire page')
        mac.wait_text(TITLE, 'Name reset: Composing')
        expect_enabled(mac, 'Reset notifications', False)
        composing('a´', 'a')
        mac.key(14)  # E commits the actual composed character.
        expect_field(mac, TITLE, label, 'aé')
        stored('aé')
        absent(mac, help_, 'AXStaticText')
        mac.key(6, flags=1 << 20)  # Command+Z: one native composition undo.
        expect_field(mac, TITLE, label, 'a')
        stored('a')
        mac.key(6, flags=(1 << 20) | (1 << 17))
        expect_field(mac, TITLE, label, 'aé')
        stored('aé')
        mac.key(14, flags=1 << 19)
        composing('aé´', 'aé')
        # An explicit page departure retires preedit, not the saved setting.
        activate(mac, mac.wait_find(TITLE, 'Advanced', 'AXLink'))
        mac.release(mac.wait_find(TITLE, 'Experiment 00', 'AXStaticText'))
        absent(mac, label, role)
        stored('aé')
        activate(mac, mac.wait_find(TITLE, 'Workspace', 'AXLink'))
        expect_field(mac, TITLE, label, 'aé')
        stored('aé')
        current = mac.wait_find(TITLE, label, role)
        try:
            assert not equal(original, current), 'Page retirement retained the old native owner'
        finally:
            mac.release(current)
        absent(mac, help_, 'AXStaticText')
        mac.press(TITLE, 'Reset workspace-name')
        expect_field(mac, TITLE, label, 'Northstar')
        stored('Northstar')
    finally:
        mac.release(original)
    activate(mac, mac.wait_find(TITLE, 'Generation', 'AXLink'))
    numeric = 'Settings response budget'
    numeric_help = 'Finish composing text before resetting this number.'
    focus_gallery_control(mac, numeric, role)
    mac.key(0, flags=1 << 20)
    for code in (18, 14, 27):  # 1e- is a persistent unfinished numeric draft.
        mac.key(code)
    expect_field(mac, TITLE, numeric, '1e-')
    mac.wait_text(TITLE, 'Finish this number before committing.')
    mac.key(14, flags=1 << 19)
    mac.wait_text(TITLE, numeric_help)
    expect_field(mac, TITLE, numeric, '1e-´')
    mac.press(TITLE, 'Reset budget')
    mac.wait_text(TITLE, 'Budget reset: Composing')
    expect_field(mac, TITLE, numeric, '1e-´')
    activate(mac, mac.wait_find(TITLE, 'Advanced', 'AXLink'))
    mac.release(mac.wait_find(TITLE, 'Experiment 00', 'AXStaticText'))
    absent(mac, numeric, role)
    activate(mac, mac.wait_find(TITLE, 'Workspace', 'AXLink'))
    activate(mac, mac.wait_find(TITLE, 'Generation', 'AXLink'))
    expect_field(mac, TITLE, numeric, '1e-')
    absent(mac, numeric_help, 'AXStaticText')
    focus_gallery_control(mac, numeric, role)
    mac.key(53)  # Cancel the draft, restoring the separate committed value.
    expect_field(mac, TITLE, numeric, '25')
    expect_enabled(mac, 'Reset budget', False)
    # Escape ends composition in the same owner. Its OS dead-key context must
    # also end, or the subsequent replacement receives the canceled accent.
    mac.key(0, flags=1 << 20)
    mac.key(18)
    mac.key(14, flags=1 << 19)
    mac.wait_text(TITLE, numeric_help)
    expect_field(mac, TITLE, numeric, '1´')
    mac.key(53)
    absent(mac, numeric_help, 'AXStaticText')
    mac.key(0, flags=1 << 20)
    mac.key(19)
    expect_field(mac, TITLE, numeric, '2')
    mac.key(53)
    expect_field(mac, TITLE, numeric, '25')
    print('GALLERY_SETTINGS_COMPOSITION_OK: real US dead-key input, stable preedit/focus/identity across layout/theme/size changes, committed-value mirroring, reset rejection/partial success, native undo/redo and page retirement', flush=True)


def exercise_settings_resize(mac, original):
    """Drive the public native divider, preserving the editor and its draft."""
    label = 'Settings sidebar width'
    def rectangle():
        node = mac.wait_find(TITLE, label, 'AXSplitter')
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)
    def settled_x(expected):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            actual = rectangle()[0]
            # Native drag coordinates and AX frames may round by one logical pixel.
            if abs(actual - expected) <= 1.5:
                return
            time.sleep(.04)
        raise AssertionError(('Settings divider position', expected, actual))
    def drag(dx):
        x, y, w, h = reveal_gallery_control(mac, label, 'AXSplitter')
        start = (x + w / 2, y + h / 2)
        mouse, point = GalleryMouse(mac), start
        mouse.check_owner(start)
        mouse.send(5, start)
        mouse.send(1, start)
        try:
            for step in range(1, 13):
                point = (start[0] + dx * step / 12, start[1])
                mouse.check_owner(point)
                mouse.send(6, point)
                time.sleep(.02)
        finally:
            mouse.send(2, point)
    initial = rectangle()[0]
    drag(60)
    settled_x(initial + 60)
    focus_gallery_control(mac, label, 'AXSplitter')
    mac.key(123)  # Left: native 16px keyboard resize.
    settled_x(initial + 44)
    mac.key(124)
    settled_x(initial + 60)
    drag(-60)
    settled_x(initial)
    expect_field(mac, TITLE, 'Settings workspace name', 'a')
    current = mac.wait_find(TITLE, 'Settings workspace name', 'AXTextField')
    try:
        assert mac.cf.CFEqual(original, current), 'Native resizing replaced the name editor'
    finally:
        mac.release(current)
    print('GALLERY_SETTINGS_RESIZE_OK: pointer and keyboard geometry, retained editor/draft', flush=True)


def exercise_settings_policy(mac):
    focus_gallery_control(mac, 'Managed preferences', 'AXLink')
    mac.key(36)  # Real keyboard navigation moves focus out of the editor.
    mac.release(mac.wait_find(TITLE, 'Managed preferences', 'AXStaticText'))
    # Disabled rows retain native semantic names and values, but cannot activate.
    expect_enabled(mac, 'Settings custom action', False)
    expect_enabled(mac, 'Reset custom', False)
    expect_enabled(mac, 'Settings organization policy', False, role='AXCheckBox')
    original = mac.wait_find(TITLE, 'Settings custom action', 'AXButton')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    try:
        activate(mac, mac.wait_find(TITLE, 'Lock custom setting', 'AXCheckBox'))
        expect_enabled(mac, 'Settings custom action', True)
        current = mac.wait_find(TITLE, 'Settings custom action', 'AXButton')
        try:
            assert equal(original, current), 'Enabling replaced the native semantic control'
        finally:
            mac.release(current)
    finally:
        mac.release(original)
    deadline = time.monotonic() + 8
    while True:
        control = mac.wait_find(TITLE, 'Settings custom action', 'AXButton')
        divider = mac.wait_find(TITLE, 'Settings sidebar width', 'AXSplitter')
        try:
            x, y, w, h = element_rect(mac, control)
            dx, dy, dw, dh = element_rect(mac, divider)
        finally:
            mac.release(control)
            mac.release(divider)
        if w > 0 and h > 0 and x >= dx + dw - 1 and y >= dy and y + h <= dy + dh:
            break
        assert time.monotonic() < deadline, ('Custom target not visible in Settings', (x,y,w,h), (dx,dy,dw,dh))
        time.sleep(.04)
    focus_gallery_control(mac, 'Settings custom action', 'AXButton')
    mac.key(36)
    mac.wait_text(TITLE, 'custom: 1')
    mac.key(36)
    mac.wait_text(TITLE, 'custom: 2')
    expect_enabled(mac, 'Reset custom', True)
    retained = mac.wait_find(TITLE, 'Settings custom action', 'AXButton')
    try:
        activate(mac, mac.wait_find(TITLE, 'Lock custom setting', 'AXCheckBox'))
        expect_enabled(mac, 'Settings custom action', False)
        expect_enabled(mac, 'Reset custom', False)
        # A reference obtained while enabled must respect the current policy.
        try:
            mac.perform(retained, 'AXPress')
        except RuntimeError:
            pass
        mac.key(36)  # The formerly focused control must no longer activate.
        time.sleep(.15)
        mac.wait_text(TITLE, 'custom: 2')
    finally:
        mac.release(retained)
    # Dirty, disabled data is intentionally excluded from whole-page reset.
    mac.press(TITLE, 'Reset entire page')
    mac.wait_text(TITLE, 'custom: 2')
    activate(mac, mac.wait_find(TITLE, 'Lock custom setting', 'AXCheckBox'))
    expect_enabled(mac, 'Reset custom', True)
    mac.press(TITLE, 'Reset custom')
    mac.wait_text(TITLE, 'custom: 0')
    expect_enabled(mac, 'Reset custom', False)
    activate(mac, mac.wait_find(TITLE, 'Lock custom setting', 'AXCheckBox'))
    activate(mac, mac.wait_find(TITLE, 'Make it yours', 'AXLink'))
    print('GALLERY_SETTINGS_POLICY_OK: locked custom input/reset, OS activation and retained dirty data', flush=True)


def exercise_settings_virtualization(mac):
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    activate(mac, mac.wait_find(TITLE, 'Advanced', 'AXLink'))
    expand = mac.find(TITLE, 'Expand Advanced', 'AXButton')
    if expand:
        activate(mac, expand)
    original = mac.wait_find(TITLE, 'Settings feature 00', 'AXCheckBox')
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    def expect_checked(label, expected):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, label, 'AXCheckBox')
            value = mac.attr(node, 'AXValue')
            try:
                if value and bool(boolean(value)) == expected:
                    return
            finally:
                if value:
                    mac.release(value)
                mac.release(node)
            time.sleep(.03)
        raise AssertionError((label, 'checked', expected))
    def active_features():
        found = set()
        def visit(node):
            values, children = mac.node_values(node)
            try:
                if values[0] == 'AXCheckBox' and values[1].startswith('Settings feature '):
                    found.add(values[1])
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        window = mac.window(TITLE)
        try:
            visit(window)
        finally:
            mac.release(window)
        assert 0 < len(found) <= 32, ('Managed group budget', len(found))
        return found
    try:
        expect_checked('Settings feature 00', False)
        focus_gallery_control(mac, 'Settings feature 00', 'AXCheckBox')
        mac.key(49)
        expect_checked('Settings feature 00', True)
        before = active_features()
        activate(mac, mac.wait_find(TITLE, 'Experiment 47', 'AXLink'))
        mac.release(mac.wait_find(TITLE, 'Settings feature 47', 'AXCheckBox'))
        # AXPress requests navigation without moving focus from the first row.
        retained = mac.wait_find(TITLE, 'Settings feature 00', 'AXCheckBox')
        try:
            assert equal(original, retained), 'Focused row was replaced'
        finally:
            mac.release(retained)
        expect_focus(mac, 'Settings feature 00', 'AXCheckBox')
        focus_gallery_control(mac, 'Settings feature 47', 'AXCheckBox')
        wait_absent(mac, 'Settings feature 00', 'AXCheckBox')
        after = active_features()
        activate(mac, mac.wait_find(TITLE, 'Experiment 00', 'AXLink'))
        expect_checked('Settings feature 00', True)
        remounted = mac.wait_find(TITLE, 'Settings feature 00', 'AXCheckBox')
        try:
            assert not equal(original, remounted), 'Eviction did not retire native row'
        finally:
            mac.release(remounted)
        mac.press(TITLE, 'Reset feature-00')
        expect_checked('Settings feature 00', False)
        print('GALLERY_SETTINGS_VIRTUALIZATION_OK: 48 groups, focused pin, blur eviction, '
              f'remounted saved state; AX materialized counts {len(before)}/{len(after)}', flush=True)
    finally:
        mac.release(original)
    activate(mac, mac.wait_find(TITLE, 'Workspace', 'AXLink'))
    activate(mac, mac.wait_find(TITLE, 'Make it yours', 'AXLink'))


def exercise_settings_windows(mac):
    """Independent application data and native owners in two normal windows."""
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    mac.press(TITLE, 'Settings')
    activate(mac, mac.wait_find(TITLE, 'Workspace', 'AXLink'))
    activate(mac, mac.wait_find(TITLE, 'Make it yours', 'AXLink'))
    def titles():
        windows = mac.children(mac.app, 'AXWindows')
        try:
            return {mac.text(window, 'AXTitle') for window in windows}
        finally:
            for window in windows:
                mac.release(window)
    previous, second = titles(), None
    mac.press(TITLE, 'New window')
    try:
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            created = titles() - previous
            if created:
                assert len(created) == 1, created
                second = created.pop()
                break
            time.sleep(.04)
        assert second, 'New gallery window did not appear'
        mac.press(second, 'Settings')
        expect_field(mac, second, 'Settings workspace name', 'Northstar')
        expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        def focus_field(title):
            # AXRaise orders a window but need not make it the AppKit key window.
            # Use an actual click before routing physical keys between windows.
            window = mac.window(title)
            field = mac.wait_find(title, 'Settings workspace name', 'AXTextField')
            try:
                mac.set(mac.app, 'AXFrontmost', mac.true)
                mac.perform(window, 'AXRaise')
                mac.set(window, 'AXMain', mac.true)
                x, y, w, h = element_rect(mac, field)
                mouse, point = GalleryMouse(mac), (x + w / 2, y + h / 2)
                mouse.check_owner(point)
                system = mac.ax.AXUIElementCreateSystemWide()
                hit = C.c_void_p()
                try:
                    code = mac.ax.AXUIElementCopyElementAtPosition(system, *point, C.byref(hit))
                    assert not code and hit.value
                    # Same-process/window ownership alone can pass when AppKit
                    # returns AXWindow instead of routing into the content view.
                    # Require the actual editor at this known content point.
                    assert equal(hit, field), (
                        'Window point did not resolve to its visible editor', title,
                        mac.text(hit, 'AXRole'), mac.text(hit, 'AXTitle'),
                        mac.text(hit, 'AXDescription'), point)
                    hit_window = (mac.retain(hit) if mac.text(hit, 'AXRole') == 'AXWindow'
                                  else mac.attr(hit, 'AXWindow'))
                    try:
                        assert hit_window and mac.text(hit_window, 'AXTitle') == title, (
                            'Another gallery window covers the input target', title)
                    finally:
                        if hit_window:
                            mac.release(hit_window)
                finally:
                    if hit.value:
                        mac.release(hit)
                    mac.release(system)
                mouse.send(5, point)
                mouse.send(1, point)
                mouse.send(2, point)
                time.sleep(.08)
                mouse.send(1, point)
                mouse.send(2, point)
            finally:
                mac.release(field)
                mac.release(window)
            expect_focus(mac, 'Settings workspace name', 'AXTextField', title=title)
        focus_field(second)
        mac.key(0, flags=1 << 20)
        mac.key(11)  # b in the second window
        expect_field(mac, second, 'Settings workspace name', 'b')
        expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        focus_field(TITLE)
        expect_focus(mac, 'Settings workspace name', 'AXTextField', title=second, focused=False)
        mac.key(0, flags=1 << 20)
        mac.key(7)  # x in the first window
        expect_field(mac, TITLE, 'Settings workspace name', 'x')
        expect_field(mac, second, 'Settings workspace name', 'b')
        focus_field(second)
        expect_focus(mac, 'Settings workspace name', 'AXTextField', focused=False)
        mac.key(0, flags=1 << 20)
        mac.key(8)  # c after returning to the second window.
        expect_field(mac, second, 'Settings workspace name', 'c')
        expect_field(mac, TITLE, 'Settings workspace name', 'x')
        focus_field(TITLE)
        expect_focus(mac, 'Settings workspace name', 'AXTextField', title=second, focused=False)
        mac.press(TITLE, 'Reset workspace-name')
        expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        expect_field(mac, second, 'Settings workspace name', 'c')
        mac.close(second)
        second = None
        focus_field(TITLE)
        mac.key(0, flags=1 << 20)
        mac.key(7)
        expect_field(mac, TITLE, 'Settings workspace name', 'x')
        mac.press(TITLE, 'Reset workspace-name')
        expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        print('GALLERY_SETTINGS_WINDOWS_OK: independent OS edits/resets, initial and switched AX focus, remaining window usable after close', flush=True)
    finally:
        if second:
            window = mac.window(second)
            if window:
                mac.release(window)
                mac.close(second)
        raise_gallery(mac)


def exercise_settings(mac, images):
    mac.press(TITLE, 'Settings')
    mac.wait_text(TITLE, 'Make it yours')
    original = mac.wait_find(TITLE, 'Settings workspace name', 'AXTextField')
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    try:
        focus_gallery_control(mac, 'Settings workspace name', 'AXTextField')
        mac.key(0, flags=1 << 20)
        mac.key(0)
        expect_field(mac, TITLE, 'Settings workspace name', 'a')
        mac.wait_text(TITLE, 'Stored name: a')
        exercise_settings_resize(mac, original)
        mac.press(TITLE, 'Narrow settings')
        expect_field(mac, TITLE, 'Settings workspace name', 'a')
        current = mac.wait_find(TITLE, 'Settings workspace name', 'AXTextField')
        try:
            assert equal(original, current), 'Responsive settings replaced the editor'
        finally:
            mac.release(current)
        mac.press(TITLE, 'Widen settings')
        for label in ['Outline groups', 'Filled groups', 'Plain groups', 'Card groups']:
            mac.press(TITLE, label)
            expect_field(mac, TITLE, 'Settings workspace name', 'a')
        activate(mac, mac.wait_find(TITLE, 'Advanced', 'AXLink'))
        mac.wait_text(TITLE, 'Experiment 00')
        activate(mac, mac.wait_find(TITLE, 'Workspace', 'AXLink'))
        expect_field(mac, TITLE, 'Settings workspace name', 'a')
        remounted = mac.wait_find(TITLE, 'Settings workspace name', 'AXTextField')
        try:
            assert not equal(original, remounted), 'Page departure did not retire editor'
        finally:
            mac.release(remounted)
        # Keep the numeric draft independent from its committed value across visits.
        activate(mac, mac.wait_find(TITLE, 'Generation', 'AXLink'))
        focus_gallery_control(mac, 'Settings response budget', 'AXTextField')
        mac.key(0, flags=1 << 20)
        mac.key(18)  # 1
        mac.key(14)  # e
        mac.key(27)  # minus
        expect_field(mac, TITLE, 'Settings response budget', '1e-')
        mac.wait_text(TITLE, 'Finish this number before committing.')
        activate(mac, mac.wait_find(TITLE, 'Advanced', 'AXLink'))
        mac.release(mac.wait_find(TITLE, 'Settings feature 00', 'AXCheckBox'))
        activate(mac, mac.wait_find(TITLE, 'Workspace', 'AXLink'))
        expect_field(mac, TITLE, 'Settings workspace name', 'a')
        activate(mac, mac.wait_find(TITLE, 'Generation', 'AXLink'))
        expect_field(mac, TITLE, 'Settings response budget', '1e-')
        focus_gallery_control(mac, 'Settings response budget', 'AXTextField')
        mac.key(53)  # Escape restores committed 25
        expect_field(mac, TITLE, 'Settings response budget', '25')
        focus_gallery_control(mac, 'Settings model', 'AXPopUpButton')
        mac.key(49)  # Space opens the native popup
        mac.key(119)  # End reaches the last of 250 virtualized options
        mac.key(36)
        mac.wait_text(TITLE, 'model: 249')
        activate(mac, mac.wait_find(TITLE, 'Make it yours', 'AXLink'))
        def group_reset(title):
            deadline, seen = time.monotonic() + 10, set()
            def visit(node):
                if mac.text(node, 'AXRole') == 'AXButton' and mac.text(node, 'AXTitle') == 'Reset group':
                    help_text = mac.text(node, 'AXHelp')
                    seen.add(help_text)
                    if help_text == title:
                        return mac.retain(node)
                children = mac.children(node)
                try:
                    for child in children:
                        found = visit(child)
                        if found:
                            return found
                finally:
                    for child in children:
                        mac.release(child)
            while time.monotonic() < deadline:
                root = mac.window(TITLE)
                try:
                    button = visit(root)
                finally:
                    mac.release(root)
                if button:
                    activate(mac, button)
                    return
                time.sleep(.04)
            raise RuntimeError(f'Group reset for {title!r} was not available; descriptions={seen}')
        group_reset('Make it yours')
        expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        mac.wait_text(TITLE, 'model: 249')  # Another group's setting stays changed.
        focus_gallery_control(mac, 'Settings workspace name', 'AXTextField')
        mac.key(0, flags=1 << 20)
        mac.key(0)
        expect_field(mac, TITLE, 'Settings workspace name', 'a')
        activate(mac, mac.wait_find(TITLE, 'Generation', 'AXLink'))
        mac.press(TITLE, 'Reset model')
        mac.wait_text(TITLE, 'model: 000')
        # Search removes and remounts whole groups without deleting application drafts.
        focus_gallery_control(mac, 'Search settings', 'AXTextField')
        mac.key(0)  # a matches broad metadata
        mac.key(0, flags=1 << 20)
        mac.key(6)
        mac.key(6)  # zz: no matching setting
        mac.wait_text(TITLE, 'No matching settings')
        mac.key(0, flags=1 << 20)
        mac.key(51)  # clear query
        # Clearing search restores the preferred group (Generation), rather
        # than promising that the first group is visible after remount.
        expect_field(mac, TITLE, 'Settings response budget', '25')
        activate(mac, mac.wait_find(TITLE, 'Make it yours', 'AXLink'))
        expect_field(mac, TITLE, 'Settings workspace name', 'a')
        # A whole-page reset must include filtered-out native editor placements.
        def search_keys(keys):
            focus_gallery_control(mac, 'Search settings', 'AXTextField')
            mac.key(0, flags=1 << 20)
            mac.key(51)
            for code in keys:
                mac.key(code)
        search_keys([11, 32, 2, 5, 14, 17])  # budget
        wait_absent(mac, 'Settings workspace name', 'AXTextField')
        mac.press(TITLE, 'Reset entire page')
        mac.wait_text(TITLE, 'Stored name: Northstar')
        focus_gallery_control(mac, 'Settings response budget', 'AXTextField')
        mac.key(0, flags=1 << 20)
        for code in [18, 14, 27]:
            mac.key(code)  # 1e-
        expect_field(mac, TITLE, 'Settings response budget', '1e-')
        search_keys([45, 0, 46, 14])  # name
        wait_absent(mac, 'Settings response budget', 'AXTextField')
        mac.press(TITLE, 'Reset entire page')
        search_keys([])
        expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        activate(mac, mac.wait_find(TITLE, 'Generation', 'AXLink'))
        expect_field(mac, TITLE, 'Settings response budget', '25')
        activate(mac, mac.wait_find(TITLE, 'Make it yours', 'AXLink'))
        mac.press(TITLE, 'Try failed export')
        mac.wait_text(TITLE, 'Export failed: Preview writer is unavailable.')
        expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        focus_gallery_control(mac, 'Settings workspace name', 'AXTextField')
        mac.key(0, flags=1 << 20)
        mac.key(51)
        expect_field(mac, TITLE, 'Settings workspace name', '')
        mac.wait_text(TITLE, 'Enter a workspace name.')
        mac.press(TITLE, 'Export settings…')
        mac.wait_text(TITLE, 'Enter a workspace name before exporting.')
        assert not mac.find(TITLE, 'Cancel', 'AXButton'), 'Invalid export opened a Save panel'
        expect_field(mac, TITLE, 'Settings workspace name', '')
        mac.press(TITLE, 'Reset workspace-name')
        expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        # Paste through AppKit, retaining the user's previous clipboard.
        clipboard_env = dict(os.environ, LANG='en_US.UTF-8', LC_ALL='en_US.UTF-8')
        previous_clipboard = subprocess.run(['/usr/bin/pbpaste'], capture_output=True, check=True, env=clipboard_env).stdout
        try:
            text = 'Aster 京都 👩‍💻'
            subprocess.run(['/usr/bin/pbcopy'], input=text.encode(), check=True, env=clipboard_env)
            focus_gallery_control(mac, 'Settings workspace name', 'AXTextField')
            mac.key(0, flags=1 << 20)
            mac.key(9, flags=1 << 20)
            expect_field(mac, TITLE, 'Settings workspace name', text)
            mac.wait_text(TITLE, 'Stored name: ' + text)
            mac.press(TITLE, 'Reset workspace-name')
            expect_field(mac, TITLE, 'Settings workspace name', 'Northstar')
        finally:
            subprocess.run(['/usr/bin/pbcopy'], input=previous_clipboard, check=True, env=clipboard_env)
        mac.press(TITLE, 'Export settings…')
        mac.release(mac.wait_find(TITLE, 'Cancel', 'AXButton'))
        mac.press(TITLE, 'Cancel')
        mac.press(TITLE, 'Try failed export')
        mac.wait_text(TITLE, 'Export failed: Preview writer is unavailable.')
        # The dialog starts in /tmp. A private random marker reserves the test's
        # namespace; only our unique sibling export is removed in cleanup.
        with tempfile.TemporaryDirectory(prefix='gpuio-settings-export-', dir='/private/tmp') as marker:
            exported = Path(marker).with_suffix('.sexp')
            assert not exported.exists()
            try:
                mac.press(TITLE, 'Export settings…')
                filename = mac.wait_find(TITLE, 'gpuio-settings.sexp', 'AXTextField')
                name = mac.string(exported.name)
                try:
                    mac.set(filename, 'AXValue', name)
                finally:
                    mac.release(name)
                    mac.release(filename)
                mac.press(TITLE, 'Save')
                mac.wait_text(TITLE, 'Preview settings exported.')
                content = exported.read_text()
                assert 'gpuio-settings-preview-v1' in content and 'Northstar' in content, content
                assert '1e-' not in content, 'Uncommitted numeric draft was exported'
            finally:
                exported.unlink(missing_ok=True)
        exercise_settings_policy(mac)
        exercise_settings_virtualization(mac)
        if images:
            screenshot(mac, images / 'gallery-settings.png', title=TITLE)
    finally:
        mac.release(original)
    print('GALLERY_SETTINGS_OK: keyboard/Unicode edits, responsive identity, variants, page/search draft recovery, long choices, group/hidden-editor resets, export validation/failure/cancel/save/readback', flush=True)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--images', type=Path)
    parser.add_argument('--executable', type=Path,
                        help='Run an independently built gallery instead of the repository executable')
    parser.add_argument('--background', action='store_true', help='Launch without focus; input scenarios may explicitly raise the window')
    parser.add_argument('--trace-canvas', action='store_true')
    parser.add_argument('--trace-motion', action='store_true')
    parser.add_argument('--trace-windows', action='store_true')
    parser.add_argument('--section', choices=['all', 'core', 'settings', 'settings-windows', 'settings-composition', 'settings-fields', 'forms', 'editor-groups', 'avatar-groups', 'rating', 'spinners', 'progress', 'selection', 'buttons', 'button-appearance', 'menu-observation', 'split-buttons', 'command-tooltip', 'control-appearance', 'status-regions', 'badges', 'labels', 'shimmer', 'markers', 'alerts', 'tags', 'keyboard-labels', 'binding-observations', 'descriptions', 'chat-composition', 'chat-list', 'attachments', 'attachment-paint', 'groups', 'links', 'empty', 'separators', 'styles', 'borders', 'aspect-ratio', 'pickers', 'choice-pickers', 'overlays', 'navigation', 'feedback', 'journeys', 'collections', 'selectable-lists', 'structural-tables', 'documents', 'document-links', 'document-images', 'highlighting', 'canvas', 'assets', 'clipboard', 'charts', 'motion', 'responsive', 'extensions', 'input', 'observations', 'desktop', 'runtime'], default='all')
    args = parser.parse_args()
    Mac.require_accessibility()
    if args.images:
        args.images.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    env = os.environ.copy()
    if args.section in ('all', 'extensions'):
        env['GPUIO_COUNTER_TRACE'] = '1'
    with tempfile.TemporaryFile(mode='w+') as log:
        executable = args.executable.resolve() if args.executable else repo / '_build/default/examples/gallery/main.exe'
        child = subprocess.Popen([str(executable),
                                  *(['--background'] if args.background else []),
                                  *(['--trace-canvas'] if args.trace_canvas else []),
                                  *(['--trace-motion'] if args.trace_motion else []),
                                  *(['--trace-windows'] if args.trace_windows else []),
                                  *(['--trace-input'] if args.section in ('all', 'input') else []),
                                  *(['--open-uri=gpuio-studio://preview/startup'] if args.section in ('all', 'desktop') else [])],
                                 cwd=repo, env=env, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            if args.section in ('all', 'core'):
                exercise(mac, args.images)
            if args.section == 'links':
                exercise_links(mac, args.images)
            if args.section == 'separators':
                exercise_separators(mac, args.images)
            if args.section == 'empty':
                exercise_empty(mac, args.images)
            if args.section == 'groups':
                exercise_groups(mac, args.images)
            if args.section == 'aspect-ratio':
                exercise_aspect_ratio(mac, args.images)
            if args.section == 'attachment-paint':
                exercise_attachment_paint(mac, args.images)
            if args.section == 'attachments':
                exercise_attachments(mac, args.images)
            if args.section == 'binding-observations':
                exercise_binding_observations(mac, args.images)
            if args.section == 'keyboard-labels':
                exercise_keyboard_labels(mac, args.images)
            if args.section == 'descriptions':
                exercise_descriptions(mac, args.images)
            if args.section == 'chat-composition':
                exercise_chat_composition(mac, args.images)
            if args.section == 'chat-list':
                exercise_chat_list(mac, args.images)
            if args.section == 'tags':
                exercise_tags(mac, args.images)
            if args.section == 'alerts':
                exercise_alerts(mac, args.images)
            if args.section == 'markers':
                exercise_markers(mac, args.images)
            if args.section == 'shimmer':
                exercise_shimmer(mac, args.images)
            if args.section == 'labels':
                exercise_labels(mac, args.images)
            if args.section == 'badges':
                exercise_badges(mac, args.images)
            if args.section == 'status-regions':
                exercise_status_regions(mac, args.images)
            if args.section == 'borders':
                exercise_borders(mac, args.images)
            if args.section in ('all', 'settings'):
                exercise_settings(mac, args.images)
            if args.section in ('all', 'avatar-groups'):
                from gallery_avatar_group import exercise as exercise_avatar_group
                exercise_avatar_group(mac, args.images)
            if args.section in ('all', 'rating'):
                from gallery_rating import exercise as exercise_rating
                exercise_rating(mac, args.images)
            if args.section in ('all', 'selection'):
                from gallery_selection import exercise as exercise_selection
                exercise_selection(mac, args.images)
            if args.section in ('all', 'buttons'):
                from gallery_buttons import exercise as exercise_buttons
                exercise_buttons(mac, args.images)
            if args.section in ('all', 'buttons', 'button-appearance'):
                from gallery_button_appearance import exercise as exercise_button_appearance
                exercise_button_appearance(mac, args.images)
            if args.section in ('all', 'buttons', 'menu-observation'):
                from gallery_menu_observation import exercise as exercise_menu_observation
                exercise_menu_observation(mac, args.images)
            if args.section in ('all', 'buttons', 'split-buttons'):
                from gallery_split import exercise as exercise_split
                exercise_split(mac, args.images)
            if args.section in ('all', 'buttons', 'command-tooltip'):
                from gallery_command_tooltip import exercise as exercise_command_tooltip
                exercise_command_tooltip(mac, args.images)
            if args.section in ('all', 'control-appearance'):
                from gallery_control_appearance import exercise as exercise_control_appearance
                exercise_control_appearance(mac, args.images)
            if args.section in ('all', 'spinners'):
                from gallery_spinner import exercise as exercise_spinners
                exercise_spinners(mac, args.images)
            if args.section in ('all', 'editor-groups'):
                from gallery_editor_groups import exercise as exercise_editor_groups
                exercise_editor_groups(mac, args.images)
            if args.section in ('all', 'forms'):
                from gallery_forms import exercise as exercise_forms
                exercise_forms(mac, args.images)
            if args.section == 'settings-fields':
                from gallery_settings_fields import exercise as exercise_settings_fields
                exercise_settings_fields(mac)
            if args.section == 'settings-composition':
                exercise_settings_composition(mac)
            if args.section in ('all', 'styles'):
                exercise_styles(mac, args.images)
            if args.section in ('all', 'pickers'):
                exercise_pickers(mac, args.images)
            if args.section in ('all', 'choice-pickers'):
                from gallery_choice_picker import exercise as exercise_choice_picker
                exercise_choice_picker(mac, args.images)
            if args.section in ('all', 'overlays'):
                exercise_overlays(mac, args.images)
            if args.section in ('all', 'navigation'):
                exercise_navigation(mac, args.images)
            if args.section in ('all', 'progress'):
                from gallery_progress import exercise as exercise_progress
                exercise_progress(mac, args.images)
            if args.section in ('all', 'feedback'):
                exercise_feedback(mac, args.images)
            if args.section in ('all', 'journeys'):
                exercise_journeys(mac, args.images)
            if args.section in ('all', 'collections'):
                exercise_collections(mac, args.images)
            if args.section in ('all', 'collections', 'structural-tables'):
                from gallery_structural_table import exercise as exercise_structural_table
                exercise_structural_table(mac, args.images)
            if args.section in ('all', 'collections', 'selectable-lists'):
                from gallery_selectable_list import exercise as exercise_selectable_list
                exercise_selectable_list(mac, args.images)
            if args.section == 'document-links':
                exercise_document_links(mac, args.images)
            if args.section == 'document-images':
                exercise_document_images(mac, args.images)
            if args.section in ('all', 'documents'):
                exercise_documents(mac, args.images)
            if args.section in ('all', 'highlighting'):
                exercise_highlighting(mac, args.images)
            if args.section in ('all', 'canvas'):
                exercise_canvas(mac, args.images)
            if args.section in ('all', 'assets'):
                exercise_assets(mac, args.images)
            if args.section in ('all', 'clipboard'):
                from gallery_clipboard import exercise as exercise_clipboard
                exercise_clipboard(mac, args.images)
            if args.section in ('all', 'charts'):
                exercise_charts(mac, args.images)
            if args.section in ('all', 'motion'):
                exercise_motion(mac, args.images,
                                second_title=('GPUIO · Component Studio 3'
                                              if args.section == 'all' else SECOND))
            if args.section in ('all', 'responsive'):
                exercise_responsive(mac, args.images)
            if args.section in ('all', 'extensions'):
                exercise_extensions(mac, args.images)
            if args.section in ('all', 'input'):
                exercise_input(mac, args.images)
            if args.section in ('all', 'observations'):
                exercise_observations(mac, args.images, second_title=('GPUIO · Component Studio 4' if args.section == 'all' else SECOND))
            if args.section in ('all', 'desktop'):
                exercise_desktop(mac, args.images, second_title=('GPUIO · Component Studio 5' if args.section == 'all' else SECOND))
            if args.section in ('all', 'runtime'):
                exercise_runtime(mac, args.images)
            if args.section in ('all', 'settings', 'settings-windows'):
                exercise_settings_windows(mac)
            mac.close(TITLE)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Gallery exited unsuccessfully')
        except Exception as error:
            print('GALLERY_FAILURE', repr(error), flush=True)
            if mac and child.poll() is None:
                try:
                    windows = mac.children(mac.app, 'AXWindows')
                    try:
                        print('GALLERY_FAILURE_WINDOWS',
                              [mac.text(window, 'AXTitle') for window in windows], flush=True)
                    finally:
                        for window in windows:
                            mac.release(window)
                    mac.dump(TITLE)
                    if args.images:
                        screenshot(mac, args.images / 'gallery-failure.png', title=TITLE)
                except Exception as diagnostic_error:
                    print('GALLERY_FAILURE_CAPTURE', diagnostic_error, flush=True)
            raise
        finally:
            if mac:
                mac.release(mac.app)
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            log.seek(0)
            output = log.read()
            print(output, end='')
        if args.section in ('all', 'extensions'):
            verify_extension_lifetimes(output)
        if args.section in ('all', 'input'):
            verify_input_transfers(output)
    print(f'GPUIO_GALLERY_AX_OK: section={args.section}, native actions, state semantics and shutdown')


if __name__ == '__main__':
    main()
