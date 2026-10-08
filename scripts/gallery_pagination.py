"""Physical breadcrumb/pagination policies and bounded native chooser lifetimes."""
import ctypes as C
import json


def exercise(mac, images=None):
    from mac_input_source import Sources, foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, activate, exercise_pagination, expect_enabled, expect_field,
        expect_focus, expect_popup_expanded, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent,
    )

    report = {'complete': False, 'cases': []}
    sources = Sources(mac)
    try:
        report['input_source'] = sources.selected()
    finally:
        sources.close()
    assert report['input_source'] in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC')
    original_post = mac.post_key
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]

    def press(label):
        mac.press(TITLE, label)

    def page(current, total):
        mac.wait_text(TITLE, f'Preview page {current} of {total}')

    def open_gap(label):
        reveal_gallery_control(mac, label, 'AXButton')
        focus_gallery_control(mac, label, 'AXButton')
        mac.key(36)
        expect_focus(mac, 'Page number', 'AXTextField')
        expect_popup_expanded(mac, label, True)

    def type_page(keys, value):
        mac.key(0, flags=1 << 20)
        for key in keys:
            mac.key(key)
        expect_field(mac, TITLE, 'Page number', value)

    def press_pagination_setting(label):
        # The header also has Compact; Disclosure also has Disabled.
        # Pagination is the final card, below those controls in screen coordinates.
        candidates = []
        window = mac.window(TITLE)
        def visit(node):
            if mac.text(node, 'AXRole') == 'AXButton' and mac.text(node, 'AXTitle') == label:
                candidates.append(mac.retain(node))
            children = mac.children(node)
            try:
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        try:
            visit(window)
            from test_gallery import element_rect
            target = max(candidates, key=lambda n: element_rect(mac, n)[1])
            mac.perform(target, 'AXPress')
        finally:
            mac.release(window)
            for node in candidates:
                mac.release(node)

    def button_paint(path, theme):
        from test_gallery import element_rect, read_png
        pixels = read_png(mac, path)
        window = mac.window(TITLE)
        try:
            wx, wy, ww, wh = element_rect(mac, window)
        finally:
            mac.release(window)
        background, foreground = ((25, 33, 44), (234, 240, 247)) if theme == 'Dark' else ((255, 255, 255), (27, 41, 57))
        samples = []
        for label in ('3', 'Cancel', 'Go to page'):
            node = mac.wait_find(TITLE, label, 'AXButton')
            try:
                x, y, w, h = element_rect(mac, node)
            finally:
                mac.release(node)
            bg = fg = total = 0
            for dy in range(3, int(h)-3):
                for dx in range(3, int(w)-3):
                    rgb = pixels.rgb((x+dx-wx)*pixels.width/ww, (y+dy-wy)*pixels.height/wh)
                    bg += max(abs(a-b) for a, b in zip(rgb, background)) < 12
                    fg += max(abs(a-b) for a, b in zip(rgb, foreground)) < 30
                    total += 1
            assert total and bg / total > .45 and fg >= 5, (label, theme, bg, fg, total)
            samples.append({'label': label, 'surface_pixels': bg, 'text_pixels': fg, 'sampled': total})
        return samples

    def chooser_shortcuts():
        root = mac.wait_find(TITLE, 'Choose a hidden page')
        labels = []
        visited = 0
        def visit(node):
            nonlocal visited
            visited += 1
            assert visited <= 128, 'billion-page chooser exposed an unbounded control tree'
            title = mac.text(node, 'AXTitle') or ''
            if mac.text(node, 'AXRole') == 'AXButton' and title.isdecimal():
                labels.append(int(title))
            children = mac.children(node)
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
        assert sorted(labels) == [3, 4, 5, 500000001, 999999997, 999999998, 999999999], labels
        return {'nodes': visited, 'shortcuts': sorted(labels)}

    try:
        foreground_keys(mac)
        # Change density away from Navigation, whose pagination has its own
        # Compact button. This prevents selecting an unrelated matching title.
        press('Presentation')
        for current, following in (('Large', 'Compact'), ('Compact', 'Comfortable')):
            node = mac.find(TITLE, current, 'AXButton')
            if node:
                activate(mac, node)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for scale, following in (('Comfortable', 'Large'), ('Large', 'Compact'), ('Compact', 'Comfortable')):
                case = {'theme': theme, 'scale': scale, 'checks': [], 'complete': False}
                report['cases'].append(case)
                press('Navigation & layout')
                mac.wait_text(TITLE, 'Current location: preview')
                reveal_gallery_control(mac, 'Workspace', 'AXLink')
                focus_gallery_control(mac, 'Workspace', 'AXLink')
                mac.key(36)
                mac.wait_text(TITLE, 'Current location: workspace')
                wait_absent(mac, 'Workspace', 'AXLink')  # current member is passive
                press('Open Preview')
                press('Passive workspace label')
                wait_absent(mac, 'Workspace', 'AXLink')
                activate(mac, mac.wait_find(TITLE, 'Studio', 'AXLink'))
                mac.wait_text(TITLE, 'Current location: studio')
                press('Open Preview')
                press('Passive workspace label')
                case['checks'].append('breadcrumb keyboard route and passive/current members')

                exercise_pagination(mac)  # Cancel focus, native 42 draft, Enter vs Go.
                page(42, 120)
                case['checks'].append('independent gap anchors, cancel focus, typed explicit confirmation')
                gap = 'Pages 44 to 119'
                open_gap(gap)
                first_editor = mac.wait_find(TITLE, 'Page number', 'AXTextField')
                try:
                    type_page([23, 23], '55')
                    mac.key(53)  # The numeric field consumes Escape to restore its value.
                    expect_field(mac, TITLE, 'Page number', '44')
                    page(42, 120)
                    focus_gallery_control(mac, 'Cancel', 'AXButton')
                    mac.key(53)  # Outside the editor, Escape dismisses the popover.
                    wait_absent(mac, 'Page number', 'AXTextField')
                    expect_focus(mac, gap)
                    open_gap(gap)
                    second_editor = mac.wait_find(TITLE, 'Page number', 'AXTextField')
                    try:
                        assert not equal(first_editor, second_editor), 'new opening reused retired AX field'
                    finally:
                        mac.release(second_editor)
                    press('Shrink to 3')
                    wait_absent(mac, 'Page number', 'AXTextField')
                    page(3, 3)
                    expect_enabled(mac, 'Next', False)
                    expect_enabled(mac, 'Last', False)
                finally:
                    mac.release(first_editor)
                case['checks'].append('Escape focus restoration, fresh opening, shrink retires chooser')
                press('Empty')
                mac.wait_text(TITLE, 'No pages to display')
                expect_enabled(mac, 'Next', False)
                expect_enabled(mac, 'Previous', False)
                press('120 pages')
                page(1, 120)
                open_gap('Pages 3 to 119')
                press_pagination_setting('Disabled')
                wait_absent(mac, 'Page number', 'AXTextField')
                expect_enabled(mac, 'Next', False)
                expect_enabled(mac, 'Pages 3 to 119', False)
                press_pagination_setting('Disabled')
                open_gap('Pages 3 to 119')
                press_pagination_setting('Compact')
                wait_absent(mac, 'Page number', 'AXTextField')
                wait_absent(mac, 'Pages 3 to 119', 'AXButton')
                press('Next')
                page(2, 120)
                press('Previous')
                page(1, 120)
                press_pagination_setting('Compact')
                case['checks'].append('empty/grow, disabled cancellation and compact navigation')
                press('1 billion pages')
                page(1, 1000000000)
                huge_gap = 'Pages 3 to 999999999'
                open_gap(huge_gap)
                case['billion_chooser'] = chooser_shortcuts()
                type_page([18] + [29] * 9, '1000000000')
                mac.key(36)
                expect_field(mac, TITLE, 'Page number', '999999999')
                page(1, 1000000000)  # Enter normalizes; only Go navigates.
                if images:
                    path = images / f'pagination-{theme}-{scale}.png'
                    screenshot(mac, path, title=TITLE)
                    case['button_paint'] = button_paint(path, theme)
                press('Go to page')
                page(999999999, 1000000000)
                press('Next')
                page(1000000000, 1000000000)
                expect_enabled(mac, 'Next', False)
                case['checks'].append('billion-page bounded chooser, native clamp and final boundary')
                press('First')
                open_gap(huge_gap)
                press('Presentation')
                wait_absent(mac, 'Page number', 'AXTextField')
                press('Navigation & layout')
                page(1, 1000000000)
                expect_popup_expanded(mac, huge_gap, False)
                case['checks'].append('page departure retires chooser while Bonsai retains page model')
                # Reset through public actions for the next presentation case.
                press('120 pages')
                open_gap('Pages 3 to 119')
                type_page([22, 29], '60')
                press('Go to page')
                page(60, 120)
                case['complete'] = True
                print('GALLERY_PAGINATION_CASE', theme, scale, 'PASS', flush=True)
                press('Presentation')
                press(scale)
                mac.release(mac.wait_find(TITLE, following, 'AXButton'))
        report['complete'] = True
        print('GALLERY_PAGINATION_OK', len(report['cases']), 'cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'pagination-report.json').write_text(json.dumps(report, indent=2) + '\n')
