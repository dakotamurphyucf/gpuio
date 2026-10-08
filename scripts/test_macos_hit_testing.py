#!/usr/bin/env python3
"""Check actual system-wide AX point routing into gallery controls and overlays.

Requires an uncovered foreground macOS window. This checks semantic identity at
control centers, not pixels, VoiceOver speech, or character-level text geometry.
"""
import argparse
import ctypes as C
import hashlib
import json
from pathlib import Path
import platform
import subprocess

from test_agent_chat import Mac
from test_canvas import screenshot
from test_gallery import (TITLE, GalleryMouse, element_rect, open_picker, raise_gallery,
                          reveal_gallery_control)
from test_macos_window_lifecycle import prepare_window, ready_pointer


class PointRouting:
    def __init__(self, mac, observations):
        self.mac, self.observations = mac, observations
        self.equal = mac.cf.CFEqual
        self.equal.restype, self.equal.argtypes = C.c_bool, [C.c_void_p, C.c_void_p]
        create = mac.ax.AXUIElementCreateSystemWide
        create.restype, create.argtypes = C.c_void_p, []
        self.root = create()
        self.hit = mac.ax.AXUIElementCopyElementAtPosition
        self.hit.restype, self.hit.argtypes = C.c_int, [
            C.c_void_p, C.c_float, C.c_float, C.POINTER(C.c_void_p)]
        self.pid = mac.ax.AXUIElementGetPid
        self.pid.restype, self.pid.argtypes = C.c_int, [C.c_void_p, C.POINTER(C.c_int)]

    def close(self):
        self.mac.release(self.root)

    def check(self, label, role='AXButton', *, reveal=False):
        mac = self.mac
        if reveal:
            # Wheel in the card gutter so nested editors do not consume the
            # scroll intended to reveal a later control on the outer page.
            reveal_gallery_control(mac, label, role, scroll_in_left_gutter=True)
        observation = {'label': label, 'role': role, 'ancestry': []}
        self.observations.append(observation)
        # Establish foreground, stable geometry and absence of another process
        # over the point. This never compares semantic identity. The one-shot
        # identity measurement below must still reject a window/ancestor hit.
        ready_pointer(mac, GalleryMouse(mac), label, role, observation)
        target = mac.wait_find(TITLE, label, role)
        node = C.c_void_p()
        try:
            x, y, width, height = element_rect(mac, target)
            assert width > 0 and height > 0, (label, width, height)
            point = (x + width / 2, y + height / 2)
            observation.update(bounds=[x, y, width, height], point=point)
            status = self.hit(self.root, *point, C.byref(node))
            observation['hit_status'] = status
            assert status == 0 and node.value, observation
            pid = C.c_int()
            pid_status = self.pid(node, C.byref(pid))
            observation.update(pid_status=pid_status, pid=pid.value)
            assert pid_status == 0 and pid.value == mac.pid, observation
            # Rich controls can return a genuine label descendant. Walking UP
            # from the hit must reach the requested control: merely returning
            # its window/ancestor must fail, even when the PID is correct.
            for depth in range(16):
                observation['ancestry'].append({
                    key: (mac.text(node, attribute) or '')[:160]
                    for key, attribute in [('role', 'AXRole'), ('title', 'AXTitle'),
                                           ('description', 'AXDescription')]})
                if self.equal(node, target):
                    observation['match'] = 'exact' if depth == 0 else 'descendant'
                    print('AX_POINT_OK', label, observation['match'], flush=True)
                    return
                parent = mac.attr(node, 'AXParent')
                mac.release(node)
                node = C.c_void_p(parent)
                if not node.value:
                    break
            raise AssertionError(('Point did not resolve into target', observation))
        finally:
            if node.value:
                mac.release(node)
            mac.release(target)


def exercise(mac, report):
    mac.wait_text(TITLE, 'A little context goes a long way')
    raise_gallery(mac)
    prepare_window(mac, report)
    routing = PointRouting(mac, report['observations'])
    try:
        routing.check('New window')
        routing.check('Selection & actions')
        mac.press(TITLE, 'Selection & actions')
        routing.check('Pressed 0 times', reveal=True)
        mac.press(TITLE, 'Pressed 0 times')
        mac.wait_text(TITLE, 'Pressed 1 times')
        routing.check('Pressed 1 times', reveal=True)
        mac.press(TITLE, 'Text editing')
        routing.check('Document title', 'AXTextField', reveal=True)
        routing.check('Show validation error', 'AXCheckBox', reveal=True)
        mac.press(TITLE, 'Numbers & codes')
        routing.check('Preview level', 'AXSlider', reveal=True)
        mac.press(TITLE, 'Overlays & help')
        for trigger, close, extra in [
            ('Open dialog', 'Close dialog', 'Change backdrop'),
            ('Open drawer', 'Close drawer', None),
            ('Review confirmation', 'Keep preview', 'Confirm reset'),
            ('Show details', 'Done with details', None),
        ]:
            routing.check(trigger, reveal=True)
            open_picker(mac, trigger, close)
            routing.check(close)
            if extra:
                routing.check(extra)
            mac.press(TITLE, close)
            routing.check(trigger, reveal=True)
    finally:
        routing.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--executable', type=Path)
    args = parser.parse_args()
    Mac.require_accessibility()
    repo = Path(__file__).resolve().parent.parent
    executable = (args.executable or repo / '_build/default/examples/gallery/main.exe').resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    with executable.open('rb') as binary:
        digest = hashlib.file_digest(binary, 'sha256').hexdigest()
    report = {'platform': platform.platform(), 'executable': str(executable), 'sha256': digest,
              'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
              'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=repo, text=True)),
              'observations': []}
    with (args.output / 'application.log').open('w') as log:
        child = subprocess.Popen([str(executable), '--trace-windows'], cwd=repo,
                                 stdout=log, stderr=subprocess.STDOUT)
        report['pid'] = child.pid
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac, report)
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
            report['result'] = 'pass'
        except BaseException as error:
            report.update(result='fail', error=repr(error))
            if mac:
                try:
                    screenshot(mac, args.output / 'failure.png', title=TITLE)
                except Exception as capture_error:
                    report['capture_error'] = repr(capture_error)
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
    print('GPUIO_MACOS_HIT_TESTING_OK: controls, overlays, restored triggers, shutdown')


if __name__ == '__main__':
    main()
