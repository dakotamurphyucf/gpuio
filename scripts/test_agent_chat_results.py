#!/usr/bin/env python3
"""Native chat results flow; simulated queries, public table, owned app cleanup."""
import ctypes as C
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_review import Review, TITLE, CONVERSATION
from test_tree_outline import Point
from test_canvas import screenshot


class Results(Review):
    def prepare_window(self):
        # The demo requests 1180x820. Hosted displays can be smaller, while AX
        # still reports unclipped column bounds beyond the physical screen.
        # Fit only our child window before sending any pointer input.
        class Rect(C.Structure):
            _fields_ = [('origin', Point), ('size', Point)]
        main_display = self.cg.CGMainDisplayID
        main_display.restype, main_display.argtypes = C.c_uint32, []
        display_bounds = self.cg.CGDisplayBounds
        display_bounds.restype, display_bounds.argtypes = Rect, [C.c_uint32]
        display = display_bounds(main_display())
        width, height = min(1000, display.size.x - 24), min(700, display.size.y - 80)
        if width < 960 or height < 650:
            raise RuntimeError(f'Results pointer fixture needs a 984x730 display: '
                               f'got {display.size.x}x{display.size.y}')
        create = self.ax.AXValueCreate
        create.restype, create.argtypes = C.c_void_p, [C.c_int, C.c_void_p]
        window = self.window(TITLE)
        origin = Point(display.origin.x + 12, display.origin.y + 40)
        def set_geometry(name, kind, point):
            value = create(kind, C.byref(point))
            try:
                self.set(window, name, value)
            finally:
                self.release(value)
        try:
            position, size = self.bounds(TITLE, 'AXWindow')
            print(f'CHAT_RESULTS_WINDOW_REQUEST screen=({display.origin.x},{display.origin.y},'
                  f'{display.size.x},{display.size.y}) initial=({position.x},{position.y},'
                  f'{size.x},{size.y}) target=({origin.x},{origin.y},{width},{height})', flush=True)
            set_geometry('AXPosition', 1, origin)
            set_geometry('AXSize', 2, Point(width, height))
            # Observe size before positioning again: AppKit can constrain a move
            # against the old, oversized frame while resize is still in flight.
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                position, size = self.bounds(TITLE, 'AXWindow')
                if abs(size.x - width) < 1 and abs(size.y - height) < 1:
                    break
                time.sleep(.03)
            else:
                raise RuntimeError(f'Results resize did not settle: requested={width}x{height}, '
                                   f'actual=({position.x},{position.y},{size.x},{size.y})')
            set_geometry('AXPosition', 1, origin)
        finally:
            self.release(window)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            position, size = self.bounds(TITLE, 'AXWindow')
            if (abs(size.x - width) < 1 and abs(size.y - height) < 1
                    and position.x >= display.origin.x and position.y >= display.origin.y
                    and position.x + size.x <= display.origin.x + display.size.x
                    and position.y + size.y <= display.origin.y + display.size.y):
                print(f'CHAT_RESULTS_WINDOW screen={display.size.x}x{display.size.y} '
                      f'window=({position.x},{position.y},{size.x},{size.y})', flush=True)
                return
            time.sleep(.03)
        raise RuntimeError(f'Results window did not fit display '
                           f'({display.origin.x},{display.origin.y},{display.size.x},{display.size.y}): '
                           f'window=({position.x},{position.y},{size.x},{size.y})')

    def bounds(self, label, role=None):
        if role == 'AXColumnHeader':
            # macOS maps headers and data cells to AXCell; both share the
            # column label. Only the header lacks the data cell's AXValue.
            def header(node):
                if (self.text(node, 'AXRole') == 'AXCell'
                        and label in [self.text(node, 'AXTitle'), self.text(node, 'AXDescription')]
                        and self.text(node, 'AXValue') is None):
                    return self.retain(node)
                children = self.children(node)
                try:
                    for child in children:
                        found = header(child)
                        if found:
                            return found
                finally:
                    for child in children:
                        self.release(child)
                return None
            root = self.wait_find(TITLE, 'Run results', 'AXTable', search_files=True)
            try:
                node = header(root)
            finally:
                self.release(root)
            if not node:
                raise RuntimeError(f'Missing visible column header: {label}')
        else:
            node = self.wait_find(TITLE, label, role, search_files=True)
        try:
            position, size = Point(), Point()
            for name, kind, result in [('AXPosition', 1, position), ('AXSize', 2, size)]:
                raw = self.attr(node, name)
                try:
                    if not raw or not self.value(raw, kind, C.byref(result)):
                        raise RuntimeError(f'Missing geometry: {label}')
                finally:
                    if raw:
                        self.release(raw)
            if size.x <= 0 or size.y <= 0:
                raise RuntimeError(f'Invisible element: {label}')
            return position, size
        finally:
            self.release(node)

    def cell(self, label):
        position, size = self.bounds(label, 'AXCell')
        return Point(position.x + size.x / 2, position.y + size.y / 2)

    def click(self, point, right=False):
        self.send(5, point)
        self.send(3 if right else 1, point, 1 if right else 0)
        try:
            time.sleep(.04)
        finally:
            self.send(4 if right else 2, point, 1 if right else 0)

    def drag(self, start, end):
        point = start
        self.send(5, start)
        self.send(1, start)
        try:
            for step in range(1, 21):
                point = Point(start.x + (end.x - start.x) * step / 20,
                              start.y + (end.y - start.y) * step / 20)
                self.send(6, point)
                time.sleep(.02)
        finally:
            self.send(2, point)

    def sort(self, label):
        node = self.wait_find(TITLE, 'Sort ' + label, 'AXButton', search_files=True)
        try:
            self.perform(node, 'AXPress')
        finally:
            self.release(node)

    def copied(self, expected):
        self.key(8, flags=1 << 20)  # Command-C through the native table action.
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            text = subprocess.run(['/usr/bin/pbpaste'], capture_output=True,
                                  check=True, timeout=2,
                                  env={**os.environ, 'LC_ALL': 'en_US.UTF-8'}).stdout.decode('utf-8')
            if text == expected:
                return
            time.sleep(.05)
        raise RuntimeError('Native table did not copy the complete Unicode finding')

    def columns(self):
        # Resize the pinned ID boundary, then reorder two unpinned columns.
        position, before = self.bounds('RESULT', 'AXColumnHeader')
        self.drag(Point(position.x + before.x - 1, position.y + before.y / 2),
                  Point(position.x + before.x + 19, position.y + before.y / 2))
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            _, after = self.bounds('RESULT', 'AXColumnHeader')
            if after.x > before.x + 10:
                break
            time.sleep(.05)
        else:
            raise RuntimeError(f'Column resize was not accepted: before={before.x}, after={after.x}, origin={position.x},{position.y}, height={before.y}')
        # Resizing the pinned column can push SCORE out of the clipped table
        # viewport. Native keyboard navigation reveals it before pointer input.
        self.click(self.cell('000001'))
        self.wait_text(TITLE, 'Selected result 000001 · number')
        self.key(124)
        self.wait_text(TITLE, 'Selected result 000001 · tool')
        self.key(124)
        self.wait_text(TITLE, 'Selected result 000001 · score')
        tool, tool_size = self.bounds('TOOL', 'AXColumnHeader')
        score, score_size = self.bounds('SCORE', 'AXColumnHeader')
        pinned, pinned_size = self.bounds('RESULT', 'AXColumnHeader')
        table, table_size = self.bounds('Run results', 'AXTable')
        window, window_size = self.bounds(TITLE, 'AXWindow')
        left = max(pinned.x + pinned_size.x, table.x, window.x) + 1
        right = min(table.x + table_size.x, window.x + window_size.x) - 1
        def visible_point(position, size, trailing=False):
            start, end = max(left, position.x), min(right, position.x + size.x)
            if end - start < 4:
                raise RuntimeError(f'Column has no visible drag target: '
                                   f'header=({position.x},{size.x}), viewport=({left},{right})')
            # Scrolling can move part of TOOL beneath the pinned ID column.
            inset = min(8, (end - start) / 2)
            return Point(end - inset if trailing else start + inset, position.y + size.y / 2)
        print(f'CHAT_RESULTS_COLUMN_GEOMETRY tool=({tool.x},{tool.y},{tool_size.x},{tool_size.y}) '
              f'score=({score.x},{score.y},{score_size.x},{score_size.y}) '
              f'viewport=({left},{right})', flush=True)
        # TOOL's leading insertion gap may be hidden behind the pinned column.
        # Moving TOOL after SCORE expresses the same swap in the visible area.
        target = visible_point(score, score_size, trailing=True)
        assert target.x > score.x + score_size.x / 2, 'SCORE trailing gap must be visible'
        self.drag(visible_point(tool, tool_size), target)
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            tool, _ = self.bounds('TOOL', 'AXColumnHeader')
            score, _ = self.bounds('SCORE', 'AXColumnHeader')
            if score.x < tool.x:
                break
            time.sleep(.05)
        else:
            raise RuntimeError('Column reorder was not accepted')
        self.press(TITLE, 'Diagram')
        self.press(TITLE, 'Back')
        _, after = self.bounds('RESULT', 'AXColumnHeader')
        tool, _ = self.bounds('TOOL', 'AXColumnHeader')
        score, _ = self.bounds('SCORE', 'AXColumnHeader')
        assert after.x > before.x + 10 and score.x < tool.x
        self.press(TITLE, 'Reset columns')

    def absent(self, label):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            node = self.find(TITLE, label, search_files=True)
            if not node:
                return
            self.release(node)
            time.sleep(.05)
        raise RuntimeError(f'Unexpected retained element: {label}')

    def check_budget(self):
        table = self.wait_find(TITLE, 'Run results', 'AXTable', search_files=True)
        def count(node):
            role = self.text(node, 'AXRole')
            rows, cells = int(role == 'AXRow'), int(role == 'AXCell')
            children = self.children(node)
            try:
                for child in children:
                    r, c = count(child)
                    rows, cells = rows + r, cells + c
                return rows, cells
            finally:
                for child in children:
                    self.release(child)
        try:
            rows, cells = count(table)
        finally:
            self.release(table)
        if not (0 < rows <= 24 and 0 < cells <= 96):
            raise RuntimeError(f'Results exceed native budget: {rows} rows, {cells} cells')
        print(f'CHAT_RESULTS_NATIVE_BUDGET rows={rows}/24 cells={cells}/96 dataset=100000', flush=True)

    def exercise(self):
        self.draft(TITLE, CONVERSATION, 'Results stay beside this draft λ')
        self.set(self.app, 'AXFrontmost', self.true)
        window = self.window(TITLE)
        try:
            self.perform(window, 'AXRaise')
        finally:
            self.release(window)
        self.prepare_window()
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.press(TITLE, 'Explore results')
        self.wait_text(TITLE, '24 results loaded')
        self.columns()
        self.click(self.cell('000001'))
        self.wait_text(TITLE, 'Selected result 000001 · number')
        self.key(125)
        self.wait_text(TITLE, 'Selected result 000002 · number')
        self.key(36)
        self.wait_text(TITLE, 'Result 000002 · Read source · score 74%')
        self.key(53)
        self.absent('Finding actions')
        self.key(109, flags=1 << 17)  # Native Shift+F10; restored table focus.
        self.wait_text(TITLE, 'Finding details')
        self.press(TITLE, 'Reveal finding')
        self.absent('Finding actions')
        self.wait_text(TITLE, 'Selected result 000002 · summary')
        self.copied('Finding 000002 · 日本語 · 👨‍👩‍👧‍👦')
        self.key(123)
        self.wait_text(TITLE, 'Selected result 000002 · score')
        self.click(self.cell('000003'), right=True)
        self.wait_text(TITLE, 'Result 000003 · Search · score 10%')
        self.press(TITLE, 'Close finding details')
        self.absent('Finding actions')
        self.wait_text(TITLE, 'Selected result 000002 · score')
        self.press(TITLE, 'Load more results')
        self.wait_text(TITLE, '48 results loaded')
        self.sort('RESULT')  # Default -> descending, across all 48 results.
        self.wait_text(TITLE, 'Sort: number · descending')
        self.press(TITLE, 'Reveal last result')
        self.wait_text(TITLE, 'Selected result 000001 · number')
        self.cell('000001')
        self.sort('RESULT')  # Ascending.
        self.wait_text(TITLE, 'Sort: number · ascending')
        self.press(TITLE, 'Reveal last result')
        self.wait_text(TITLE, 'Selected result 000048 · number')
        self.press(TITLE, 'Unpin result IDs')
        self.wait_text(TITLE, 'Pin result IDs')
        self.press(TITLE, 'Pin result IDs')
        self.press(TITLE, 'Reset columns')
        self.press(TITLE, 'Reveal last result')
        self.wait_text(TITLE, 'Selected result 000048 · number')
        self.press(TITLE, 'Diagram')
        self.press(TITLE, 'Back')
        self.wait_text(TITLE, 'Selected result 000048 · number')
        self.press(TITLE, 'Close workspace inspector')
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, 'Selected result 000048 · number')
        self.press(TITLE, 'Score ≥ 80')
        self.wait_text(TITLE, '10 results loaded')
        self.press(TITLE, 'Empty results')
        self.wait_text(TITLE, '0 results loaded')
        self.wait_text(TITLE, 'No findings match')
        self.press(TITLE, 'Fail next query')
        self.wait_text(TITLE, 'Sample results unavailable. Retry to continue.')
        self.press(TITLE, 'Retry results')
        self.wait_text(TITLE, '24 results loaded')
        self.press(TITLE, 'Slow query')
        self.wait_text(TITLE, 'Loading results…')
        self.press(TITLE, 'Empty results')
        self.wait_text(TITLE, '0 results loaded')
        time.sleep(5.2)  # Past the retired request's deterministic five-second producer.
        self.wait_text(TITLE, '0 results loaded')
        self.press(TITLE, 'Load 100,000 results')
        self.wait_text(TITLE, '100,000 results loaded')
        self.press(TITLE, 'Reveal last result')
        self.wait_text(TITLE, 'Selected result 100000 · number')
        self.cell('100000')
        self.check_budget()
        self.press(TITLE, 'Paged sample')
        self.wait_text(TITLE, '24 results loaded')
        self.click(self.cell('000001'))
        images = os.environ.get('GPUIO_SCREENSHOT_DIR')
        if images:
            time.sleep(.2)
            screenshot(self, Path(images) / 'results-dark.png')
        self.press(TITLE, 'Light theme')
        self.wait_text(TITLE, 'Dark theme')
        if images:
            time.sleep(.2)
            screenshot(self, Path(images) / 'results-light.png')
        assert self.draft(TITLE, CONVERSATION) == 'Results stay beside this draft λ'
        self.close(TITLE)
        self.press(TITLE, 'Discard drafts and close')


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This test requires AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Chat results exceeded its 180-second budget')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe')],
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(180)
            mac = Results(child.pid, child)
            mac.exercise()
            if child.wait(timeout=15):
                raise RuntimeError('Chat example failed')
        finally:
            signal.alarm(0)
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
    print('GPUIO_CHAT_RESULTS_APPKIT_OK: native cell pointer/keys/context/reveal, '
          'paging/failure/retry/query cancellation, filters, native resize/reorder/sort, Unicode copy, column pin/reset, '
          'page/inspector retention, 100k rows, themes, draft and cleanup')


if __name__ == '__main__':
    main()
