#!/usr/bin/env python3
"""Simultaneous large chat artifacts, streaming, native motion and scoped cleanup.

Requires macOS accessibility and screen-capture access. All input targets the
owned child. Windows are concurrently mounted but may be occluded; keyboard
latency is key-post to AX readback, not input-to-pixel latency. Always reaps child.
"""
import ctypes as C
import json
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
import time
from test_agent_chat_responsive import Responsive
from test_agent_chat_sources import Sources
from test_agent_chat_results import Results
from test_agent_chat_diagram import Diagram
from test_agent_chat_review import Review, TITLE, CONVERSATION
from test_canvas import screenshot

QUIET_FIELDS = ('commits', 'submission_attempts', 'attempted_bytes',
                'submitted_messages', 'submitted_bytes', 'received_events')
RESOURCE_FIELDS = ('scopes', 'tasks', 'cleanups', 'windows', 'assets',
                   'documents', 'document_source_bytes', 'canvases', 'canvas_scene_bytes')
POLL_FIELDS = ('elapsed_ms', 'clock_ticks', 'turns', 'drain_calls', 'drained_bytes')


class Combined(Responsive):
    current = TITLE

    def __init__(self, pid, child, log_path, directory):
        super().__init__(pid, child)
        self.log_path, self.directory = log_path, directory

    def pixels(self, path, bounds, window_bounds):
        # Decode the captured PNG with system ImageIO, with no Python imaging
        # dependency. Compare decoded spinner pixels, not PNG metadata.
        image_io = C.CDLL('/System/Library/Frameworks/ImageIO.framework/ImageIO')
        ptr = C.c_void_p

        def bind(lib, name, result, *args):
            fn = getattr(lib, name)
            fn.restype, fn.argtypes = result, args
            return fn

        class Point(C.Structure):
            _fields_ = [('x', C.c_double), ('y', C.c_double)]

        class Rect(C.Structure):
            _fields_ = [('origin', Point), ('size', Point)]
        url_create = bind(self.cf, 'CFURLCreateFromFileSystemRepresentation', ptr,
                          ptr, C.c_char_p, C.c_long, C.c_bool)
        source_create = bind(image_io, 'CGImageSourceCreateWithURL', ptr, ptr, ptr)
        image_create = bind(image_io, 'CGImageSourceCreateImageAtIndex', ptr, ptr, C.c_size_t, ptr)
        width = bind(self.cg, 'CGImageGetWidth', C.c_size_t, ptr)
        height = bind(self.cg, 'CGImageGetHeight', C.c_size_t, ptr)
        crop = bind(self.cg, 'CGImageCreateWithImageInRect', ptr, ptr, Rect)
        color_create = bind(self.cg, 'CGColorSpaceCreateDeviceRGB', ptr)
        bitmap_create = bind(self.cg, 'CGBitmapContextCreate', ptr, ptr, C.c_size_t,
                             C.c_size_t, C.c_size_t, C.c_size_t, ptr, C.c_uint32)
        draw = bind(self.cg, 'CGContextDrawImage', None, ptr, Rect, ptr)
        encoded = str(path).encode()
        url = url_create(None, encoded, len(encoded), False)
        source = source_create(url, None)
        image = image_create(source, 0, None) if source else None
        region = color = context = None
        try:
            if not image:
                raise RuntimeError('Cannot decode owned-window screenshot')
            x, y, w, h = bounds
            wx, wy, ww, wh = window_bounds
            sx, sy = width(image) / ww, height(image) / wh
            box = Rect(Point((x - wx) * sx, (y - wy) * sy), Point(w * sx, h * sy))
            region = crop(image, box)
            if not region or width(region) < 12 or height(region) < 12:
                raise RuntimeError('Spinner capture is empty or clipped')
            # A cropped CGImage can share its original data provider. Render it
            # into a fresh packed bitmap so unrelated pixels cannot count.
            w, h = width(region), height(region)
            buffer = C.create_string_buffer(w * h * 4)
            color = color_create()
            context = bitmap_create(buffer, w, h, 8, w * 4, color, 1 | (4 << 12))
            if not context:
                raise RuntimeError('Cannot create spinner bitmap')
            draw(context, Rect(Point(0, 0), Point(w, h)), region)
            pixels = buffer.raw
            if len(set(pixels[i:i + 4] for i in range(0, len(pixels), 4))) < 8:
                raise RuntimeError('Spinner capture does not contain its painted arc')
            return pixels
        finally:
            for value in (context, color, region, image, source, url):
                if value:
                    self.release(value)

    def animation_without_transactions(self):
        spinner = self.wait_find(TITLE, 'Generating response', 'AXProgressIndicator')
        window = self.window(TITLE)
        try:
            bounds, window_bounds = self.rect(spinner), self.rect(window)
        finally:
            self.release(spinner)
            self.release(window)
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline:
            before = self.samples()[-1]
            images = []
            for index in range(2):
                path = self.directory / f'animation-{index}.png'
                screenshot(self, path, title=self.current)
                images.append(self.pixels(path, bounds, window_bounds))
                time.sleep(.17)
            # The next sample brackets both captures. Thus the equality below
            # covers actual changing pixels, including any asynchronous work.
            last = self.samples()[-1]['elapsed_ms']
            while self.samples()[-1]['elapsed_ms'] <= last:
                time.sleep(.03)
            after = self.samples()[-1]
            if all(before[k] == after[k] for k in QUIET_FIELDS):
                assert images[0] != images[1], 'Native spinner did not change pixels'
                delta = {k: after[k] - before[k] for k in POLL_FIELDS + QUIET_FIELDS}
                assert delta['clock_ticks'] > 0
                print('NATIVE_ANIMATION_WITHOUT_TRANSACTIONS', json.dumps(delta), flush=True)
                return
        raise RuntimeError('No quiet inter-chunk interval brackets native animation')

    def close_selected(self):
        self.close(TITLE)
        deadline = time.monotonic() + 8
        while self.has_window(TITLE) and time.monotonic() < deadline:
            time.sleep(.05)
        assert not self.has_window(TITLE), self.current

    def same_resources(self, before, after):
        assert all(before[k] == after[k] for k in RESOURCE_FIELDS), (before, after)
        assert all(after[k] == 0 for k in ('queued_jobs', 'queued_commands',
                   'pending_requests', 'asset_uploads')), after

    def node_values(self, node):
        # Five separate IPC round trips per element can exhaust a full-tree
        # search deadline during animation. One batch also observes one snapshot.
        fields = ('AXRole', 'AXTitle', 'AXDescription', 'AXValue', 'AXChildren')
        names = [self.string(n) for n in fields]
        array = self.cf.CFArrayCreate
        array.restype = C.c_void_p
        array.argtypes = [C.c_void_p, C.POINTER(C.c_void_p), C.c_long, C.c_void_p]
        multiple = self.ax.AXUIElementCopyMultipleAttributeValues
        multiple.restype = C.c_int
        multiple.argtypes = [C.c_void_p, C.c_void_p, C.c_uint32, C.POINTER(C.c_void_p)]
        attrs = array(None, (C.c_void_p * len(names))(*names), len(names), None)
        values = C.c_void_p()
        try:
            if multiple(node, attrs, 0, C.byref(values)) or not values.value:
                return ['', '', '', ''], []
            texts = []
            for i in range(4):
                value = self.item(values, i)
                buffer = C.create_string_buffer(262145)
                is_text = (self.type_id(value) == self.string_type
                           and self.get_string(value, buffer, len(buffer), 0x08000100))
                texts.append(buffer.value.decode() if is_text else '')
            children = self.item(values, 4)
            count = self.count(children) if self.type_id(children) == self.array_type else 0
            if count > 4096:
                raise RuntimeError('Unexpected AX tree size')
            return texts, [self.retain(self.item(children, i)) for i in range(count)]
        finally:
            if values.value:
                self.release(values)
            self.release(attrs)
            for name in names:
                self.release(name)

    def find(self, title, label, role=None, contains=False, search_files=False):
        root = self.window(title)
        if not root:
            return None
        deadline = time.monotonic() + 8

        def visit(node, depth, in_conversation=False):
            if depth > 48 or time.monotonic() >= deadline:
                return None
            values, children = self.node_values(node)
            node_role = values[0]
            try:
                if any(label in v if contains else label == v for v in values[1:]) and (
                        role is None or role == node_role):
                    return self.retain(node)
                in_conversation = in_conversation or (
                    node_role == 'AXGroup' and values[1] == CONVERSATION)
                # Lookups in this walkthrough target controls and artifacts.
                # Transcript layout has its own dedicated streaming regression.
                if node_role == 'AXList' and in_conversation:
                    return None
                if not search_files and node_role in ('AXTable', 'AXOutline', 'AXBrowser'):
                    return None
                if search_files:
                    children.reverse()
                for child in children:
                    result = visit(child, depth + 1, in_conversation)
                    if result:
                        return result
            finally:
                for child in children:
                    self.release(child)
        try:
            return visit(root, 0)
        finally:
            self.release(root)

    def window(self, title):
        return super().window(self.current if title == TITLE else title)

    def select(self, number):
        self.current = f'GPUIO · Agent workspace {number}'
        self.release(self.wait_find(TITLE, 'Explore workspace', 'AXButton'))
        w = self.window(TITLE)
        try:
            self.perform(w, 'AXRaise')
        finally:
            self.release(w)
        time.sleep(.15)

    def wait_status(self, node, text):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            actual = self.text(node, 'AXTitle') or ''
            if text in actual:
                return
            time.sleep(.02)
        raise RuntimeError(f'Status handle missing {text}: {actual}')

    def samples(self):
        out = []
        for line in self.log_path.read_text().splitlines(keepends=True):
            if line.startswith('GPUIO_CHAT_WORKLOAD') and line.endswith('\n'):
                pairs = re.findall(r'\(([a-z_]+) ([0-9]+)\)', line)
                row = {k: int(v) for k, v in pairs}
                row['elapsed_ms'] = int(re.search(r'elapsed_ms=([0-9]+)', line).group(1))
                out.append(row)
        return out

    def report(self, label):
        time.sleep(.65)
        row = self.samples()[-1]
        row['rss_bytes'] = int(subprocess.check_output(
            ['ps', '-o', 'rss=', '-p', str(self.pid)], text=True, timeout=5).strip()) * 1024
        # Broad regression budget, not a performance promise.
        assert row['rss_bytes'] < 1536 * 1024 * 1024, row
        print(label, json.dumps(row, sort_keys=True), flush=True)
        return row

    def budget(self):
        self.select(1)
        Sources.check_row_budget(self)
        self.select(2)
        Results.check_budget(self)

    def exercise(self):
        self.select(1)
        self.set(self.app, 'AXFrontmost', self.true)
        self.report('BASELINE')
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.press(TITLE, 'Explore sources')
        self.press(TITLE, 'Load 100,000 sources')
        self.wait_text(TITLE, '100,000 loaded')
        self.press(TITLE, 'Reveal last source')
        self.release(self.wait_find(TITLE, 'Source 099997', 'AXRow', search_files=True))
        Sources.check_row_budget(self)
        self.press(TITLE, 'New window')
        self.select(2)
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Workspace')
        self.press(TITLE, 'Explore results')
        self.press(TITLE, 'Load 100,000 results')
        self.wait_text(TITLE, '100,000 results loaded')
        self.release(self.wait_find(TITLE, '000001', 'AXCell', search_files=True))
        Results.check_budget(self)
        self.press(TITLE, 'New window')
        self.select(3)
        self.press(TITLE, 'Explore workspace')
        self.press(TITLE, 'Diagram')
        self.wait_text(TITLE, 'Read sources · x 34 · y 42')
        self.press(TITLE, 'New window')
        self.select(4)
        self.press(TITLE, 'Explore workspace')
        self.wait_text(TITLE, '0 of 100 checkpoints')
        loaded = self.report('ALL_MOUNTED')
        assert loaded['windows'] == 4 and loaded['canvases'] >= 1
        self.draft(TITLE, CONVERSATION, 'Combined workload: streaming with large artifacts')
        status = self.wait_find(TITLE, '40 messages · Ready', 'AXStaticText')
        editor = self.wait_find(TITLE, 'Message · ' + CONVERSATION, 'AXTextArea')
        self.press(TITLE, 'Send')
        self.wait_status(status, 'Responding')
        deadline = time.monotonic() + 5
        while self.text(editor, 'AXValue') != '' and time.monotonic() < deadline:
            time.sleep(.02)
        assert self.text(editor, 'AXValue') == ''
        times = []
        try:
            self.set(editor, 'AXFocused', self.true)
            for count in range(1, 21):
                started = time.perf_counter()
                self.key(0)
                while self.text(editor, 'AXValue') != 'a' * count:
                    if time.perf_counter() - started > 2:
                        raise RuntimeError('Keyboard delivery timeout')
                    time.sleep(.002)
                times.append((time.perf_counter() - started) * 1000)
        finally:
            self.release(editor)
        print('KEYBOARD_AX_READBACK_MS', json.dumps(times), flush=True)
        assert sorted(times)[18] < 500 and max(times) < 1500, times
        self.wait_status(status, 'Responding')
        Review.click_counter(self, 0)
        self.wait_text(TITLE, '1 of 100 checkpoints')
        self.select(3)
        Diagram.focus_stage(self, 'Read sources')
        self.key(124, flags=1 << 17)
        self.wait_text(TITLE, 'Read sources · x 35 · y 42')
        self.budget()
        self.select(4)
        self.animation_without_transactions()
        before = self.report('STREAMING')
        time.sleep(5)
        after = self.report('STREAMING_END')
        rows = [s for s in self.samples() if before['elapsed_ms'] <=
                s['elapsed_ms'] <= after['elapsed_ms']]
        quiet = []
        for a, b in zip(rows, rows[2:]):
            if all(a[k] == b[k] for k in QUIET_FIELDS):
                quiet.append({k: b[k] - a[k] for k in POLL_FIELDS + QUIET_FIELDS})
        print('ANIMATION_QUIET_INTERVALS', json.dumps(quiet), flush=True)
        assert quiet, rows
        self.press(TITLE, 'Cancel')
        self.wait_status(status, 'Cancelled')
        self.release(status)
        self.draft(TITLE, CONVERSATION, '')
        for number in (4, 3, 2):
            self.select(number)
            self.close_selected()
        self.select(1)
        closed = self.report('ONE_WINDOW_RETAINED')
        assert closed['windows'] == 1 and closed['canvases'] == 0
        for _ in range(3):
            self.press(TITLE, 'Close workspace inspector')
            self.absent('Close workspace inspector', 'AXButton')
            self.press(TITLE, 'Explore workspace')
            Sources.check_row_budget(self)
        remounted = self.report('REMOUNTED')
        self.same_resources(closed, remounted)
        # Recreate/dispose actual canvas registrations and native components in
        # three new windows. Source data stays live in the original window.
        for number in (5, 6, 7):
            self.press(TITLE, 'New window')
            self.select(number)
            self.press(TITLE, 'Explore workspace')
            Review.click_counter(self, 0)
            self.wait_text(TITLE, '1 of 100 checkpoints')
            self.press(TITLE, 'Diagram')
            self.wait_text(TITLE, 'Read sources · x 34 · y 42')
            mounted = self.report(f'RECREATED_{number}')
            assert mounted['canvases'] == 1 and mounted['windows'] == 2, mounted
            for _ in range(2):
                self.press(TITLE, 'Close workspace inspector')
                self.absent('Read sources', 'AXStaticText')
                self.press(TITLE, 'Explore workspace')
                self.wait_text(TITLE, 'Read sources · x 34 · y 42')
            self.same_resources(mounted, self.report(f'CANVAS_REMOUNTED_{number}'))
            self.close_selected()
            self.select(1)
            self.same_resources(closed, self.report(f'RELEASED_{number}'))
        self.close_selected()


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')

    def timeout(_signal, _frame):
        raise TimeoutError('Combined workload exceeded its 240-second budget')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix='gpuio-combined-') as temporary:
        directory = Path(temporary)
        log_path = directory / 'child.log'
        with log_path.open('w') as log:
            child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe'),
                                      '--workload-metrics', '--full-motion'], cwd=repo,
                                     stdout=log, stderr=subprocess.STDOUT)
            mac = None
            try:
                signal.alarm(240)
                mac = Combined(child.pid, child, log_path, directory)
                mac.exercise()
                if child.wait(timeout=15):
                    raise RuntimeError('Combined workload child failed')
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
                print(log_path.read_text(), end='')
    print('GPUIO_CHAT_COMBINED_APPKIT_OK', flush=True)


if __name__ == '__main__':
    main()
