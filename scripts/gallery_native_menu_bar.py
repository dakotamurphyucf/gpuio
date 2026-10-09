"""Actual AppKit menu artwork and command routing from the public OCaml gallery."""
import json
import math
import subprocess
import tempfile
import time
from pathlib import Path

from test_gallery import TITLE, GalleryMouse, activate, element_rect
from window_pixels import read_png


def bar_item(mac, label):
    bar = mac.attr(mac.app, 'AXMenuBar')
    if not bar:
        return None
    children = mac.children(bar)
    try:
        return next((mac.retain(item) for item in children
                     if mac.text(item, 'AXTitle') == label), None)
    finally:
        for item in children:
            mac.release(item)
        mac.release(bar)


def open_workspace(mac):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        item = bar_item(mac, 'Workspace')
        if item:
            break
        time.sleep(.03)
    else:
        raise AssertionError('The active gallery window has no Workspace menu')
    try:
        bounds = element_rect(mac, item)
        print('BAR_ITEM_BOUNDS', bounds, flush=True)
        mouse = GalleryMouse(mac)
        point = (bounds[0]+bounds[2]/2, bounds[1]+bounds[3]/2)
        mouse.check_owner(point)
        mouse.send(5, point)
        time.sleep(.15)
        visible = mac.children(item)
        try:
            already_open = bool(visible and element_rect(mac, visible[0])[2] > 0)
        finally:
            for child in visible:
                mac.release(child)
        if not already_open:
            mouse.send(1, point)
            time.sleep(.04)
            mouse.send(2, point)
            time.sleep(.2)
        children = mac.children(item)
        try:
            assert len(children) == 1, 'Workspace must own one native menu'
            return mac.retain(children[0])
        finally:
            for child in children:
                mac.release(child)
    finally:
        mac.release(item)


def capture(mac, menu, path, *, minimum_contrast=60):
    bounds = element_rect(mac, menu)
    print('NATIVE_MENU_BOUNDS', bounds, flush=True)
    rows = mac.children(menu)
    try:
        names = [mac.text(row, 'AXTitle') for row in rows]
        rects = [element_rect(mac, row) for row in rows]
    finally:
        for row in rows:
            mac.release(row)
    GalleryMouse(mac).check_owner((bounds[0]+bounds[2]/2, bounds[1]+12))
    subprocess.run(['screencapture', '-x', '-R'+','.join(str(round(n)) for n in bounds),
                    str(path)], check=True, timeout=10)
    pixels = read_png(mac, path)
    sx, sy = pixels.width/bounds[2], pixels.height/bounds[3]
    scores = []
    for row in rects:
        if row[3] < 20:  # Separator.
            scores.append(None)
            continue
        top = row[1]-bounds[1]
        background = pixels.rgb(10*sx, (top+11)*sy)
        matched = total = 0
        def distance(x, y, a, b):
            dx, dy = b[0]-a[0], b[1]-a[1]
            t = max(0, min(1, ((x-a[0])*dx+(y-a[1])*dy)/(dx*dx+dy*dy)))
            return math.hypot(x-a[0]-t*dx, y-a[1]-t*dy)
        for iy in range(round(3*sy), round(19*sy)):
            for ix in range(round(14*sx), round(30*sx)):
                x, y = (ix+.5)/sx, (iy+.5)/sy
                expected = min(distance(x,y,(17,11),(20,14)),
                               distance(x,y,(20,14),(27,7))) <= 1
                actual = max(abs(a-b) for a,b in zip(pixels.rgb(ix,top*sy+iy),background)) > minimum_contrast
                matched += actual == expected
                total += 1
        scores.append(matched/total)
    return {'names': names, 'bounds': bounds, 'rows': rects,
            'pixels': [pixels.width, pixels.height], 'minimum_contrast': minimum_contrast,
            'checkmark_match': scores}


def exercise(mac, images):
    from mac_input_source import foreground_keys
    foreground_keys(mac)
    temporary = tempfile.TemporaryDirectory(prefix='gpuio-bar-icons-')
    output = images or Path(temporary.name)
    evidence = {}
    mac.press(TITLE, 'Commands & feedback')
    stages = ['Ready to begin', 'Finding the right pieces',
              'Halfway there', 'Everything is in place']
    initial_stage = None
    deadline = time.monotonic()+10
    while initial_stage is None and time.monotonic()<deadline:
        for index, label in enumerate(stages):
            node = mac.find(TITLE, label, 'AXStaticText', deadline=deadline)
            if node:
                mac.release(node)
                initial_stage = index
                break
        if initial_stage is None:
            time.sleep(.03)
    assert initial_stage is not None, 'Missing public preview stage'
    next_stage = stages[(initial_stage+1) % len(stages)]
    mac.set(mac.app, 'AXFrontmost', mac.true)
    window = mac.window(TITLE)
    try:
        mac.perform(window, 'AXRaise')
    finally:
        mac.release(window)
    def snapshot(name, present, nested=False):
        menu = open_workspace(mac)
        child_menu = None
        try:
            time.sleep(.15)
            if nested:
                rows = mac.children(menu)
                try:
                    bounds = element_rect(mac, rows[-1])
                    point = (bounds[0]+bounds[2]-15,bounds[1]+bounds[3]/2)
                    mouse = GalleryMouse(mac)
                    mouse.check_owner(point)
                    mouse.send(5, point)
                    deadline = time.monotonic()+5
                    while time.monotonic()<deadline:
                        children = mac.children(rows[-1])
                        try:
                            if children and element_rect(mac, children[0])[2] > 0:
                                child_menu = mac.retain(children[0])
                                break
                        finally:
                            for child in children:
                                mac.release(child)
                        time.sleep(.03)
                    assert child_menu, 'Native submenu did not open'
                finally:
                    for row in rows:
                        mac.release(row)
            time.sleep(.3)  # AX bounds precede the submenu opening animation.
            result = capture(mac, child_menu or menu, output/f'native-bar-{name}.png')
            print('GALLERY_NATIVE_BAR_SNAPSHOT', name, json.dumps(result), flush=True)
            icon_rows = [0] if nested else [0,1,3]
            assert all((result['checkmark_match'][i] > .92 if present else
                        result['checkmark_match'][i] < .90) for i in icon_rows), (name, result)
            evidence[name] = result
        finally:
            if child_menu:
                mac.release(child_menu)
                mac.key(53)
            mac.release(menu)
            mac.key(53)
            time.sleep(.2)

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    try:
        time.sleep(.6)  # Allow the tiny asynchronous SVG decode before tracking.
        snapshot('ready', True)
        snapshot('nested', True, nested=True)
        toggle('Detailed menu items')
        snapshot('cleared', False)
        toggle('Detailed menu items')
        snapshot('restored', True)
        # Actual pointer selection must reach the OCaml Bonsai reducer.
        menu = open_workspace(mac)
        rows = mac.children(menu)
        try:
            bounds = element_rect(mac, rows[0])
            point = (bounds[0]+bounds[2]/2,bounds[1]+bounds[3]/2)
            mouse = GalleryMouse(mac)
            mouse.check_owner(point)
            mouse.send(5, point)
            mouse.send(1, point)
            mouse.send(2, point)
        finally:
            for row in rows:
                mac.release(row)
            mac.release(menu)
        mac.wait_text(TITLE, next_stage)
        # Restore the model so this section composes with the full gallery run.
        for offset in (2, 3, 4):
            mac.press(TITLE, 'Advance preview')
            mac.wait_text(TITLE, stages[(initial_stage+offset) % len(stages)])
        toggle('Enable preview command')
        from test_gallery import expect_enabled
        expect_enabled(mac, 'Advance preview', False)
        menu = open_workspace(mac)
        rows = mac.children(menu)
        try:
            from test_native_popup_macos import enabled
            assert not enabled(mac, rows[0]), 'Disabled command remained enabled'
            assert enabled(mac, rows[1]), 'Independent command was disabled'
        finally:
            for row in rows:
                mac.release(row)
            mac.release(menu)
            mac.key(53)
        toggle('Enable preview command')
        expect_enabled(mac, 'Advance preview', True)
        # Another window without a bar must not inherit this window's commands.
        def window_titles():
            windows = mac.children(mac.app, 'AXWindows')
            try:
                return {mac.text(window, 'AXTitle') for window in windows}
            finally:
                for window in windows:
                    mac.release(window)
        original_windows = window_titles()
        mac.press(TITLE, 'New window')
        deadline = time.monotonic()+10
        added = set()
        while not added and time.monotonic()<deadline:
            added = window_titles()-original_windows
            if not added:
                time.sleep(.03)
        assert len(added) == 1, ('Expected one new gallery window', added)
        secondary_title = added.pop()
        evidence['window_ownership'] = {'created': secondary_title}
        print('GALLERY_NATIVE_BAR_SECONDARY', secondary_title, flush=True)
        mac.wait_text(secondary_title, 'A little context goes a long way')
        time.sleep(.2)
        stale = bar_item(mac, 'Workspace')
        if stale:
            mac.release(stale)
            raise AssertionError('Second window inherited the first menu bar')
        mac.close(secondary_title)
        window = mac.window(TITLE)
        try:
            mac.perform(window, 'AXRaise')
            mac.set(window, 'AXMain', mac.true)
        finally:
            mac.release(window)
        snapshot('surviving-window', True)
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'A little context goes a long way')
        stale = bar_item(mac, 'Workspace')
        if stale:
            mac.release(stale)
            raise AssertionError('Unmounted page retained its bar')
        mac.press(TITLE, 'Commands & feedback')
        time.sleep(.6)
        snapshot('remounted', True)
        (output/'native-bar-report.json').write_text(json.dumps(evidence,indent=2)+'\n')
        print('GALLERY_NATIVE_BAR_OK: rendered/nested/cleared/restored artwork, pointer command, disabled state, window ownership and remount', flush=True)
    finally:
        temporary.cleanup()
