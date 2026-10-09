"""Real macOS resizing and retained children in the public flat split group."""

import json
import re
import time


def exercise(mac, images=None):
    from mac_input_source import Sources, foreground_keys
    from test_numeric import Numeric
    from test_gallery import (
        TITLE,
        GalleryMouse,
        activate,
        element_rect,
        expect_field,
        focus_gallery_control,
        reveal_gallery_control,
        select_gallery_appearance,
        wait_absent,
    )

    group = "Resizable workspace"
    first, second = "Resize Files and Draft", "Resize Draft and Inspector"
    editor = "Resizable workspace draft"
    mouse = GalleryMouse(mac)
    report = {
        "complete": False,
        "cases": [],
        "scope": "OS pointer/keyboard/AX and retained state; not VoiceOver or frame timing",
    }
    original_post = mac.post_key
    sources = Sources(mac)
    try:
        source = sources.selected()
    finally:
        sources.close()
    assert source in ("com.apple.keylayout.US", "com.apple.keylayout.ABC"), source
    report["input_source"] = source

    def rect(name, role):
        node = mac.wait_find(TITLE, name, role)
        try:
            return element_rect(mac, node)
        finally:
            mac.release(node)

    def value(name, attribute="AXValue"):
        node = mac.wait_find(TITLE, name, "AXSplitter")
        try:
            return Numeric.number(mac, node, attribute)
        finally:
            mac.release(node)

    def wait_value(name, expected):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            actual = value(name)
            if abs(actual - expected) < 0.6:
                return actual
            time.sleep(0.025)
        raise AssertionError((name, expected, actual))

    def draft_size():
        return value(second) - value(first)

    def wait_draft(expected):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            actual = draft_size()
            if abs(actual - expected) < 0.6:
                return actual
            time.sleep(0.025)
        raise AssertionError(("Draft extent", expected, actual))

    def toggle(name):
        activate(mac, mac.wait_find(TITLE, name, "AXCheckBox"))

    def observed_sizes():
        # The caption is updated only by the public on_resize callback, so AX
        # boundary movement alone cannot satisfy this asynchronous-delivery check.
        deadline = time.monotonic() + 5
        expected = {"files": value(first), "draft": draft_size()}
        actual = None
        while time.monotonic() < deadline:
            node = mac.find(TITLE, "Completed sizes:", "AXStaticText", contains=True)
            if node:
                try:
                    text = mac.text(node, "AXTitle")
                finally:
                    mac.release(node)
                actual = {
                    name: float(size)
                    for name, size in re.findall(r"(\w+) ([0-9.]+) px", text)
                }
                if all(
                    name in actual and abs(actual[name] - size) <= 0.6
                    for name, size in expected.items()
                ):
                    return actual
            time.sleep(0.025)
        raise AssertionError(
            ("Bonsai resize observation did not settle", expected, actual)
        )

    def drag(axis, cancel):
        reveal_gallery_control(mac, first, "AXSplitter")
        before = value(first)
        x, y, w, h = rect(first, "AXSplitter")
        start = (x + w / 2, y + h / 2)
        finish = (
            (start[0] + 30, start[1])
            if axis == "horizontal"
            else (start[0], start[1] + 30)
        )
        mouse.check_owner(start)
        mouse.check_owner(finish)
        try:
            mouse.send(5, start)
            mouse.send(1, start)
            time.sleep(0.05)
            for step in range(1, 11):
                mouse.send(
                    6, tuple(a + (b - a) * step / 10 for a, b in zip(start, finish))
                )
                time.sleep(0.02)
            preview = wait_value(first, before + 30)
            if cancel:
                mac.key(53)
                wait_value(first, before)
        finally:
            mouse.send(2, finish)
        expected = before if cancel else before + 30
        wait_value(first, expected)
        time.sleep(0.15)
        wait_value(first, expected)
        return dict(
            cancel=cancel,
            before=before,
            preview=preview,
            after=value(first),
            observed=observed_sizes(),
        )

    try:
        foreground_keys(mac)
        mac.press(TITLE, "Navigation & layout")
        reveal_gallery_control(mac, group, "AXGroup", scroll_in_left_gutter=True)
        reveal_gallery_control(mac, editor, "AXTextArea")
        focus_gallery_control(mac, editor, "AXTextArea")
        mac.key(0, flags=1 << 20)
        mac.key(0)
        expect_field(mac, TITLE, editor, "a", role="AXTextArea")
        for theme in ("Dark", "Light"):
            select_gallery_appearance(mac, theme)
            for axis in ("horizontal", "vertical"):
                if axis == "vertical":
                    mac.press(TITLE, "Vertical layout")
                mac.press(TITLE, "Reset sizes")
                reveal_gallery_control(
                    mac, group, "AXGroup", scroll_in_left_gutter=True
                )
                reveal_gallery_control(mac, first, "AXSplitter")
                case = dict(theme=theme, axis=axis)
                case["drag"] = [drag(axis, False), drag(axis, True)]
                focus_gallery_control(mac, first, "AXSplitter")
                before = value(first)
                mac.key(124 if axis == "horizontal" else 125)
                wait_value(first, before + 16)
                mac.key(123 if axis == "horizontal" else 126)
                wait_value(first, before)
                mac.key(115)
                wait_value(first, value(first, "AXMinValue"))
                mac.key(119)
                wait_value(first, value(first, "AXMaxValue"))
                mac.press(TITLE, "Reset sizes")
                reveal_gallery_control(mac, first, "AXSplitter")
                before = value(first)
                node = mac.wait_find(TITLE, first, "AXSplitter")
                try:
                    mac.perform(node, "AXIncrement")
                    wait_value(first, before + 16)
                    mac.perform(node, "AXDecrement")
                    wait_value(first, before)
                finally:
                    mac.release(node)
                case["keyboard_and_ax"] = True
                expect_field(mac, TITLE, editor, "a", role="AXTextArea")
                report["cases"].append(case)
                if axis == "vertical":
                    mac.press(TITLE, "Horizontal layout")
        mac.press(TITLE, "Reset sizes")
        reveal_gallery_control(mac, first, "AXSplitter")
        mac.press(TITLE, "Resize draft to 320 px")
        wait_draft(320)
        observed_sizes()
        focus_gallery_control(mac, first, "AXSplitter")
        mac.key(124)
        wait_draft(304)
        toggle("Custom divider grips")
        wait_draft(304)  # appearance/model renders must not replay the old request
        x, y, w, h = rect(first, "AXSplitter")
        assert abs(w - 16) < 1, ("Custom grip changed hit extent", w)
        report["request_then_keyboard_draft"] = draft_size()
        toggle("Constrain each panel to 200 px")
        wait_draft(200)
        mac.press(TITLE, "Resize draft to 320 px")
        wait_draft(200)
        observed_sizes()
        report["constrained_request_draft"] = draft_size()
        toggle("Constrain each panel to 200 px")
        mac.press(TITLE, "Reorder panels")
        mac.release(mac.wait_find(TITLE, "Resize Inspector and Draft", "AXSplitter"))
        expect_field(mac, TITLE, editor, "a", role="AXTextArea")
        mac.press(TITLE, "Hide inspector")
        wait_absent(mac, "Resize Inspector and Draft", "AXSplitter")
        expect_field(mac, TITLE, editor, "a", role="AXTextArea")
        mac.press(TITLE, "Show inspector")
        mac.press(TITLE, "Add outline")
        mac.release(mac.wait_find(TITLE, "Resize Outline and Inspector", "AXSplitter"))
        expect_field(mac, TITLE, editor, "a", role="AXTextArea")
        mac.press(TITLE, "Remove outline")
        wait_absent(mac, "Resize Outline and Inspector", "AXSplitter")
        mac.press(TITLE, "Reorder panels")
        mac.press(TITLE, "Reset sizes")
        expect_field(mac, TITLE, editor, "a", role="AXTextArea")
        report["structural_retention"] = True
        mac.press(TITLE, "Presentation")
        wait_absent(mac, editor, "AXTextArea")
        mac.press(TITLE, "Navigation & layout")
        reveal_gallery_control(mac, group, "AXGroup", scroll_in_left_gutter=True)
        expect_field(
            mac,
            TITLE,
            editor,
            "A retained draft. Reorder the panels, hide the inspector, or add an outline without losing your edits.",
            role="AXTextArea",
        )
        report["complete"] = True
        print(
            "GALLERY_SPLIT_GROUP_OK",
            len(report["cases"]),
            "axis/theme cases, native resize/cancel, serial requests, bounds and retained children",
            flush=True,
        )
    except BaseException as error:
        report["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        mac.post_key = original_post
        if images:
            (images / "split-group-report.json").write_text(
                json.dumps(report, indent=2) + "\n"
            )
