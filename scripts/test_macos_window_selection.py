#!/usr/bin/env python3
"""Exercise read-only window selection through native pointer/keyboard input.

VoiceOver and OS settings are untouched. Only the clipboard change counter is
observed; clipboard payloads are neither read nor written.
"""
import argparse
import ctypes as C
import json
from pathlib import Path
import platform
import signal
import subprocess
import time

from mac_clipboard import Pasteboard
from mac_input_source import foreground_keys
from package_macos_reference import digest
from test_agent_chat import Mac
from test_canvas import screenshot
from test_gallery import (TITLE, SECOND, GalleryMouse, element_rect,
                          raise_gallery, reveal_gallery_control)

LINES = ['A quiet workspace for your next idea.',
         'Unicode stays whole: café · 京都 · 👩‍💻',
         'Selections can span several text nodes.']
MODIFIERS = (1 << 20) | (1 << 17)


def quoted_ocaml(text):
    """The gallery displays UTF-8 via OCaml %S, including decimal byte escapes."""
    escapes = {34: '\\"', 92: '\\\\', 10: '\\n', 13: '\\r', 9: '\\t', 8: '\\b'}
    return '"' + ''.join(escapes.get(byte, chr(byte) if 32 <= byte < 127
                                    else f'\\{byte:03d}')
                         for byte in text.encode('utf-8')) + '"'


def shortcut(mac, key, expected, title=TITLE):
    mac.key(key, flags=MODIFIERS)
    mac.wait_text(title, expected)


def inspect(mac, expected, title=TITLE):
    # Change the visible acknowledgement before querying, so an old identical
    # result cannot satisfy the wait while a new request is still in flight.
    shortcut(mac, 16, 'Selection is present.' if expected else 'No selection is present.', title)
    label = (f'Selected {len(expected.encode("utf-8"))} UTF-8 bytes: '
             + quoted_ocaml(expected)) if expected else 'No text is selected.'
    shortcut(mac, 32, label, title)  # U.


def bounds(mac, label):
    node = mac.wait_find(TITLE, label, 'AXStaticText')
    try:
        return element_rect(mac, node)
    finally:
        mac.release(node)


def focus_card(mac, title):
    # Shortcuts belong to this card's registry, not the whole application.
    # Read-only pointer selection supplies this focus in the populated case.
    node = mac.wait_find(title, 'Check selection', 'AXButton')
    try:
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)
    time.sleep(.15)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/gallery/main.exe'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    Mac.require_accessibility()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = {'complete': False, 'platform': platform.platform(), 'checks': [],
              'executable_sha256': digest(args.executable),
              'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
              'dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], text=True))}
    child = mac = board = mouse = None
    down = False
    pointer = (0, 0)
    def interrupted(signum, _frame):
        raise RuntimeError(f'Window selection test interrupted: {signum}')
    signals = (signal.SIGTERM, signal.SIGALRM)
    handlers = {sig: signal.signal(sig, interrupted) for sig in signals}
    signal.alarm(150)
    try:
        board = Pasteboard()
        before = board.call(board.board, 'changeCount', result=C.c_long)
        with (output / 'application.log').open('w') as log:
            child = subprocess.Popen([str(args.executable.resolve())], stdout=log,
                                     stderr=subprocess.STDOUT)
            mac = Mac(child.pid, child)
            foreground_keys(mac)
            mac.wait_text(TITLE, 'A little context goes a long way')
            mac.press(TITLE, 'Runtime & windows')
            reveal_gallery_control(mac, LINES[2], 'AXStaticText')
            mouse = GalleryMouse(mac)
            focus_card(mac, TITLE)
            shortcut(mac, 16, 'No selection is present.')
            inspect(mac, '')
            first, second, third = [bounds(mac, line) for line in LINES]
            start = (first[0] + .25, first[1] + first[3] / 2)
            end = (second[0] + second[2] - .25, second[1] + second[3] / 2)
            mouse.check_owner(start)
            mouse.check_owner(end)
            pointer = start
            mouse.send(5, start)
            mouse.send(1, start)
            down = True
            time.sleep(.1)
            for step in range(1, 9):
                pointer = tuple(a + (b-a) * step/8 for a, b in zip(start, end))
                mouse.send(6, pointer)
                time.sleep(.03)
            expected = '\n'.join(LINES[:2])
            inspect(mac, expected)
            shortcut(mac, 16, 'Selection is present.')
            report['checks'].append({'case': 'cross-node-exact-unicode-read',
                                     'utf8_bytes': len(expected.encode('utf-8'))})

            # End the selection in the application while the physical button is
            # still held, then move over another selectable line. The old range
            # must remain unchanged despite the subsequent native drag event.
            shortcut(mac, 14, 'Drag ended; selection preserved.')  # E.
            pointer = (third[0] + third[2] - .25, third[1] + third[3] / 2)
            mouse.check_owner(pointer)
            mouse.send(6, pointer)
            time.sleep(.15)
            inspect(mac, expected)
            mouse.send(2, pointer)
            down = False
            inspect(mac, expected)
            report['checks'].append({'case': 'end-drag-retains-range-after-native-motion-and-up'})
            screenshot(mac, output / 'selected-unicode.png', title=TITLE)

            # Opening and querying another window must not consume the first
            # window's registered selection.
            mac.press(TITLE, 'New window')
            mac.wait_text(SECOND, 'A little context goes a long way')
            mac.press(SECOND, 'Runtime & windows')
            focus_card(mac, SECOND)
            shortcut(mac, 16, 'No selection is present.', SECOND)
            inspect(mac, '', SECOND)
            mac.close(SECOND)
            raise_gallery(mac)
            time.sleep(.15)
            inspect(mac, expected)
            report['checks'].append({'case': 'window-isolation-and-close'})

            shortcut(mac, 40, 'Selection cleared.')  # K.
            shortcut(mac, 16, 'No selection is present.')
            inspect(mac, '')
            report['checks'].append({'case': 'clear-removes-selection'})
            # Leave an active range behind when unmounting the native page.
            # Bonsai retains its notice model; native selection must retire.
            reveal_gallery_control(mac, LINES[2], 'AXStaticText')
            x, y, w, h = bounds(mac, LINES[2])
            pointer = (x + .25, y + h / 2)
            mouse.check_owner(pointer)
            mouse.send(5, pointer)
            mouse.send(1, pointer)
            down = True
            time.sleep(.1)
            pointer = (x + w - .25, y + h / 2)
            mouse.check_owner(pointer)
            mouse.send(6, pointer)
            time.sleep(.1)
            mouse.send(2, pointer)
            down = False
            inspect(mac, LINES[2])
            mac.press(TITLE, 'Presentation')
            mac.wait_text(TITLE, 'A little context goes a long way')
            mac.press(TITLE, 'Runtime & windows')
            mac.wait_text(TITLE, 'Selection without the clipboard')
            focus_card(mac, TITLE)
            shortcut(mac, 16, 'No selection is present.')
            inspect(mac, '')
            report['checks'].append({'case': 'page-remount-without-stale-selection'})
            assert board.call(board.board, 'changeCount', result=C.c_long) == before, \
                'Clipboard changed during window selection test'
            report['clipboard_change_count_unchanged'] = True
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
            report['complete'] = True
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        signal.alarm(0)
        for sig, handler in handlers.items():
            signal.signal(sig, handler)
        if down and mouse:
            mouse.send(2, pointer)
        if child and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
        if mac:
            mac.release(mac.app)
        if board:
            board.close()
        report['child_reaped'] = child is None or child.poll() is not None
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print('GPUIO_WINDOW_SELECTION_OK: exact Unicode read, end/clear, windows and remount')


if __name__ == '__main__':
    main()
