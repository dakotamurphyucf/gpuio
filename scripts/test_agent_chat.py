#!/usr/bin/env python3
"""Exercise only a child GPUIO app through real macOS AX controls and keyboard.

Build examples/agent_chat/main.exe first. Requires the same accessibility access
as the native picker/editor suites. The child is always reaped, even on failure.
"""
import ctypes as C
from pathlib import Path
import subprocess
import sys
import tempfile
import time


class Mac:
    def __init__(self, pid, child=None):
        self.pid = pid
        self.child = child
        self.cf = C.CDLL('/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation')
        self.ax = C.CDLL('/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices')
        self.cg = C.CDLL('/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics')
        def bind(lib, name, result, *args):
            fn = getattr(lib, name)
            fn.restype, fn.argtypes = result, args
            return fn
        ptr = C.c_void_p
        self.release = bind(self.cf, 'CFRelease', None, ptr)
        self.retain = bind(self.cf, 'CFRetain', ptr, ptr)
        self.make_string = bind(self.cf, 'CFStringCreateWithCString', ptr, ptr, C.c_char_p, C.c_uint)
        self.get_string = bind(self.cf, 'CFStringGetCString', C.c_bool, ptr, ptr, C.c_long, C.c_uint)
        self.type_id = bind(self.cf, 'CFGetTypeID', C.c_ulong, ptr)
        self.string_type = bind(self.cf, 'CFStringGetTypeID', C.c_ulong)()
        self.array_type = bind(self.cf, 'CFArrayGetTypeID', C.c_ulong)()
        self.count = bind(self.cf, 'CFArrayGetCount', C.c_long, ptr)
        self.item = bind(self.cf, 'CFArrayGetValueAtIndex', ptr, ptr, C.c_long)
        self.copy = bind(self.ax, 'AXUIElementCopyAttributeValue', C.c_int, ptr, ptr, C.POINTER(ptr))
        self.set_attr = bind(self.ax, 'AXUIElementSetAttributeValue', C.c_int, ptr, ptr, ptr)
        self.action = bind(self.ax, 'AXUIElementPerformAction', C.c_int, ptr, ptr)
        if not bind(self.ax, 'AXIsProcessTrusted', C.c_bool)():
            raise RuntimeError('macOS accessibility access is unavailable to this test process')
        self.app = bind(self.ax, 'AXUIElementCreateApplication', ptr, C.c_int)(pid)
        self.true = ptr.in_dll(self.cf, 'kCFBooleanTrue').value
        self.key_event = bind(self.cg, 'CGEventCreateKeyboardEvent', ptr, ptr, C.c_ushort, C.c_bool)
        self.key_flags = bind(self.cg, 'CGEventSetFlags', None, ptr, C.c_ulonglong)
        self.post_key = bind(self.cg, 'CGEventPostToPid', None, C.c_int, ptr)

    def string(self, text):
        return self.make_string(None, text.encode(), 0x08000100)

    def attr(self, node, name):
        attribute, value = self.string(name), C.c_void_p()
        try:
            return value.value if self.copy(node, attribute, C.byref(value)) == 0 else None
        finally:
            self.release(attribute)

    def text(self, node, name):
        value = self.attr(node, name)
        if not value:
            return None
        try:
            if self.type_id(value) != self.string_type:
                return None
            buffer = C.create_string_buffer(262145)
            return buffer.value.decode() if self.get_string(value, buffer, len(buffer), 0x08000100) else None
        finally:
            self.release(value)

    def children(self, node, attribute='AXChildren'):
        value = self.attr(node, attribute)
        if not value:
            return []
        try:
            if self.type_id(value) != self.array_type:
                return []
            count = self.count(value)
            if count > 4096:
                raise RuntimeError('Unexpected accessibility tree size')
            return [self.retain(self.item(value, i)) for i in range(count)]
        finally:
            self.release(value)

    def window(self, title):
        windows = self.children(self.app, 'AXWindows')
        try:
            for window in windows:
                if self.text(window, 'AXTitle') == title:
                    return self.retain(window)
        finally:
            for window in windows:
                self.release(window)
        return None

    def find(self, title, label, role=None, contains=False):
        root = self.window(title)
        if not root:
            return None
        deadline = time.monotonic() + 3
        def visit(node, depth):
            if depth > 48 or time.monotonic() >= deadline:
                return None
            values = [self.text(node, field) or '' for field in ['AXTitle', 'AXDescription', 'AXValue']]
            matches = any(label in value if contains else label == value for value in values)
            if matches and (role is None or self.text(node, 'AXRole') == role):
                return self.retain(node)
            if role != 'AXTextField' and self.text(node, 'AXRole') in ['AXTable', 'AXOutline', 'AXBrowser']:
                return None
            children = self.children(node)
            try:
                for child in children:
                    found = visit(child, depth + 1)
                    if found:
                        return found
            finally:
                for child in children:
                    self.release(child)
            return None
        try:
            return visit(root, 0)
        finally:
            self.release(root)

    def wait_find(self, title, label, role=None, contains=False):
        end = time.monotonic() + 35
        while time.monotonic() < end:
            if self.child is not None and self.child.poll() is not None:
                raise RuntimeError(f"Reference app exited early: {self.child.returncode}")
            node = self.find(title, label, role, contains)
            if node:
                return node
            time.sleep(0.05)
        self.dump(title)
        raise RuntimeError(f'Timed out finding {label!r} ({role}) in {title}')

    def dump(self, title):
        root = self.window(title)
        if not root:
            print('Window absent:', title, flush=True)
            return
        def visit(node, depth):
            print(' ' * depth, self.text(node, 'AXRole'),
                  [self.text(node, field) for field in ['AXTitle', 'AXDescription', 'AXValue']], flush=True)
            if depth < 12 and self.text(node, 'AXRole') not in ['AXTable', 'AXOutline', 'AXBrowser']:
                children = self.children(node)
                try:
                    for child in children:
                        visit(child, depth + 1)
                finally:
                    for child in children:
                        self.release(child)
        try:
            visit(root, 0)
        finally:
            self.release(root)

    def wait_text(self, title, text):
        print('AX_WAIT', text, flush=True)
        node = self.wait_find(title, text, contains=True)
        self.release(node)

    def perform(self, node, action):
        value = self.string(action)
        try:
            result = self.action(node, value)
            if result:
                raise RuntimeError(f'{action} failed: {result}')
        finally:
            self.release(value)

    def press(self, title, label):
        print('AX_PRESS', label, flush=True)
        node = self.wait_find(title, label, 'AXButton')
        try:
            self.perform(node, 'AXPress')
        finally:
            self.release(node)

    def set(self, node, attribute, value):
        name = self.string(attribute)
        try:
            result = self.set_attr(node, name, value)
            if result:
                raise RuntimeError(f'{attribute} failed: {result}')
        finally:
            self.release(name)

    def field(self, title, label, role, text=None):
        node = self.wait_find(title, label, role)
        try:
            if text is not None:
                value = self.string(text)
                try:
                    self.set(node, 'AXFocused', self.true)
                    self.set(node, 'AXValue', value)
                finally:
                    self.release(value)
            return self.text(node, 'AXValue')
        finally:
            self.release(node)

    def draft(self, title, conversation, text=None):
        return self.field(title, 'Message · ' + conversation, 'AXTextArea', text)

    def key(self, code, flags=0):
        for down in [True, False]:
            event = self.key_event(None, code, down)
            if not event:
                raise RuntimeError('Cannot create keyboard event')
            try:
                self.key_flags(event, flags)
                self.post_key(self.pid, event)
            finally:
                self.release(event)

    def close(self, title):
        window = self.window(title)
        if not window:
            raise RuntimeError('Expected open window')
        button = self.attr(window, 'AXCloseButton')
        try:
            if not button:
                raise RuntimeError('Native window has no close button')
            self.perform(button, 'AXPress')
        finally:
            if button:
                self.release(button)
            self.release(window)

    def has_window(self, title):
        window = self.window(title)
        if window:
            self.release(window)
            return True
        return False


def exercise(mac, attachment):
    first, second = 'GPUIO · Agent workspace 1', 'GPUIO · Agent workspace 2'
    one, two = 'Build a native app', 'Review a patch'
    mac.field(first, 'Find conversations', 'AXTextField', 'review')
    node = mac.wait_find(first, two, 'AXButton')
    mac.release(node)
    end = time.monotonic() + 10
    while time.monotonic() < end:
        node = mac.find(first, one, 'AXButton')
        if not node:
            break
        mac.release(node)
        time.sleep(0.05)
    else:
        raise RuntimeError('Conversation search did not filter the sidebar')
    mac.field(first, 'Find conversations', 'AXTextField', '')
    node = mac.wait_find(first, one, 'AXButton')
    mac.release(node)
    mac.draft(first, one, 'Native Send button λ')
    mac.press(first, 'Send')
    mac.wait_text(first, 'Sending…')
    mac.draft(first, one, 'Keep this newer draft')
    mac.wait_text(first, 'your newer draft was kept')
    mac.wait_text(first, '· Complete')
    actual = mac.draft(first, one)
    assert actual == 'Keep this newer draft', repr(actual)
    mac.press(first, 'Demo controls')
    mac.press(first, 'Simulate error')
    mac.press(first, two)
    mac.draft(first, two, 'Native Enter submission')
    mac.key(36)  # Return, delivered only to the child PID.
    mac.wait_text(first, 'Simulated connection interrupted')
    mac.press(first, 'Retry response')
    mac.wait_text(first, '· Complete')
    mac.press(first, one)
    actual = mac.draft(first, one)
    assert actual == 'Keep this newer draft', repr(actual)
    mac.press(first, 'Close tab')
    node = mac.wait_find(first, 'Message · ' + two, 'AXTextArea')
    mac.release(node)
    mac.press(first, one)
    actual = mac.draft(first, one)
    assert actual == 'Keep this newer draft', repr(actual)
    mac.press(first, 'Attach text…')
    filename = mac.wait_find(first, attachment.name, 'AXTextField')
    row = filename
    try:
        while mac.text(row, 'AXRole') != 'AXRow':
            parent = mac.attr(row, 'AXParent')
            mac.release(row)
            row = parent
            if not row:
                raise RuntimeError('Attachment has no selectable native row')
        mac.set(row, 'AXSelected', mac.true)
    finally:
        if row:
            mac.release(row)
    mac.press(first, 'Open')
    mac.wait_text(first, 'Attached native-attachment.txt')
    mac.press(first, 'New window')
    assert mac.draft(second, one) == ''
    mac.draft(second, one, 'Independent window draft')
    mac.press(second, 'Light theme')
    mac.wait_text(second, 'Dark theme')
    mac.key(35, (1 << 20) | (1 << 17))  # Command-Shift-P.
    mac.wait_text(second, 'Workspace commands')
    mac.key(53)  # Escape.
    mac.close(second)
    mac.wait_text(second, 'Close this window and discard its drafts?')
    mac.press(second, 'Keep editing')
    assert mac.has_window(second)
    mac.close(second)
    mac.press(second, 'Discard drafts and close')
    end = time.monotonic() + 10
    while mac.has_window(second) and time.monotonic() < end:
        time.sleep(0.05)
    assert not mac.has_window(second) and mac.has_window(first)
    actual = mac.draft(first, one)
    assert actual == 'Keep this newer draft', repr(actual)
    mac.close(first)
    mac.press(first, 'Discard drafts and close')


def main():
    if sys.platform != 'darwin':
        raise SystemExit('This harness exercises macOS AX; Linux GUI remains an informational job.')
    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix='gpuio-chat-') as directory, tempfile.TemporaryFile(mode='w+t') as log:
        attachment = Path(directory) / 'native-attachment.txt'
        attachment.write_text('Native attachment λ\nRead through Eio and rendered as a document.\n')
        child = subprocess.Popen([str(root / '_build/default/examples/agent_chat/main.exe'), '--native-test', '--directory', directory], cwd=root, stdout=log, stderr=subprocess.STDOUT)
        try:
            exercise(Mac(child.pid, child), attachment)
            if child.wait(timeout=15):
                raise RuntimeError('Reference app failed')
        finally:
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            log.seek(0)
            output = log.read()
            print(output, end='')
        if 'GPUIO_AGENT_CHAT_NATIVE_APP_RETURNED' not in output:
            raise RuntimeError('Missing post-App.run marker')
        print('GPUIO_AGENT_CHAT_NATIVE_OK: native search, AX Send, targeted Return, error/retry, retained tabs, independent windows, native picker/Eio attachment, theme/keyboard palette, OS close deny/allow and application return')


if __name__ == '__main__':
    main()
