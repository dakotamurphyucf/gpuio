#!/usr/bin/env python3
"""Real macOS AX selection/range checks against the public gallery.

Owns one GUI child, preserves the clipboard, and checks page text independently
of the application's model. This does not qualify VoiceOver or range geometry.
"""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import time

from mac_input_source import foreground_keys
from mac_clipboard import preserved_clipboard
from test_agent_chat import Mac
from test_gallery import TITLE, expect_field, focus_gallery_control


class Range(C.Structure):
    _fields_ = [('location', C.c_long), ('length', C.c_long)]


def utf16(text):
    return len(text.encode('utf-16-le')) // 2


class Selection:
    def __init__(self, mac):
        self.mac = mac
        self.get = mac.ax.AXValueGetValue
        self.get.restype, self.get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
        self.create = mac.ax.AXValueCreate
        self.create.restype, self.create.argtypes = C.c_void_p, [C.c_int, C.c_void_p]

    def read(self, node):
        value = self.mac.attr(node, 'AXSelectedTextRange')
        if not value:
            raise AssertionError('Missing AXSelectedTextRange')
        result = Range()
        try:
            assert self.get(value, 4, C.byref(result)), 'Expected CFRange'
            return [result.location, result.length]
        finally:
            self.mac.release(value)

    def set(self, node, location, length):
        value = self.create(4, C.byref(Range(location, length)))
        assert value
        try:
            self.mac.set(node, 'AXSelectedTextRange', value)
        finally:
            self.mac.release(value)

    def expect(self, node, text, location, length):
        deadline = time.monotonic() + 5
        while True:
            selected = self.mac.text(node, 'AXSelectedText')
            selection = self.read(node)
            if selected == text and selection == [location, length]:
                return {'text': selected, 'range_utf16': selection}
            assert time.monotonic() < deadline, (selected, selection, text, location, length)
            time.sleep(.025)


def exercise(mac, board, *, source_only):
    selection = Selection(mac)
    observations = []
    mac.press(TITLE, 'Markdown & code')
    mac.press(TITLE, 'Code')
    focus_gallery_control(mac, 'Code preview', 'AXTextArea')
    foreground_keys(mac)
    node = mac.wait_find(TITLE, 'Code preview', 'AXTextArea')
    try:
        text = mac.text(node, 'AXValue')
        assert '世界' in text and 'let greeting' in text
        mac.key(0, flags=1 << 20)
        observations.append({'case': 'source-native-select-all',
                             **selection.expect(node, text, 0, utf16(text))})
        start = utf16(text[:text.index('世界')])
        selection.set(node, start, 2)
        observations.append({'case': 'source-ax-unicode-selection',
                             **selection.expect(node, '世界', start, 2)})
        sequence = board.call(board.board, 'changeCount', result=C.c_long)
        mac.key(8, flags=1 << 20)
        deadline = time.monotonic() + 5
        while (board.call(board.board, 'changeCount', result=C.c_long) == sequence
               or board.text() != '世界'):
            assert time.monotonic() < deadline, 'Native clipboard selection did not match'
            time.sleep(.025)
        mac.key(51)  # Read-only source permits selecting/copying, not deletion.
        time.sleep(.1)
        assert mac.text(node, 'AXValue') == text
        selection.set(node, utf16(text), 0)
        observations.append({'case': 'source-trailing-empty-line-caret',
                             **selection.expect(node, '', utf16(text), 0)})
    finally:
        mac.release(node)
    if source_only:
        return observations

    mac.press(TITLE, 'Text editing')
    mac.press(TITLE, 'Multiline & search')
    label = 'Working notes'
    focus_gallery_control(mac, label, 'AXTextArea')
    text = 'Prefix λ🙂\nFamily 👨‍👩‍👧‍👦 and e\u0301\n日本語\n'
    mac.field(TITLE, label, 'AXTextArea', text)
    expect_field(mac, TITLE, label, text, 'AXTextArea')
    node = mac.wait_find(TITLE, label, 'AXTextArea')
    try:
        for selected in ['🙂', '👨‍👩‍👧‍👦', 'e\u0301', '日本語', 'λ🙂\nFamily']:
            start, length = utf16(text[:text.index(selected)]), utf16(selected)
            selection.set(node, start, length)
            observations.append({'case': 'textarea-ax-unicode-selection',
                                 **selection.expect(node, selected, start, length)})
        mac.key(0, flags=1 << 20)
        observations.append({'case': 'textarea-native-select-all',
                             **selection.expect(node, text, 0, utf16(text))})
        # A native backward selection must be observed as the correct OS range.
        selection.set(node, utf16(text), 0)
        mac.key(123, flags=1 << 17)  # Shift-Left over the final newline.
        observations.append({'case': 'textarea-native-backward-selection',
                             **selection.expect(node, '\n', utf16(text) - 1, 1)})
        selection.set(node, utf16(text), 0)
        observations.append({'case': 'textarea-end-caret',
                             **selection.expect(node, '', utf16(text), 0)})
    finally:
        mac.release(node)
    return observations


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--executable', type=Path)
    parser.add_argument('--source-only', action='store_true')
    args = parser.parse_args()
    Mac.require_accessibility()
    repo = Path(__file__).resolve().parent.parent
    executable = (args.executable or repo / '_build/default/examples/gallery/main.exe').resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    with executable.open('rb') as binary:
        digest = hashlib.file_digest(binary, 'sha256').hexdigest()
    report = {'platform': platform.platform(), 'executable': str(executable), 'sha256': digest,
              'source_only': args.source_only,
              'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
              'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=repo, text=True))}
    (args.output / 'executable.json').write_text(json.dumps(report, indent=2) + '\n')
    with preserved_clipboard() as board, (args.output / 'application.log').open('w') as log:
        child = subprocess.Popen([str(executable), '--trace-windows'], cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        report['pid'] = child.pid
        mac = None
        try:
            mac = Mac(child.pid, child)
            report['observations'] = exercise(mac, board, source_only=args.source_only)
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
            report['result'] = 'pass'
        except BaseException as error:
            report.update(result='fail', error=repr(error))
            raise
        finally:
            if mac:
                mac.release(mac.app)
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            report['child_exit'] = child.returncode
            (args.output / 'report.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')
    report['clipboard_restored'] = True
    (args.output / 'report.json').write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')
    scope = 'source' if args.source_only else 'source and editor'
    print(f'GPUIO_MACOS_TEXT_SELECTION_OK: {scope} AX ranges, Unicode, native selection, read-only copy, shutdown')


if __name__ == '__main__':
    main()
