#!/usr/bin/env python3
"""Exercise the public gallery's native navigation, editing and window isolation."""
import argparse
import ctypes as C
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
    field = mac.wait_find(TITLE, 'Document title', 'AXTextField')
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        mac.set(field, 'AXFocused', mac.true)
        mac.key(0, flags=1 << 20)  # Command+A
        mac.key(0)  # a
    finally:
        mac.release(field)
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


def expect_enabled(mac, label, expected):
    get = mac.cf.CFBooleanGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        node = mac.wait_find(TITLE, label, 'AXButton')
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


def exercise_documents(mac, images):
    mac.press(TITLE, 'Markdown & code')
    mac.wait_text(TITLE, 'Markdown preview')
    mac.wait_text(TITLE, 'A place for ideas')
    mac.wait_text(TITLE, '世界 · 👨‍👩‍👧‍👦')
    mac.wait_text(TITLE, 'Explore a direction')
    mac.wait_text(TITLE, 'Read the design notes')
    roles = tree_counts(mac, mac.wait_find(TITLE, 'Markdown preview', 'AXGroup'))
    assert roles.get('AXHeading') == 1 and roles.get('AXList') == 1, roles
    assert roles.get('AXLink') == 2, roles
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
        press = mac.string('AXPress')
        try:
            # An OS-retained reference may report success for an asynchronous
            # action; either way it must not navigate the collapsed document.
            mac.action(stale_link, press)
        finally:
            mac.release(press)
        time.sleep(.1)
        mac.wait_text(TITLE, 'Link requested: gpuio-preview:unicode')
        mac.press(TITLE, 'Expand')
        mac.release(mac.wait_find(TITLE, 'Read the design notes', 'AXLink'))
    finally:
        mac.release(stale_link)
    mac.press(TITLE, 'Append a finding')
    mac.wait_text(TITLE, 'Appended findings: 1 / 6')
    mac.release(mac.wait_find(TITLE, 'Copy code', 'AXButton'))
    mac.wait_text(TITLE, 'Finding 1')
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
    mac.press(TITLE, 'Collapse')
    mac.release(mac.wait_find(TITLE, 'Expand', 'AXButton'))
    wait_absent(mac, 'Diff preview', 'AXTextArea')
    mac.press(TITLE, 'Expand')
    mac.release(mac.wait_find(TITLE, 'Collapse', 'AXButton'))
    mac.release(mac.wait_find(TITLE, 'Diff preview', 'AXTextArea'))
    mac.press(TITLE, 'Markdown')
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
        wait_absent(mac, 'Copy code', 'AXButton')
        mac.press(TITLE, 'Append a finding')
        mac.wait_text(TITLE, 'Appended findings: 1 / 6')
        mac.release(mac.wait_find(TITLE, 'Copy code', 'AXButton'))
    mac.press(TITLE, 'Presentation')
    mac.wait_text(TITLE, 'A little context goes a long way')


def wait_absent(mac, label, role):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        node = mac.find(TITLE, label, role)
        if not node:
            return
        mac.release(node)
        time.sleep(.03)
    raise RuntimeError(f'Removed content is still accessible: {label}')


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
    parser.add_argument('--section', choices=['all', 'core', 'pickers', 'overlays', 'navigation', 'feedback', 'journeys', 'collections', 'documents', 'runtime'], default='all')
    args = parser.parse_args()
    if args.images:
        args.images.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/gallery/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            if args.section in ('all', 'core'):
                exercise(mac, args.images)
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
            if args.section in ('all', 'documents'):
                exercise_documents(mac, args.images)
            if args.section in ('all', 'runtime'):
                exercise_runtime(mac, args.images)
            mac.close(TITLE)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Gallery exited unsuccessfully')
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
            print(log.read(), end='')
    print(f'GPUIO_GALLERY_AX_OK: section={args.section}, native actions, state semantics, focus and shutdown')


if __name__ == '__main__':
    main()
