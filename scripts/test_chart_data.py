#!/usr/bin/env python3
"""Real macOS chart original-data browsing, including unpainted and sampled values."""
import argparse
import ctypes as C
from pathlib import Path
import subprocess
import time

from test_agent_chat import Mac
from test_canvas import screenshot
from test_charts import TITLE, DESCRIPTIONS


def row(mac, text):
    node = mac.wait_find(TITLE, text, 'AXRow', contains=True, search_files=True)
    mac.release(node)


def table(mac, family, count):
    node = mac.wait_find(TITLE, DESCRIPTIONS[family] + ' · original data', 'AXTable')
    try:
        value = mac.attr(node, 'AXRowCount')
        read = mac.cf.CFNumberGetValue
        read.restype, read.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        result = C.c_longlong()
        try:
            if not value or not read(value, 4, C.byref(result)) or result.value != count:
                raise RuntimeError(f'Unexpected accessible row count: {result.value}, expected {count}')
        finally:
            if value:
                mac.release(value)
        rows = mac.children(node, 'AXRows')
        try:
            if not (0 <= len(rows) <= 10) or (count > 0 and not rows):
                raise RuntimeError(f'Unbounded or missing original-data page: {len(rows)}')
            print('CHART_DATA_ROWS', count, len(rows), flush=True)
        finally:
            for item in rows:
                mac.release(item)
    finally:
        mac.release(node)


def focus(mac, family):
    node = mac.wait_find(TITLE, DESCRIPTIONS[family], 'AXGroup')
    try:
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)


def exercise(mac, output):
    mac.wait_text(TITLE, '48 values')
    mac.press(TITLE, 'Edge cases')
    mac.wait_text(TITLE, '100_000 values')
    mac.press(TITLE, 'View data')
    table(mac, 0, 100_000)
    row(mac, 'Row 1: Line · Original values. x: 0 · y: Missing. Series 1 · datum 100000')
    mac.key(119)  # End: last original, irrespective of the prepared envelope.
    row(mac, 'Row 100000:')
    table(mac, 0, 100_000)
    mac.wait_text(TITLE, 'current row 100000')
    mac.key(116)  # Page Up.
    mac.wait_text(TITLE, 'current row 99992')
    mac.key(115)  # Home.
    row(mac, 'Row 1:')
    mac.press(TITLE, 'Next data page')
    mac.wait_text(TITLE, 'current row 9')
    mac.press(TITLE, 'Previous data page')
    mac.wait_text(TITLE, 'current row 1')
    # Focus a native AX row; it changes browsing position, not plot selection.
    node = mac.wait_find(TITLE, 'Row 3:', 'AXRow', contains=True, search_files=True)
    try:
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)
    mac.wait_text(TITLE, 'current row 3')
    mac.wait_text(TITLE, 'Select a value ·')
    screenshot(mac, output / 'original-data.png', title=TITLE)
    mac.press(TITLE, 'Update data ↗')
    mac.wait_text(TITLE, '100_000 values')
    time.sleep(0.15)
    row(mac, 'Row 3:')
    mac.wait_text(TITLE, 'current row 3')
    focus(mac, 0)
    mac.key(53)  # Escape returns to the plot without changing its selection.
    mac.wait_text(TITLE, 'View data')
    focus(mac, 0)
    mac.key(2)  # D opens the built-in companion.
    table(mac, 0, 100_000)
    mac.key(2)
    mac.wait_text(TITLE, 'View data')

    cases = [
        (1, 'Area', 0, None),
        (2, 'Bar', 3, 'Row 1: Bar · Signed values. x: 0 · y: -5'),
        (3, 'Pie', 2, 'Row 1: Zero slice. 0. Slice 1'),
        (4, 'Radar', 3, 'Row 3: Reordered axes · Third axis. 0.125 / 1'),
        (5, 'Candlestick', 1, 'Row 1: Flat session. Open -2 · High -2 · Low -2 · Close -2'),
        (6, 'Sankey', 4, 'Row 4: Source → Target. 0. Edge 1'),
    ]
    for family, name, count, expected in cases:
        mac.press(TITLE, name)
        mac.release(mac.wait_find(TITLE, DESCRIPTIONS[family], 'AXGroup'))
        native_count = 1 if family == 6 else count
        mac.release(mac.wait_find(TITLE, f'{native_count} values · native rendering', 'AXStaticText'))
        mac.press(TITLE, 'View data')
        table(mac, family, count)
        if expected:
            row(mac, expected)
        else:
            mac.wait_text(TITLE, 'No data values')
        if family == 6:
            row(mac, 'Row 3: Isolated. Incoming 0 · Outgoing 0')
        mac.wait_text(TITLE, 'Select a value ·')
        # Reset while the data view is open must return to plot mode.
        if family == 6:
            mac.press(TITLE, 'Sample data')
            mac.release(mac.wait_find(TITLE, '4 values · native rendering', 'AXStaticText'))
            mac.wait_text(TITLE, 'View data')
            assert mac.find(TITLE, 'Original data · all values') is None
        else:
            mac.press(TITLE, 'Back to chart')
    mac.close(TITLE)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('.cache/chart-data-acceptance'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with (args.output / 'application.log').open('w') as log:
        child = subprocess.Popen(['_build/default/examples/charts/main.exe'], stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac, args.output)
            assert child.wait(timeout=15) == 0
            print('CHART_DATA_OK: 100k originals, bounded AX rows, keys/buttons/row focus, gaps/zeros/empty/negative/reordered axes/isolated nodes, reset, silent browsing, OS close', flush=True)
        finally:
            if mac is not None:
                mac.release(mac.app)
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()


if __name__ == '__main__':
    main()
