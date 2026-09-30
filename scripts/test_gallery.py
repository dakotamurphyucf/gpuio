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


def exercise(mac, images):
    mac.wait_text(TITLE, 'A little context goes a long way')
    exercise_status_regions(mac, images)
    exercise_badges(mac, images)
    exercise_labels(mac, images)
    exercise_groups(mac, images)
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


def reveal_gallery_control(mac, label, role):
    mouse = GalleryMouse(mac)
    window = mac.window(TITLE)
    try:
        wx, wy, ww, wh = element_rect(mac, window)
    finally:
        mac.release(window)
    point = (wx + ww * .78, wy + wh * .67)
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
        event = create(None, 0, 1, C.c_int(-75 if y + h > wy + wh - 30 else 75))
        assert event
        try:
            locate(event, GalleryMouse.Point(*point))
            mouse.post(0, event)
        finally:
            mac.release(event)
        time.sleep(.08)
    raise RuntimeError(f'{label} did not become visible')


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
    mac.wait_text(TITLE, 'Images: 0 · Charts: 0 · Canvases: 0')
    mac.press(TITLE, 'Presentation')
    mac.release(mac.wait_find(TITLE, labels[0], 'AXLink'))
    print('GALLERY_COMPOSED_LINK_OK: 8 theme/content/icon cases plus image/avatar/loading, '
          '42 pointer/Return/Space/AX '
          'actions, stable identity, signed Tab/reverse order and focused non-stop anchors '
          'with viewport reveal, disabled recovery and scoped SVG cleanup', flush=True)


def exercise_empty(mac, images):
    """Public rich slots: geometry, native identity and independent action owners."""
    mac.wait_text(TITLE, 'Make space for a fresh start.')
    raise_gallery(mac)
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    original = mac.wait_find(TITLE, 'Keep empty-state updates', 'AXCheckBox')
    clicks = cases = 0
    toggles = {'Frame empty media': False, 'Align empty slots to start': False,
               'Narrow empty preview': False}

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
    mac.wait_text(TITLE, 'Images: 0 · Charts: 0 · Canvases: 0')
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, f'Empty actions: {clicks}')
    print(f'GALLERY_EMPTY_OK: {cases} theme/media/alignment/width cases; intrinsic media, '
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
    mac.wait_text(TITLE, 'Images: 0 · Charts: 0 · Canvases: 0')
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
    mac.wait_text(TITLE, 'Images: 0 · Charts: 0 · Canvases: 0')
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
    mac.wait_text(TITLE, 'Images: 0 · Charts: 0 · Canvases: 0')
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


def expect_focus(mac, trigger, role="AXButton"):
    get = mac.cf.CFBooleanGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        node = mac.wait_find(TITLE, trigger, role)
        value = mac.attr(node, 'AXFocused')
        try:
            if value and get(value):
                return
        finally:
            if value:
                mac.release(value)
            mac.release(node)
        time.sleep(.03)
    raise RuntimeError(f'Focus not restored to {trigger}')


def exercise_pickers(mac, images):
    mac.press(TITLE, 'Dates & colors')
    mac.wait_text(TITLE, 'Appointment: 2026-09-14')
    open_picker(mac, 'Choose appointment', 'Cancel appointment')
    activate(mac, within(mac, 'Preview appointment', 'September 17, 2026', 'AXCheckBox'))
    mac.wait_text(TITLE, 'Appointment: 2026-09-14')
    mac.press(TITLE, 'Cancel appointment')
    expect_focus(mac, 'Choose appointment')
    mac.wait_text(TITLE, 'Appointment: 2026-09-14')
    open_picker(mac, 'Choose appointment', 'Cancel appointment')
    activate(mac, within(mac, 'Preview appointment', 'September 18, 2026', 'AXCheckBox'))
    mac.press(TITLE, 'Apply appointment')
    mac.wait_text(TITLE, 'Appointment: 2026-09-18')
    expect_focus(mac, 'Choose appointment')
    open_picker(mac, 'Choose accent', 'Cancel accent')
    activate(mac, within(mac, 'Preview accent', 'Iris', 'AXRadioButton'))
    mac.wait_text(TITLE, 'Accent: #89DDC9')
    mac.press(TITLE, 'Cancel accent')
    expect_focus(mac, 'Choose accent')
    open_picker(mac, 'Choose accent', 'Cancel accent')
    activate(mac, within(mac, 'Preview accent', 'Coral', 'AXRadioButton'))
    mac.press(TITLE, 'Apply accent')
    mac.wait_text(TITLE, 'Accent: #F6A89D')
    expect_focus(mac, 'Choose accent')
    if images:
        screenshot(mac, images / 'gallery-pickers.png', title=TITLE)
    open_picker(mac, 'Choose accent', 'Cancel accent')
    mac.key(53)
    expect_focus(mac, 'Choose accent')
    mac.wait_text(TITLE, 'Accent: #F6A89D')


def exercise_overlays(mac, images):
    mac.press(TITLE, 'Overlays & help')
    open_picker(mac, 'Open dialog', 'Close dialog')
    mac.key(53)
    expect_focus(mac, 'Open dialog')
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
    activate(mac, mac.wait_find(TITLE, 'Draft'))
    expect_field(mac, TITLE, 'Retained draft', 'A separate draft with its own native editing history.', role='AXTextArea')
    hidden = mac.find(TITLE, 'Retained notes', 'AXTextArea')
    if hidden:
        mac.release(hidden)
        raise RuntimeError('Inactive retained tab editor remains accessible')
    activate(mac, mac.wait_find(TITLE, 'Notes'))
    expect_field(mac, TITLE, 'Retained notes', 'a', role='AXTextArea')
    mac.press(TITLE, 'Identity')
    mac.wait_text(TITLE, 'Stable keys preserve the identity')
    mac.press(TITLE, 'Behavior')
    mac.wait_text(TITLE, 'Native controls handle immediate input')
    mac.press(TITLE, 'Next')
    mac.wait_text(TITLE, 'Preview page 2 of 12')
    mac.press(TITLE, 'Last')
    mac.wait_text(TITLE, 'Preview page 12 of 12')
    if images:
        screenshot(mac, images / 'gallery-navigation.png', title=TITLE)


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
    mac.wait_text(TITLE, 'Entry 0000')
    mac.press(TITLE, 'Last entry')
    mac.wait_text(TITLE, 'Entry 0999')
    mac.press(TITLE, 'Grow first entry')
    mac.wait_text(TITLE, 'Extra lines: 1 / 8')
    mac.wait_text(TITLE, 'Entry 0999')
    mac.press(TITLE, 'First entry')
    mac.wait_text(TITLE, 'Another useful detail.')
    mac.press(TITLE, 'Outline tree')
    mac.release(mac.wait_find(TITLE, 'Field notes', 'AXRow', search_files=True))
    mac.press(TITLE, 'Reveal observatory')
    activate(mac, mac.wait_find(TITLE, 'Observatory study', 'AXRow', search_files=True))
    mac.wait_text(TITLE, 'Selected outline: observatory')
    counts = tree_counts(mac, mac.wait_find(TITLE, 'Preview outline', 'AXOutline'))
    if not 1 <= counts.get('AXRow', 0) <= 16:
        raise RuntimeError(f'Outline escaped its row budget: {counts}')
    mac.press(TITLE, 'Result table')
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
    mac.wait_text(TITLE, 'Another useful detail.')
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
        assert len(headings) == 1, len(headings)
        assert number(headings[0], 'AXValue') == 1
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


def reveal_document_code(mac):
    # The table makes the initial Markdown taller. Scroll the actual native
    # viewport to mount the appended fence before looking for its Copy action.
    raise_gallery(mac)
    mouse = GalleryMouse(mac)
    create = mac.cg.CGEventCreateScrollWheelEvent
    create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_uint, C.c_uint, C.c_int]
    locate = mac.cg.CGEventSetLocation
    locate.restype, locate.argtypes = None, [C.c_void_p, GalleryMouse.Point]
    previous = None
    for _ in range(20):
        control = mac.find(TITLE, 'Copy code', 'AXButton')
        if control:
            mac.release(control)
            return
        # A newly selected preview can expose its toolbar before preparation
        # installs the body. Target the actual body, and follow any reflow.
        current = mouse.bounds('Document content')
        x, y, width, height = current
        assert width > 0 and height > 0, current
        point = (x + width / 2, y + height / 2)
        if current != previous:
            print('DOCUMENT_SCROLL_TARGET', current, 'point', point, flush=True)
            mouse.send(5, point)
            time.sleep(.1)
            previous = current
        mouse.check_owner(point)
        event = create(None, 0, 1, C.c_int(-80))
        assert event
        try:
            locate(event, GalleryMouse.Point(*point))
            mouse.post(0, event)
        finally:
            mac.release(event)
        time.sleep(.1)
    print('DOCUMENT_SCROLL_FINAL_BODY', mouse.bounds('Document content'), flush=True)
    raise RuntimeError('Appended code did not become accessible after native document scrolling')


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
    assert roles.get('AXHeading') == 1 and roles.get('AXList') == 1, roles
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
    reveal_document_code(mac)
    mac.release(mac.wait_find(TITLE, 'Copy code', 'AXButton'))
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
    reveal_document_code(mac)
    mac.release(mac.wait_find(TITLE, 'Copy code', 'AXButton'))
    if images:
        screenshot(mac, images / 'gallery-documents.png', title=TITLE)
    mac.press(TITLE, 'Reset document')
    mac.wait_text(TITLE, 'Document reset')
    wait_absent(mac, 'Copy code', 'AXButton')
    wait_absent(mac, 'Finding 1', 'AXStaticText')
    for _ in range(3):
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        absent(mac, 'Markdown preview', 'AXGroup')
        mac.press(TITLE, 'Markdown & code')
        mac.wait_text(TITLE, 'Markdown preview')
        document_structure(mac)
        wait_absent(mac, 'Copy code', 'AXButton')
        mac.press(TITLE, 'Append a finding')
        mac.wait_text(TITLE, 'Appended findings: 1 / 6')
        reveal_document_code(mac)
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
    mac.wait_text(TITLE, 'Images: 0 · Charts: 0 · Canvases: 0')
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
    mac.wait_text(TITLE, 'Images: 0 · Charts: 0 · Canvases: 0')
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
    mac.wait_text(TITLE, 'Images: 0 · Charts: 0 · Canvases: 0')
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
            assert (not hit_test(root, *point, C.byref(hit)) and hit.value
                    and not get_pid(hit, C.byref(owner)) and owner.value == mac.pid), ('Pointer target is occluded', point)
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--images', type=Path)
    parser.add_argument('--executable', type=Path,
                        help='Run an independently built gallery instead of the repository executable')
    parser.add_argument('--trace-canvas', action='store_true')
    parser.add_argument('--trace-motion', action='store_true')
    parser.add_argument('--section', choices=['all', 'core', 'status-regions', 'badges', 'labels', 'groups', 'links', 'empty', 'styles', 'pickers', 'overlays', 'navigation', 'feedback', 'journeys', 'collections', 'documents', 'document-links', 'document-images', 'highlighting', 'canvas', 'assets', 'charts', 'motion', 'responsive', 'extensions', 'input', 'observations', 'desktop', 'runtime'], default='all')
    args = parser.parse_args()
    if args.images:
        args.images.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    env = os.environ.copy()
    if args.section in ('all', 'extensions'):
        env['GPUIO_COUNTER_TRACE'] = '1'
    with tempfile.TemporaryFile(mode='w+') as log:
        executable = args.executable.resolve() if args.executable else repo / '_build/default/examples/gallery/main.exe'
        child = subprocess.Popen([str(executable),
                                  *(['--trace-canvas'] if args.trace_canvas else []),
                                  *(['--trace-motion'] if args.trace_motion else []),
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
            if args.section == 'empty':
                exercise_empty(mac, args.images)
            if args.section == 'groups':
                exercise_groups(mac, args.images)
            if args.section == 'labels':
                exercise_labels(mac, args.images)
            if args.section == 'badges':
                exercise_badges(mac, args.images)
            if args.section == 'status-regions':
                exercise_status_regions(mac, args.images)
            if args.section in ('all', 'styles'):
                exercise_styles(mac, args.images)
            if args.section in ('all', 'pickers'):
                exercise_pickers(mac, args.images)
            if args.section in ('all', 'overlays'):
                exercise_overlays(mac, args.images)
            if args.section in ('all', 'navigation'):
                exercise_navigation(mac, args.images)
            if args.section in ('all', 'feedback'):
                exercise_feedback(mac, args.images)
            if args.section in ('all', 'journeys'):
                exercise_journeys(mac, args.images)
            if args.section in ('all', 'collections'):
                exercise_collections(mac, args.images)
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
            mac.close(TITLE)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Gallery exited unsuccessfully')
        except Exception:
            if mac and child.poll() is None:
                try:
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
    print(f'GPUIO_GALLERY_AX_OK: section={args.section}, native actions, state semantics, focus and shutdown')


if __name__ == '__main__':
    main()
