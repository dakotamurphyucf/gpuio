"""Real macOS public-gallery notification placement and keyboard lifecycle."""
import ctypes as C
import json
import time


ANCHORS = ('Bottom right', 'Bottom center', 'Bottom left', 'Left center',
           'Top left', 'Top center', 'Top right', 'Right center')
CARDS = ('Workspace saved', 'Research complete', 'A new idea is ready')


def exercise_policy(mac, images=None):
    """Pointer expansion and actual active-time expiry under both motion policies."""
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, focus_gallery_control,
        reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot

    report = {'complete': False, 'policies': [],
              'scope': 'real pointer/focus/timer behavior; not FPS, VoiceOver or idle-resource qualification'}
    mouse = GalleryMouse(mac)

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    def park():
        window = mac.window(TITLE)
        try:
            x, y, width, _ = element_rect(mac, window)
        finally:
            mac.release(window)
        point = (x + width / 2, y + 75)
        mouse.check_owner(point)
        mouse.send(5, point)

    def hover(label):
        node = mac.wait_find(TITLE, label, 'AXGroup')
        window = mac.window(TITLE)
        try:
            wx, wy, ww, wh = element_rect(mac, window)
            previous, stable = None, 0
            deadline = time.monotonic() + 3
            while True:
                bounds = element_rect(mac, node)
                x, y, width, height = bounds
                point = (x + width / 2, y + height / 2)
                contained = (width > 0 and height > 0 and wx <= x and wy <= y
                             and x + width <= wx + ww and y + height <= wy + wh)
                stable = stable + 1 if contained and previous == bounds else 0
                if stable >= 2:
                    break
                assert time.monotonic() < deadline, ('Toast did not settle inside window', bounds)
                previous = bounds
                time.sleep(.04)
        finally:
            mac.release(node)
            mac.release(window)
        mouse.check_owner(point)
        mouse.send(5, point)
        return point

    def wait_visible(label, seconds):
        started = time.monotonic()
        while time.monotonic() - started < seconds:
            node = mac.find(TITLE, label, 'AXGroup')
            assert node, ('Notification disappeared during paused lifetime', label)
            mac.release(node)
            time.sleep(.1)
        return time.monotonic() - started

    try:
        for preference in ('Full', 'Reduced'):
            mac.press(TITLE, 'Motion & rhythm')
            mac.press(TITLE, 'Use full motion' if preference == 'Full' else 'Use reduced motion')
            mac.wait_text(TITLE, 'Motion preference: ' + preference)
            mac.press(TITLE, 'Commands & feedback')
            reveal_gallery_control(mac, 'Layer notification cards', 'AXCheckBox')
            if preference == 'Full':
                toggle('Animate notifications')
            toggle('Layer notification cards')
            focus_gallery_control(mac, 'Show three sample notifications', 'AXButton')
            park()
            wait_absent(mac, CARDS[0], None)
            wait_absent(mac, CARDS[1], None)
            point = hover(CARDS[2])
            bounds = []
            for label in CARDS:
                node = mac.wait_find(TITLE, label, 'AXGroup')
                mac.release(node)
            time.sleep(.8)
            for label in CARDS:
                node = mac.wait_find(TITLE, label, 'AXGroup')
                try:
                    bounds.append(element_rect(mac, node))
                finally:
                    mac.release(node)
            ordered = sorted(bounds, key=lambda b: b[1])
            for before, after in zip(ordered, ordered[1:]):
                assert abs(after[1] - before[1] - before[3] - 14) < 1.5, bounds
            if images:
                screenshot(mac, images / f'notifications-hover-{preference.lower()}.png', title=TITLE)
            park()
            wait_absent(mac, CARDS[0], None)
            wait_absent(mac, CARDS[1], None)
            observation = {'preference': preference, 'hover_point': point,
                           'expanded_bounds': bounds, 'pointer_departure_collapsed': True}
            report['policies'].append(observation)

            # Switch to the real five-second notification. Its observer updates
            # the OCaml status; semantic disappearance alone is not expiry ack.
            toggle('Layer notification cards')
            focus_gallery_control(mac, 'Animate notifications', 'AXCheckBox')
            mac.press(TITLE, 'Save preview')
            mac.wait_text(TITLE, 'Notification visible')
            hover('Preview saved')
            observation['hover_pause_seconds'] = wait_visible('Preview saved', 5.6)
            park()
            resumed = time.monotonic()
            mac.wait_text(TITLE, 'No pending notification')
            wait_absent(mac, 'Preview saved', 'AXGroup')
            observation['expiry_after_hover_seconds'] = time.monotonic() - resumed

            mac.press(TITLE, 'Save preview')
            mac.wait_text(TITLE, 'Notification visible')
            park()
            focus_gallery_control(mac, 'Dismiss saved preview', 'AXButton')
            observation['focus_pause_seconds'] = wait_visible('Preview saved', 5.6)
            focus_gallery_control(mac, 'Animate notifications', 'AXCheckBox')
            resumed = time.monotonic()
            mac.wait_text(TITLE, 'No pending notification')
            wait_absent(mac, 'Preview saved', 'AXGroup')
            observation['expiry_after_focus_seconds'] = time.monotonic() - resumed

        mac.press(TITLE, 'Motion & rhythm')
        mac.press(TITLE, 'Use system motion')
        mac.wait_text(TITLE, 'Motion preference: System')
        report['complete'] = True
        print('GALLERY_NOTIFICATION_POLICY_OK', json.dumps(report), flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        if images:
            screenshot(mac, images / 'notification-policy-failure.png', title=TITLE)
        raise
    finally:
        if images:
            (images / 'notification-policy.json').write_text(json.dumps(report, indent=2) + '\n')


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, GalleryMouse, activate, element_rect, expect_focus,
        focus_gallery_control, reveal_gallery_control, wait_absent,
    )
    from test_canvas import screenshot

    report = {'complete': False, 'geometry': [],
              'scope': 'native AX geometry/input; not VoiceOver or frame-time acceptance'}
    original = None
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]

    def toggle(label):
        activate(mac, mac.wait_find(TITLE, label, 'AXCheckBox'))

    def rect(label, role=None):
        node = mac.wait_find(TITLE, label, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def park():
        window = mac.window(TITLE)
        try:
            x, y, width, height = element_rect(mac, window)
        finally:
            mac.release(window)
        mouse = GalleryMouse(mac)
        point = (x + width / 2, y + 75)
        mouse.check_owner(point)
        mouse.send(5, point)

    def expanded():
        focus_gallery_control(mac, 'Notifications', 'AXGroup')
        time.sleep(.8)  # Bounded settlement, not an animation-performance claim.
        bounds = [rect(label) for label in CARDS]
        ordered = sorted(bounds, key=lambda b: b[1])
        for before, after in zip(ordered, ordered[1:]):
            assert abs(after[1] - before[1] - before[3] - 14) < 1.5, (
                'Expanded cards overlap or have the wrong gap', bounds)
        assert all(abs(bound[2] - 360) < 1.5 for bound in bounds), bounds
        assert max(b[3] for b in bounds) > min(b[3] for b in bounds), (
            'Expected genuinely different measured card heights', bounds)
        node = mac.wait_find(TITLE, CARDS[2])
        try:
            assert equal(original, node), 'Placement/theme update replaced a toast owner'
        finally:
            mac.release(node)
        return (min(b[0] for b in bounds), min(b[1] for b in bounds),
                360, max(b[1] + b[3] for b in bounds) - min(b[1] for b in bounds))

    def near(actual, expected, context):
        assert abs(actual - expected) < 1.5, (context, actual, expected)

    def check_anchors(rows):
        br, bc, bl, lc, tl, tc, tr, rc = rows
        for value in (bl[0], lc[0]):
            near(value, tl[0], 'left edge')
        for value in (br[0], rc[0]):
            near(value, tr[0], 'right edge')
        near(tc[0], (tl[0] + tr[0]) / 2, 'top horizontal center')
        near(bc[0], tc[0], 'bottom horizontal center')
        for value in (tc[1], tr[1]):
            near(value, tl[1], 'top edge')
        for value in (bc[1], bl[1]):
            near(value, br[1], 'bottom edge')
        near(lc[1], (tl[1] + bl[1]) / 2, 'left vertical center')
        near(rc[1], lc[1], 'right vertical center')

    try:
        mac.press(TITLE, 'Motion & rhythm')
        mac.press(TITLE, 'Use full motion')
        mac.wait_text(TITLE, 'Motion preference: Full')
        mac.press(TITLE, 'Commands & feedback')
        reveal_gallery_control(mac, 'Layer notification cards', 'AXCheckBox')
        toggle('Layer notification cards')
        park()
        original = mac.wait_find(TITLE, CARDS[2])
        # Only the newest card is semantic while collapsed; older paint layers
        # must not masquerade as accessible interactive cards.
        focus_gallery_control(mac, 'Show three sample notifications', 'AXButton')
        wait_absent(mac, CARDS[0], None)
        wait_absent(mac, CARDS[1], None)
        report['collapsed_older_cards_hidden'] = True

        for palette in ('Light', 'Dark'):
            button = mac.find(TITLE, 'Dark' if palette == 'Light' else 'Light', 'AXButton')
            if button:
                activate(mac, button)
            mac.release(mac.wait_find(TITLE, palette, 'AXButton'))
            for animated in (False, True):
                if animated:
                    toggle('Animate notifications')
                rows = []
                for anchor in ANCHORS:
                    bounds = expanded()
                    report['geometry'].append({'palette': palette, 'motion': animated,
                                               'anchor': anchor, 'bounds': bounds})
                    rows.append(bounds)
                    mac.press(TITLE, 'Placement: ' + anchor)
                check_anchors(rows)
                if animated:
                    toggle('Reserve window margins')
                    inset = expanded()
                    near(inset[0], rows[0][0] - 24, 'right inset')
                    near(inset[1], rows[0][1] - 16, 'bottom inset')
                    report['geometry'].append({'palette': palette, 'motion': True,
                                               'anchor': 'Bottom right with margins', 'bounds': inset})
                    toggle('Reserve window margins')
                    expanded()
                    if images:
                        screenshot(mac, images / f'notifications-{palette.lower()}.png', title=TITLE)
                    toggle('Animate notifications')

        toggle('Animate notifications')
        expanded()
        # Group entry is a real focus stop; Tab reaches an actual close button
        # and Escape dismisses through native delivery to the Bonsai model.
        mac.key(48)
        expect_focus(mac, 'Dismiss notification', 'AXButton')
        oldest = mac.wait_find(TITLE, CARDS[0])
        # Layered paint is newest-at-anchor; keyboard traversal preserves source
        # order. Verify the focused button belongs to the oldest source card.
        # The application-level AXFocusedUIElement is the hosting NSWindow.
        # expect_focus above checks the leaf's AXFocused state directly.
        focused = mac.wait_find(TITLE, 'Dismiss notification', 'AXButton')
        try:
            found_owner = False
            for _ in range(12):
                if equal(focused, oldest):
                    found_owner = True
                    break
                parent = mac.attr(focused, 'AXParent')
                mac.release(focused)
                focused = parent
                if not focused:
                    break
            assert found_owner, 'Keyboard close button did not belong to the oldest source card'
        finally:
            if focused:
                mac.release(focused)
        try:
            mac.key(53)
            wait_absent(mac, CARDS[0], None)
            report['keyboard_dismissal'] = CARDS[0]
            mac.press(TITLE, 'Show three sample notifications')
            focus_gallery_control(mac, 'Notifications', 'AXGroup')
            restored = mac.wait_find(TITLE, CARDS[0])
            try:
                assert not equal(oldest, restored), 'Restoration reused a retired toast owner'
            finally:
                mac.release(restored)
        finally:
            mac.release(oldest)
        report['restored_fresh_owner'] = True
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, 'Notifications', 'AXGroup')
        for label in CARDS:
            wait_absent(mac, label, None)
        mac.press(TITLE, 'Commands & feedback')
        mac.release(mac.wait_find(TITLE, CARDS[2]))
        report['page_retirement_and_return'] = True
        report['complete'] = True
        print('GALLERY_NOTIFICATIONS_OK', json.dumps(report), flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        if images:
            screenshot(mac, images / 'notifications-failure.png', title=TITLE)
        raise
    finally:
        if original:
            mac.release(original)
        if images:
            (images / 'notifications.json').write_text(json.dumps(report, indent=2) + '\n')
