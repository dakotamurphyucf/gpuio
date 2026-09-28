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
    disabled = mac.wait_find(TITLE, 'Advance preview', 'AXButton')
    value = mac.attr(disabled, 'AXEnabled')
    get = mac.cf.CFBooleanGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p]
    try:
        if not value or get(value):
            raise RuntimeError('Disabled command still exposed as enabled')
    finally:
        if value:
            mac.release(value)
        mac.release(disabled)
    activate(mac, mac.wait_find(TITLE, 'Enable preview command', 'AXCheckBox'))
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--images', type=Path)
    parser.add_argument('--section', choices=['all', 'core', 'pickers', 'overlays', 'navigation', 'feedback'], default='all')
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
