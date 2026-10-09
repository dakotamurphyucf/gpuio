"""Installed multi-month calendar: independent grid oracle and native range keys."""
import ctypes as C
from datetime import date, timedelta
import json
import time


def exercise(mac, images=None):
    from test_gallery import (
        TITLE, activate, element_rect, expect_focus, raise_gallery,
        reveal_gallery_control, select_gallery_appearance, wait_absent, within,
    )
    from test_canvas import screenshot

    mac.press(TITLE, 'Presentation')
    mac.press(TITLE, 'Dates & colors')
    group = 'Preview date range'
    start, end = 'September 30, 2026', 'October 1, 2026'
    selection_text = '2026-09-30 → 2026-10-01'
    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    samples = []

    def grid(first, count):
        dates = set()
        for offset in range(count):
            index = first.year * 12 + first.month - 1 + offset
            beginning = date(index // 12, index % 12 + 1, 1)
            monday = beginning - timedelta(days=beginning.weekday())
            dates.update(monday + timedelta(days=d) for d in range(42))
        return sorted(dates)

    def label(day):
        suffix = ', Today' if day == date(2026, 9, 14) else ''
        return f'{day:%B} {day.day}, {day.year}{suffix}'

    def cells():
        root = mac.wait_find(TITLE, group, 'AXGroup')
        labels = []

        def visit(node):
            values, children = mac.node_values(node)
            try:
                if values[0] == 'AXCheckBox':
                    labels.append(values[1])
                for child in children:
                    visit(child)
            finally:
                for child in children:
                    mac.release(child)
        try:
            visit(root)
        finally:
            mac.release(root)
        return labels

    def checked(name, expected):
        node = within(mac, group, name, 'AXCheckBox')
        raw = mac.attr(node, 'AXValue')
        try:
            assert raw and bool(boolean(raw)) == expected, (name, expected)
        finally:
            if raw:
                mac.release(raw)
            mac.release(node)

    def viewport(first, count, case, *, retained_day_panes=True):
        dates = grid(first, count)
        mac.wait_text(TITLE, f'Event content for {dates[0]} – {dates[-1]} · {len(dates)} grid dates')
        expected = {label(day) for day in dates}
        deadline = time.monotonic() + 10
        while True:
            actual = cells()
            if set(actual) == expected and len(actual) == len(expected):
                break
            if time.monotonic() >= deadline:
                raise AssertionError((case, 'wrong/duplicate native date targets',
                                      sorted(expected - set(actual)), sorted(set(actual) - expected),
                                      len(actual), len(expected)))
            time.sleep(.03)
        current = mac.wait_find(TITLE, group, 'AXGroup')
        try:
            assert equal(owner, current), 'Month count replaced the native calendar owner'
        finally:
            mac.release(current)
        mac.wait_text(TITLE, selection_text)
        bounds = []
        for name, retained in ((start, first_cell), (end, last_cell)):
            node = within(mac, group, name, 'AXCheckBox')
            try:
                # A date moving from its own September pane into October's
                # padding is a new structural target. The calendar owner and
                # logical selection must survive; targets within retained panes
                # must keep identity. Month/year mode retires the day surface.
                if retained_day_panes and (name == end or first == date(2026, 9, 1)):
                    assert equal(retained, node), ('Retained pane replaced date target', name)
                bounds.append(element_rect(mac, node))
            finally:
                mac.release(node)
            checked(name, True)
        assert bounds[0] != bounds[1], 'Distinct dates share one target rectangle'
        samples.append({'case': case, 'first_month': str(first), 'months': count,
                        'grid_first': str(dates[0]), 'grid_last': str(dates[-1]),
                        'unique_targets': len(actual), 'selected_bounds': bounds})
        print('GALLERY_CALENDAR_VIEWPORT_CASE', case, len(actual), 'PASS', flush=True)

    mac.press(TITLE, '2 months')
    reveal_gallery_control(mac, start, 'AXCheckBox')
    raise_gallery(mac)
    owner = mac.wait_find(TITLE, group, 'AXGroup')
    first_cell = within(mac, group, start, 'AXCheckBox')
    last_cell = within(mac, group, end, 'AXCheckBox')
    try:
        mac.set(first_cell, 'AXFocused', mac.true)
        expect_focus(mac, start, 'AXCheckBox')
        mac.key(36)
        mac.wait_text(TITLE, '2026-09-30 → choose an end date')
        mac.key(124)
        expect_focus(mac, end, 'AXCheckBox')
        mac.key(36)
        mac.wait_text(TITLE, selection_text)
        checked(start, True)
        checked(end, True)
        checked('September 29, 2026', False)
        checked('October 2, 2026', False)

        # Cursor moves into October while the two displayed panes stay Sep–Oct.
        viewport(date(2026, 9, 1), 2, 'initial-cross-month-range')
        for theme in ('Light', 'Dark'):
            select_gallery_appearance(mac, theme)
            for count in (3, 12, 1, 2):
                mac.press(TITLE, f'{count} month' + ('s' if count != 1 else ''))
                first = date(2026, 9, 1) if theme == 'Light' and count in (3, 12) else date(2026, 10, 1)
                viewport(first, count, f'{theme}-{count}-months')
                if images:
                    # A 12-pane layout is taller than this window. Capture its
                    # beginning only; the independent target check covers all panes.
                    reveal_gallery_control(mac, end, 'AXCheckBox')
                    screenshot(mac, images / f'gallery-calendar-{theme}-{count}.png', title=TITLE)

        # Month/year selection publishes an empty day-grid prefetch set, then a
        # new bounded set on returning to Days. Navigation does not select dates.
        activate(mac, within(mac, group, 'Choose month', 'AXButton'))
        mac.wait_text(TITLE, 'Choose a month or year to load day content')
        assert cells() == []
        activate(mac, within(mac, group, 'October', 'AXButton'))
        viewport(date(2026, 10, 1), 2, 'month-mode-return', retained_day_panes=False)
        activate(mac, within(mac, group, 'Choose year', 'AXButton'))
        mac.wait_text(TITLE, 'Choose a month or year to load day content')
        assert cells() == []
        activate(mac, within(mac, group, '2026', 'AXButton'))
        activate(mac, within(mac, group, 'October', 'AXButton'))
        viewport(date(2026, 10, 1), 2, 'year-mode-return', retained_day_panes=False)
        mac.press(TITLE, 'Presentation')
        wait_absent(mac, group, 'AXGroup')
        mac.press(TITLE, 'Dates & colors')
        current = mac.wait_find(TITLE, group, 'AXGroup')
        try:
            assert not equal(owner, current), 'Retired calendar owner reused'
        finally:
            mac.release(current)
        mac.wait_text(TITLE, 'No date selected')
        print('GALLERY_CALENDAR_VIEWPORT_OK: native cross-month range keys, '
              '1/2/3/12 panes, independent unique grid targets, retained selection/owners, '
              'Light/Dark, month/year navigation and remount', flush=True)
    finally:
        mac.release(last_cell)
        mac.release(first_cell)
        mac.release(owner)
        if images:
            (images / 'calendar-viewport-samples.json').write_text(json.dumps(samples, indent=2) + '\n')
