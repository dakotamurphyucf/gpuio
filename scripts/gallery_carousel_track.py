"""Public measured carousel: real native navigation and retained editor state."""
import ctypes as C
import json
import time


def exercise_automatic(mac, images=None):
    """Observe native four-second deadlines under real window/input policies."""
    from test_gallery import (
        TITLE, SECOND, GalleryMouse, activate, element_rect, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent,
    )

    label = 'Measured idea cards'
    report = {'complete': False, 'holds': [], 'resumes': [],
              'scope': 'sampled real deadline behavior; not exact frame timing or idle resource measurement'}
    caption = None
    mouse = GalleryMouse(mac)

    def toggle(name):
        activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))

    def caption_for(name):
        return mac.wait_find(TITLE, 'Selected card: ' + name, 'AXStaticText')

    def read():
        return mac.text(caption, 'AXTitle')

    def hold(name, reason, seconds=5.2):
        started = time.monotonic()
        count = 0
        while time.monotonic() - started < seconds:
            assert read() == 'Selected card: ' + name, (reason, read())
            count += 1
            time.sleep(.05)
        report['holds'].append(dict(reason=reason, selected=name,
                                    seconds=time.monotonic() - started, samples=count))
        print('CAROUSEL_AUTO_HOLD', reason, name, flush=True)

    def park():
        window = mac.window(TITLE)
        try:
            x, y, w, _ = element_rect(mac, window)
        finally:
            mac.release(window)
        point = (x + w / 2, y + 70)
        mouse.check_owner(point)
        mouse.send(5, point)

    def outside_focus():
        focus_gallery_control(mac, 'Dark', 'AXButton')

    def resume(name, expected, reason, *, before=None):
        started = time.monotonic()
        if before:
            before()
        outside_focus()
        park()
        observations = []
        while time.monotonic() - started < 8:
            actual = read()
            elapsed = time.monotonic() - started
            observations.append({'seconds': elapsed, 'selected': actual})
            if actual != 'Selected card: ' + name:
                assert actual == 'Selected card: ' + expected, (reason, actual)
                assert elapsed >= 4, ('Resume did not wait a fresh four-second interval', reason, elapsed)
                report['resumes'].append(dict(reason=reason, seconds=elapsed,
                                              selected=expected, observations=observations))
                # Stop the next interval before inspecting more state.
                focus_gallery_control(mac, label, 'AXGroup')
                hold(expected, reason + '-no-burst', seconds=.4)
                print('CAROUSEL_AUTO_RESUME', reason, expected, elapsed, flush=True)
                return
            time.sleep(.05)
        raise AssertionError(('Automatic advancement did not resume', reason, read()))

    try:
        mac.press(TITLE, 'Motion & rhythm')
        mac.press(TITLE, 'Use full motion')
        mac.wait_text(TITLE, 'Motion preference: Full')
        mac.press(TITLE, 'Carousels & journeys')
        select_gallery_appearance(mac, 'Dark')
        toggle('Compact viewport')
        toggle('Loop cards')
        reveal_gallery_control(mac, label, 'AXGroup')
        focus_gallery_control(mac, label, 'AXGroup')
        toggle('Auto-advance cards')
        focus_gallery_control(mac, label, 'AXGroup')
        park()
        caption = caption_for('Capture')
        hold('Capture', 'viewport-focus')
        resume('Capture', 'Explore', 'focus-release')

        # Hover is independent of focus: the header has focus, pointer is in track.
        viewport = mac.wait_find(TITLE, label, 'AXGroup')
        try:
            x, y, w, h = element_rect(mac, viewport)
        finally:
            mac.release(viewport)
        point = (x + w / 2, y + h - 8)
        mouse.check_owner(point)
        mouse.send(5, point)
        outside_focus()
        hold('Explore', 'track-hover')
        resume('Explore', 'Refine', 'hover-release')

        toggle('Disable track navigation')
        outside_focus()
        park()
        hold('Refine', 'disabled')
        resume('Refine', 'Publish', 'reenabled',
               before=lambda: toggle('Disable track navigation'))

        # A second window makes the original inactive without leaving the app.
        outside_focus()
        park()
        mac.press(TITLE, 'New window')
        mac.wait_text(SECOND, 'A little context goes a long way')
        hold('Publish', 'inactive-original-window')
        mac.close(SECOND)
        focus_gallery_control(mac, label, 'AXGroup')
        resume('Publish', 'Capture', 'reactivated')

        # Application motion policy, never a change to macOS preferences.
        mac.release(caption)
        caption = None
        mac.press(TITLE, 'Motion & rhythm')
        mac.press(TITLE, 'Use reduced motion')
        mac.wait_text(TITLE, 'Motion preference: Reduced')
        mac.press(TITLE, 'Carousels & journeys')
        reveal_gallery_control(mac, label, 'AXGroup')
        outside_focus()
        park()
        caption = caption_for('Capture')
        hold('Capture', 'reduced-motion')
        mac.release(caption)
        caption = None
        mac.press(TITLE, 'Motion & rhythm')
        mac.press(TITLE, 'Use full motion')
        mac.press(TITLE, 'Carousels & journeys')
        reveal_gallery_control(mac, label, 'AXGroup')
        focus_gallery_control(mac, label, 'AXGroup')
        park()
        caption = caption_for('Capture')
        resume('Capture', 'Explore', 'full-motion-restored')

        # Leave with an eligible timer; its retired callback must not mutate Bonsai.
        outside_focus()
        park()
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, label, 'AXGroup')
        time.sleep(5.2)
        mac.press(TITLE, 'Carousels & journeys')
        mac.wait_text(TITLE, 'Selected card: Explore')
        focus_gallery_control(mac, label, 'AXGroup')
        toggle('Auto-advance cards')
        mac.press(TITLE, 'Motion & rhythm')
        mac.press(TITLE, 'Use system motion')
        mac.wait_text(TITLE, 'Motion preference: System')
        report['complete'] = True
        print('GALLERY_CAROUSEL_AUTO_OK', len(report['resumes']), 'fresh resumes; pause policies and retired timer', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        if caption:
            mac.release(caption)
        if images:
            (images / 'carousel-automatic-report.json').write_text(json.dumps(report, indent=2) + '\n')


def exercise(mac, images=None):
    from mac_input_source import Sources, foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_field,
        focus_gallery_control, reveal_gallery_control, select_gallery_appearance,
        wait_absent,
    )

    label = 'Measured idea cards'
    report = {'complete': False, 'cases': [],
              'scope': 'OS keyboard/pointer and native state/geometry; not VoiceOver or presentation timing'}
    original_post = mac.post_key
    sources = Sources(mac)
    try:
        source = sources.selected()
    finally:
        sources.close()
    assert source in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC'), source
    report['input_source'] = source
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]

    def selected(name):
        mac.wait_text(TITLE, 'Selected card: ' + name)

    def toggle(name, expected):
        activate(mac, mac.wait_find(TITLE, name, 'AXCheckBox'))
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            node = mac.wait_find(TITLE, name, 'AXCheckBox')
            value = mac.attr(node, 'AXValue')
            try:
                if value and bool(boolean(value)) == expected:
                    return
            finally:
                if value:
                    mac.release(value)
                mac.release(node)
            time.sleep(.025)
        raise AssertionError(('Carousel option did not settle', name, expected))

    def key(code, expected):
        mac.key(code)
        selected(expected)

    def viewport():
        bounds = reveal_gallery_control(mac, label, 'AXGroup')
        focus_gallery_control(mac, label, 'AXGroup')
        return bounds

    def rect(name, role):
        node = mac.wait_find(TITLE, name, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def disabled_controls():
        viewport_node = mac.wait_find(TITLE, label, 'AXGroup')
        group = mac.attr(viewport_node, 'AXParent')
        controls = []

        def visit(node):
            if (mac.text(node, 'AXRole') == 'AXButton'
                    and mac.text(node, 'AXTitle') in ('Previous', 'Next')):
                enabled = mac.attr(node, 'AXEnabled')
                try:
                    assert enabled and not boolean(enabled), 'Disabled track exposed an enabled control'
                    controls.append(mac.text(node, 'AXTitle'))
                finally:
                    if enabled:
                        mac.release(enabled)
            children = mac.children(node)
            try:
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        try:
            assert group
            visit(group)
            assert sorted(controls) == ['Next', 'Previous'], ('Wrong track control group', controls)
        finally:
            if group:
                mac.release(group)
            mac.release(viewport_node)

    try:
        mac.press(TITLE, 'Carousels & journeys')
        selected('Capture')
        toggle('Compact viewport', True)
        viewport()
        foreground_keys(mac)
        focus_gallery_control(mac, 'Card draft', 'AXTextField')
        mac.key(0, flags=1 << 20)
        mac.key(0)  # a, with verified US/ABC input source
        expect_field(mac, TITLE, 'Card draft', 'a')
        viewport()
        key(119, 'Publish')  # End
        wait_absent(mac, 'Card draft', 'AXTextField')
        key(115, 'Capture')  # Home
        expect_field(mac, TITLE, 'Card draft', 'a')

        for theme in ('Dark', 'Light'):
            select_gallery_appearance(mac, theme)
            for axis, forward, backward in (('horizontal', 124, 123), ('vertical', 125, 126)):
                if axis == 'vertical':
                    mac.press(TITLE, 'Use vertical track')
                bounds = viewport()
                key(115, 'Capture')
                key(forward, 'Explore')
                key(forward, 'Refine')
                key(backward, 'Explore')
                key(119, 'Publish')
                wait_absent(mac, 'Card draft', 'AXTextField')
                key(115, 'Capture')
                expect_field(mac, TITLE, 'Card draft', 'a')
                report['cases'].append({'theme': theme, 'axis': axis, 'viewport': bounds})
                if images:
                    screenshot(mac, images / f'carousel-{theme}-{axis}.png', title=TITLE)
                if axis == 'vertical':
                    mac.press(TITLE, 'Use horizontal track')

        viewport()
        key(115, 'Capture')
        # A partially visible neighboring card keeps its ordinary button.
        deadline = time.monotonic() + 3
        while True:
            vx, vy, vw, vh = rect(label, 'AXGroup')
            bx, by, bw, bh = rect('Select Explore', 'AXButton')
            left, top = max(vx, bx), max(vy, by)
            right, bottom = min(vx + vw, bx + bw), min(vy + vh, by + bh)
            if right - left > 8 and bottom - top > 8:
                break
            assert time.monotonic() < deadline, ('Neighbor button has no clickable intersection', (vx, vy, vw, vh), (bx, by, bw, bh))
            time.sleep(.025)
        assert right - left < bw, 'Fixture must exercise a partially clipped button'
        point = ((left + right) / 2, (top + bottom) / 2)
        mouse = GalleryMouse(mac)
        mouse.check_owner(point)
        for kind in (5, 1, 2):
            mouse.send(kind, point)
        selected('Explore')
        report['partial_neighbor_button'] = {'bounds': (bx, by, bw, bh), 'click': point}

        viewport()
        key(115, 'Capture')
        mac.press(TITLE, 'Reverse cards')
        selected('Capture')
        expect_field(mac, TITLE, 'Card draft', 'a')
        viewport()
        key(115, 'Publish')
        key(119, 'Capture')
        expect_field(mac, TITLE, 'Card draft', 'a')
        toggle('Compact viewport', False)
        selected('Capture')
        expect_field(mac, TITLE, 'Card draft', 'a')
        toggle('Compact viewport', True)
        toggle('Animate card movement', False)
        mac.press(TITLE, 'Reverse cards')
        viewport()
        key(115, 'Capture')
        toggle('Loop cards', True)
        viewport()
        key(123, 'Publish')
        key(124, 'Capture')
        toggle('Disable track navigation', True)
        selected('Capture')
        disabled_controls()
        toggle('Disable track navigation', False)
        viewport()
        key(124, 'Explore')
        key(115, 'Capture')
        expect_field(mac, TITLE, 'Card draft', 'a')
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, label, 'AXGroup')
        mac.press(TITLE, 'Carousels & journeys')
        selected('Capture')
        viewport()
        # This editor belongs to the page computation and is recreated on leave;
        # draft retention assertions above apply to mounted cards, not persistence.
        expect_field(mac, TITLE, 'Card draft', 'Make something worth sharing.')
        report['complete'] = True
        print('GALLERY_CAROUSEL_TRACK_OK', len(report['cases']), 'axis/theme cases; retained editing, partial input, reorder, loop and remount', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'carousel-track-report.json').write_text(json.dumps(report, indent=2) + '\n')
