#!/usr/bin/env python3
"""Real macOS AX range geometry and the public OCaml range query.

Source/selection are injected through AX; this is not physical typing, IME or
VoiceOver qualification. Owns and reaps one gallery child, changes no OS settings.
"""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import platform
import re
import subprocess

from test_agent_chat import Mac
from test_gallery import TITLE, element_rect, expect_field, focus_gallery_control
from test_macos_text_selection import Range, Selection, utf16


class Geometry(Selection):
    def range_for_line(self, node, line):
        create = self.mac.cf.CFNumberCreate
        create.restype, create.argtypes = C.c_void_p, [C.c_void_p, C.c_int, C.c_void_p]
        parameter = create(None, 4, C.byref(C.c_longlong(line)))
        attribute = self.mac.string('AXRangeForLine')
        result = C.c_void_p()
        copy = self.mac.ax.AXUIElementCopyParameterizedAttributeValue
        copy.restype, copy.argtypes = C.c_int, [C.c_void_p, C.c_void_p, C.c_void_p,
                                              C.POINTER(C.c_void_p)]
        try:
            status = copy(node, attribute, parameter, C.byref(result))
            assert status == 0 and result.value, ('AXRangeForLine', status)
            value = Range()
            assert self.get(result, 4, C.byref(value)), 'Expected CFRange'
            return [value.location, value.length]
        finally:
            if result.value:
                self.mac.release(result)
            self.mac.release(attribute)
            self.mac.release(parameter)

    def bounds(self, node, location, length):
        parameter = self.create(4, C.byref(Range(location, length)))
        attribute = self.mac.string('AXBoundsForRange')
        result = C.c_void_p()
        copy = self.mac.ax.AXUIElementCopyParameterizedAttributeValue
        copy.restype, copy.argtypes = C.c_int, [C.c_void_p, C.c_void_p, C.c_void_p,
                                              C.POINTER(C.c_void_p)]
        try:
            status = copy(node, attribute, parameter, C.byref(result))
            assert status == 0 and result.value, ('AXBoundsForRange', status)
            rect = (C.c_double * 4)()
            assert self.get(result, 3, C.byref(rect)), 'Expected CGRect'
            return list(rect)
        finally:
            if result.value:
                self.mac.release(result)
            self.mac.release(attribute)
            self.mac.release(parameter)


def content_origin(mac):
    window = mac.window(TITLE)
    children = mac.children(window)
    try:
        groups = [node for node in children if mac.text(node, 'AXRole') == 'AXGroup']
        assert len(groups) == 1, 'Expected one window content group'
        return element_rect(mac, groups[0])[:2]
    finally:
        for node in children:
            mac.release(node)
        mac.release(window)


def public_bounds(mac):
    # Clear any equal prior result, then wait for this asynchronous query chain.
    mac.press(TITLE, 'Inspect view')
    mac.wait_text(TITLE, 'Layout covers lines')
    mac.press(TITLE, 'Inspect selection bounds')
    mac.wait_text(TITLE, 'Selection bounds')
    node = mac.wait_find(TITLE, 'Selection bounds', contains=True)
    try:
        label = mac.text(node, 'AXTitle') or mac.text(node, 'AXValue')
        match = re.search(r'x ([\d.-]+) · y ([\d.-]+) · ([\d.-]+) × ([\d.-]+) px · revision (\d+)', label)
        assert match, label
        values = list(map(float, match.groups()[:4]))
        origin = content_origin(mac)
        values[0] += origin[0]
        values[1] += origin[1]
        return values, int(match.group(5))
    finally:
        mac.release(node)


def union(a, b):
    x, y = min(a[0], b[0]), min(a[1], b[1])
    return [x, y, max(a[0] + a[2], b[0] + b[2]) - x,
            max(a[1] + a[3], b[1] + b[3]) - y]


def exercise(mac, report):
    mac.press(TITLE, 'Text editing')
    mac.press(TITLE, 'Multiline & search')
    focus_gallery_control(mac, 'Working notes', 'AXTextArea')
    geometry = Geometry(mac)
    for name, text in [
        ('intermediate_wide_line', 'a\nMMMMMMMMMMMM\nz'),
        ('soft_wrap', 'alpha beta gamma delta epsilon ' * 8),
        ('empty', ''),
        ('blank_lines', '\n\n'),
        ('crlf', 'one\r\ntwo\r\n'),
        ('clusters', 'e\u0301 👨\u200d👩\u200d👧\u200d👦 日本語'),
        ('right_to_left', 'אבגד'),
        ('mixed_direction', 'English אבגד tail'),
    ]:
        mac.field(TITLE, 'Working notes', 'AXTextArea', text)
        expect_field(mac, TITLE, 'Working notes', text, 'AXTextArea')
        node = mac.wait_find(TITLE, 'Working notes', 'AXTextArea')
        entry = {'case': name, 'source': text}
        report['cases'].append(entry)
        try:
            geometry.set(node, 0, utf16(text))
            geometry.expect(node, text, 0, utf16(text))
            # AX text actions are asynchronous. The public query also serves as
            # a source-revision/layout readiness barrier for this operation.
            public, revision = public_bounds(mac)
            actual = geometry.bounds(node, 0, utf16(text))
            entry.update(public_screen_bounds=public, revision=revision,
                         ax_screen_bounds=actual)
            assert actual[3] > 0 and actual[2] >= 0, entry
            assert (actual[2] > 0) == bool(text.strip('\r\n')), entry
            if name in ('intermediate_wide_line', 'soft_wrap', 'empty', 'blank_lines', 'crlf'):
                # The public contract includes endpoint carets. AccessKit's
                # nonempty text range excludes the final empty line after LF.
                end_caret = geometry.bounds(node, utf16(text), 0)
                combined = union(actual, end_caret)
                entry.update(end_caret=end_caret, ax_with_end_caret=combined)
                assert all(abs(a - b) < .2 for a, b in zip(combined, public)), entry
            if name == 'right_to_left':
                characters = [geometry.bounds(node, i, 1) for i in range(4)]
                entry['characters'] = characters
                assert all(rect[2] > 0 and rect[3] > 0 for rect in characters), entry
                assert all(a[0] > b[0] for a, b in zip(characters, characters[1:])), entry
            if name == 'clusters':
                accent = [geometry.bounds(node, i, 1) for i in (0, 1)]
                entry['combining_cluster'] = accent
                assert accent[0] == accent[1] and accent[0][2] > 0, entry
                family = '👨\u200d👩\u200d👧\u200d👦'
                whole = geometry.bounds(node, utf16('e\u0301 '), utf16(family))
                position = utf16('e\u0301 ')
                parts = []
                for character in family:
                    parts.append(geometry.bounds(node, position, utf16(character)))
                    position += utf16(character)
                entry['family_cluster'] = {'whole': whole, 'scalars': parts}
                assert whole[2] > 0 and all(part == whole for part in parts), entry
            if name == 'mixed_direction':
                entry['first_line'] = geometry.range_for_line(node, 0)
                assert entry['first_line'] == [0, utf16(text)], entry
                characters = [geometry.bounds(node, utf16('English ') + i, 1) for i in range(4)]
                entry['rtl_characters'] = characters
                assert all(a[0] > b[0] and a[2] > 0 for a, b in zip(characters, characters[1:])), entry
            assert mac.text(node, 'AXValue') == text, 'Geometry must not edit text'
            assert geometry.read(node) == [0, utf16(text)], 'Geometry must not change selection'
            entry['result'] = 'pass'
            print(f'PASS {name}: AX={actual}, public={public}', flush=True)
        finally:
            mac.release(node)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, default=Path('_build/default/examples/gallery/main.exe'))
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    executable = args.executable.resolve()
    args.report.parent.mkdir(parents=True, exist_ok=True)
    report = {'platform': platform.platform(), 'executable': str(executable),
              'binary_sha256': hashlib.sha256(executable.read_bytes()).hexdigest(),
              'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], text=True)),
              'input': 'AX value/selection/actions; no keyboard/IME/VoiceOver claim',
              'cases': []}
    mac = None
    with args.report.with_suffix('.app.log').open('w') as log:
        child = subprocess.Popen([str(executable), '--trace-windows'], stdout=log, stderr=subprocess.STDOUT)
        report['pid'] = child.pid
        try:
            mac = Mac(child.pid, child)
            exercise(mac, report)
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
            args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')


if __name__ == '__main__':
    main()
