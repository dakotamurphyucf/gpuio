"""Public navigation history with real keys, editor retention and page retirement."""
import ctypes as C
import json


def exercise(mac, images=None):
    from mac_input_source import Sources, foreground_keys
    from test_canvas import screenshot
    from test_gallery import (
        TITLE, activate, expect_field, focus_gallery_control,
        reveal_gallery_control, select_gallery_appearance, wait_absent,
    )

    report = {'complete': False, 'cases': [],
              'scope': 'native keyboard/AX history and editor lifetimes; not motion frame timing or VoiceOver'}
    original_post = mac.post_key
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    initial = 'Return here and pick up where you left off.'
    sources = Sources(mac)
    try:
        report['input_source'] = sources.selected()
    finally:
        sources.close()
    assert report['input_source'] in ('com.apple.keylayout.US', 'com.apple.keylayout.ABC')

    def selected(chapter, back, forward):
        mac.wait_text(TITLE, 'Current journey: ' + chapter)
        mac.wait_text(TITLE, f'Journey history: {back} back · {forward} forward')

    def retention(expected):
        node = mac.wait_find(TITLE, 'Retain journey pages', 'AXCheckBox')
        value = mac.attr(node, 'AXValue')
        try:
            assert value
            actual = bool(boolean(value))
        finally:
            if value:
                mac.release(value)
            mac.release(node)
        if actual != expected:
            activate(mac, mac.wait_find(TITLE, 'Retain journey pages', 'AXCheckBox'))

    def enabled(label, expected):
        node = mac.wait_find(TITLE, label, 'AXButton')
        value = mac.attr(node, 'AXEnabled')
        try:
            assert value and bool(boolean(value)) == expected, (label, expected)
        finally:
            if value:
                mac.release(value)
            mac.release(node)

    def edit():
        reveal_gallery_control(mac, 'Journey note', 'AXTextField', scroll_in_left_gutter=True)
        focus_gallery_control(mac, 'Journey note', 'AXTextField')
        mac.key(0, flags=1 << 20)
        mac.key(0)
        expect_field(mac, TITLE, 'Journey note', 'a')

    try:
        foreground_keys(mac)
        mac.press(TITLE, 'Carousels & journeys')
        mode = 'Slide'
        for theme in ('Dark', 'Light'):
            select_gallery_appearance(mac, theme)
            for expected_mode in ('Slide', 'Fade', 'Immediate'):
                assert mode == expected_mode
                mac.press(TITLE, 'Reset journey')
                retention(True)
                selected('Imagine', 0, 2)
                enabled('Go back', False)
                enabled('Journey root', False)
                enabled('Replace with Share', False)
                expect_field(mac, TITLE, 'Journey note', initial)
                edit()
                reveal_gallery_control(mac, 'Continue journey', 'AXButton', scroll_in_left_gutter=True)
                focus_gallery_control(mac, 'Continue journey', 'AXButton')
                mac.key(49)  # Actual Space activation, not a Bonsai injection.
                selected('Shape', 1, 1)
                wait_absent(mac, 'Journey note', 'AXTextField')
                mac.press(TITLE, 'Continue journey')
                selected('Share', 2, 0)
                enabled('Continue journey', False)
                mac.press(TITLE, 'Go back')
                selected('Shape', 1, 1)
                mac.press(TITLE, 'Journey root')
                selected('Imagine', 0, 2)
                expect_field(mac, TITLE, 'Journey note', 'a')
                mac.press(TITLE, 'Visit next chapter')
                selected('Shape', 1, 0)  # New visit discards the old forward branch.
                mac.press(TITLE, 'Replace with Share')
                selected('Share', 1, 0)
                mac.press(TITLE, 'Visit next chapter')
                selected('Imagine', 2, 0)
                wait_absent(mac, 'Journey note', 'AXTextField')  # Only the root owns it.
                mac.press(TITLE, 'Journey root')
                selected('Imagine', 0, 2)
                expect_field(mac, TITLE, 'Journey note', 'a')
                retention(False)
                mac.press(TITLE, 'Continue journey')
                selected('Share', 1, 1)
                wait_absent(mac, 'Journey note', 'AXTextField')
                mac.press(TITLE, 'Go back')
                selected('Imagine', 0, 2)
                expect_field(mac, TITLE, 'Journey note', initial)
                if images and mode == 'Immediate':
                    reveal_gallery_control(mac, 'Current journey: Imagine', 'AXStaticText', scroll_in_left_gutter=True)
                    screenshot(mac, images / f'journey-{theme.lower()}.png', title=TITLE)
                report['cases'].append({'theme': theme, 'motion': mode, 'complete': True,
                                        'retained_draft': True, 'unmounted_draft_reset': True})
                mac.press(TITLE, 'Journey motion: ' + mode)
                mode = {'Slide': 'Fade', 'Fade': 'Immediate', 'Immediate': 'Slide'}[mode]
                mac.release(mac.wait_find(TITLE, 'Journey motion: ' + mode, 'AXButton'))
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, 'Idea journey', 'AXGroup')
        wait_absent(mac, 'Journey note', 'AXTextField')
        mac.press(TITLE, 'Carousels & journeys')
        selected('Imagine', 0, 2)
        expect_field(mac, TITLE, 'Journey note', initial)
        report['complete'] = True
        print('GALLERY_JOURNEY_OK', len(report['cases']), 'history/retention cases', flush=True)
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / 'journey-report.json').write_text(json.dumps(report, indent=2) + '\n')
