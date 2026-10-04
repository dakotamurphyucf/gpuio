"""Physical structural-table gallery checks; requires macOS AX permission.

Defining this walkthrough does not establish desktop or VoiceOver acceptance.
"""

import ctypes as C
import time

from test_canvas import screenshot

TITLE = "GPUIO · Component Studio 1"


def exercise(mac, images):
    mac.press(TITLE, "Lists, trees & tables")
    mac.press(TITLE, "Structural table")
    # Captions belong to the table subtree, which ordinary label searches skip.
    mac.release(mac.wait_find(TITLE, "Grouped headings, merged cells, and native actions.",
                              contains=True, search_files=True))
    table = mac.wait_find(TITLE, "Workspace review summary", "AXTable")
    retained = []

    def children(node, attribute="AXChildren"):
        result = mac.children(node, attribute)
        retained.extend(result)
        return result

    def descendants(node, role):
        result = []
        for child in children(node):
            if mac.text(child, "AXRole") == role:
                result.append(child)
            else:
                result.extend(descendants(child, role))
        return result

    def number(node, attribute):
        value = mac.attr(node, attribute)
        try:
            get = mac.cf.CFNumberGetValue
            get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
            result = C.c_longlong()
            assert value and get(value, 4, C.byref(result)), attribute
            return result.value
        finally:
            if value:
                mac.release(value)

    def index_range(node, attribute):
        class Range(C.Structure):
            _fields_ = [("location", C.c_long), ("length", C.c_long)]

        value, result = mac.attr(node, attribute), Range()
        try:
            get = mac.ax.AXValueGetValue
            get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
            if not value or not get(value, 4, C.byref(result)):
                value_type_id = mac.ax.AXValueGetTypeID
                value_type_id.restype, value_type_id.argtypes = C.c_ulong, []
                kind = mac.ax.AXValueGetType
                kind.restype, kind.argtypes = C.c_int, [C.c_void_p]
                actual_kind = (kind(value) if value and mac.type_id(value) == value_type_id()
                               else None)
                raise AssertionError((attribute, bool(value), actual_kind,
                                      mac.text(node, "AXRole"), label(node)))
            return result.location, result.length
        finally:
            if value:
                mac.release(value)

    def label(cell):
        return "".join(mac.text(node, "AXValue") or mac.text(node, "AXTitle") or ""
                       for node in descendants(cell, "AXStaticText"))

    equal = mac.cf.CFEqual
    equal.restype, equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
    try:
        assert number(table, "AXRowCount") == 6
        assert number(table, "AXColumnCount") == 3
        assert len(children(table, "AXColumnHeaderUIElements")) == 5
        assert len(children(table, "AXRowHeaderUIElements")) == 3
        rows = children(table, "AXRows")
        assert len(rows) == 6
        expected_spans = [[2, 1], [1, 1, 1], [1, 1, 1], [1, 1, 1], [1, 1, 1], [2, 1]]
        for row_index, (row, spans) in enumerate(zip(rows, expected_spans, strict=True)):
            cells = descendants(row, "AXCell")
            assert len(cells) == len(spans), (row_index, len(cells))
            column = 0
            for cell, span in zip(cells, spans, strict=True):
                assert index_range(cell, "AXRowIndexRange") == (row_index, 1)
                assert index_range(cell, "AXColumnIndexRange") == (column, span)
                column += span
        first_button = descendants(rows[2], "AXButton")[0]
        mac.perform(first_button, "AXPress")
        mac.wait_text(TITLE, "Opened Memory")
        mac.press(TITLE, "Reverse rows")
        deadline = time.monotonic() + 5
        while True:
            reordered = children(table, "AXRows")
            first_cells = descendants(reordered[2], "AXCell")
            if label(first_cells[0]) == "Streaming":
                break
            if time.monotonic() >= deadline:
                raise AssertionError("Table did not reverse its native row order")
            time.sleep(.05)
        moved_button = descendants(reordered[4], "AXButton")[0]
        assert equal(first_button, moved_button), "Row reorder replaced a keyed native button"
        mac.perform(descendants(reordered[2], "AXButton")[0], "AXPress")
        mac.wait_text(TITLE, "Opened Streaming")
        mac.perform(moved_button, "AXPress")
        mac.wait_text(TITLE, "Opened Memory")
        if images:
            screenshot(mac, images / "gallery-structural-table.png", title=TITLE)
        mac.press(TITLE, "Presentation")
        mac.wait_text(TITLE, "A little context goes a long way")
        print("GALLERY_STRUCTURAL_TABLE_OK: grouped/row headers, merged ranges, "
              "native actions, reordered control identity and page retirement", flush=True)
    finally:
        for node in retained:
            mac.release(node)
        mac.release(table)
