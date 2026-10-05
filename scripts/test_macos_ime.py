#!/usr/bin/env python3
"""Real macOS Japanese IME acceptance in the public gallery.

Temporarily enables/selects installed Japanese Romaji/Hiragana, then restores the
original input sources. Requires an idle desktop and AX/screen-recording access.
Captures only the owned app's candidate window, never the entire desktop.
"""
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import plistlib
import signal
import subprocess
import sys
import time

from mac_input_source import Sources, foreground_keys, japanese_source
from test_agent_chat import Mac
from test_gallery import TITLE, expect_field, focus_gallery_control

LABEL = 'Settings workspace name'
COMPOSING = 'Finish composing text before resetting this field.'


def windows(mac):
    copy = mac.cg.CGWindowListCopyWindowInfo
    copy.restype, copy.argtypes = C.c_void_p, [C.c_uint, C.c_uint]
    encode = mac.cf.CFPropertyListCreateData
    encode.restype, encode.argtypes = C.c_void_p, [C.c_void_p, C.c_void_p, C.c_long, C.c_ulong, C.c_void_p]
    data_len = mac.cf.CFDataGetLength
    data_len.restype, data_len.argtypes = C.c_long, [C.c_void_p]
    data_bytes = mac.cf.CFDataGetBytePtr
    data_bytes.restype, data_bytes.argtypes = C.c_void_p, [C.c_void_p]
    array = copy(1, 0)
    if not array:
        raise RuntimeError('Cannot enumerate visible macOS windows')
    data = None
    try:
        data = encode(None, array, 100, 0, None)
        if not data:
            raise RuntimeError('Cannot encode macOS window metadata')
        return plistlib.loads(C.string_at(data_bytes(data), data_len(data)))
    finally:
        if data:
            mac.release(data)
        mac.release(array)


def composing(mac, stored):
    mac.wait_text(TITLE, COMPOSING)
    mac.wait_text(TITLE, 'Stored name: ' + stored + ' ·')


def committed(mac, expected):
    expect_field(mac, TITLE, LABEL, expected)
    mac.wait_text(TITLE, 'Stored name: ' + expected + ' ·')
    node = mac.find(TITLE, COMPOSING, 'AXStaticText')
    if node:
        mac.release(node)
        raise RuntimeError('Committed field still reports active composition')


def field_bounds(mac):
    class Pair(C.Structure):
        _fields_ = [('x', C.c_double), ('y', C.c_double)]
    get = mac.ax.AXValueGetValue
    get.restype, get.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
    node = mac.wait_find(TITLE, LABEL, 'AXTextField')
    result = []
    try:
        for name, kind in [('AXPosition', 1), ('AXSize', 2)]:
            value = mac.attr(node, name)
            pair = Pair()
            try:
                if not value or not get(value, kind, C.byref(pair)):
                    raise RuntimeError('Missing native editor geometry: ' + name)
                result.extend([pair.x, pair.y])
            finally:
                if value:
                    mac.release(value)
    finally:
        mac.release(node)
    return result


def capture_candidates(mac, output, initial_ids):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        candidates = [w for w in windows(mac)
                      if w.get('kCGWindowOwnerPID') == mac.pid
                      and w.get('kCGWindowLayer') == 20
                      and w['kCGWindowNumber'] not in initial_ids]
        if candidates:
            break
        time.sleep(.05)
    else:
        raise RuntimeError('No owned OS candidate window appeared after conversion')
    editor = field_bounds(mac)
    (output / 'candidate-windows.json').write_text(json.dumps(
        {'editor_bounds': editor, 'windows': candidates}, indent=2) + '\n')
    x, y, width, height = editor
    for i, candidate in enumerate(candidates):
        bounds = candidate['kCGWindowBounds']
        # Coarse physical placement check, not exact candidate/caret-pixel proof.
        if not (bounds['Width'] > 0 and bounds['Height'] > 0
                and x - 20 <= bounds['X'] <= x + width + 20
                and y - bounds['Height'] - 30 <= bounds['Y'] <= y + height + 80):
            raise RuntimeError(f'Candidate window is not near its editor: {bounds}, {editor}')
        subprocess.run(['screencapture', '-x', '-l', str(candidate['kCGWindowNumber']),
                        str(output / f'candidate-{i}.png')], check=True, timeout=10)
    print('IME_CANDIDATES_CAPTURED', len(candidates), flush=True)


def exercise(mac, output):
    mac.press(TITLE, 'Settings')
    focus_gallery_control(mac, LABEL, 'AXTextField')
    # Seed through the public AX editor API; independent of the original layout.
    mac.field(TITLE, LABEL, 'AXTextField', 'a')
    committed(mac, 'a')
    sources = Sources(mac)
    try:
        with japanese_source(sources, output):
            # Input-source selection and OS context activation are asynchronous.
            time.sleep(.5)
            foreground_keys(mac)
            focus_gallery_control(mac, LABEL, 'AXTextField')
            time.sleep(.3)
            initial_ids = {w['kCGWindowNumber'] for w in windows(mac)
                           if w.get('kCGWindowOwnerPID') == mac.pid and w.get('kCGWindowLayer') == 20}
            for code in (45, 34, 4, 31, 45, 5, 31):  # nihongo, physical US keycodes
                mac.key(code)
                time.sleep(.15)
            composing(mac, 'a')
            print('IME_PREEDIT', mac.field(TITLE, LABEL, 'AXTextField'), flush=True)
            for _ in range(2):
                mac.key(49)  # Space: conversion, then OS candidate list.
                time.sleep(.4)
            composing(mac, 'a')
            capture_candidates(mac, output, initial_ids)
            for _ in range(2):
                mac.key(36)  # Accept candidate, then commit remaining live-conversion segment.
                time.sleep(.4)
            value = mac.field(TITLE, LABEL, 'AXTextField')
            if not value or not value.startswith('a') or len(value) < 2 or not all(ord(c) > 127 for c in value[1:]):
                raise RuntimeError('Expected committed Japanese text, received ' + repr(value))
            committed(mac, value)
            print('IME_COMMITTED', value, flush=True)
            mac.key(6, flags=1 << 20)
            committed(mac, 'a')
            mac.key(6, flags=(1 << 20) | (1 << 17))
            committed(mac, value)
            print('IME_UNDO_REDO_OK', flush=True)
            for code in (45, 34):  # A fresh ni composition.
                mac.key(code)
                time.sleep(.15)
            composing(mac, value)
            for _ in range(3):
                mac.key(53)  # Revert conversion, clear preedit; bounded extra Escape is harmless.
                time.sleep(.15)
            committed(mac, value)
            print('IME_CANCEL_OK', flush=True)
    finally:
        sources.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--executable', type=Path)
    parser.add_argument('--restore', type=Path, help='Recover input sources from a prior original.json; opens no app')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('This test requires a real macOS desktop')
    Mac.require_accessibility()
    args.output.mkdir(parents=True, exist_ok=False)
    if args.restore:
        mac = Mac(os.getpid())
        sources = Sources(mac)
        try:
            sources.restore(json.loads(args.restore.read_text()), args.output)
        finally:
            sources.close()
            mac.release(mac.app)
        return
    repo = Path(__file__).resolve().parent.parent
    executable = args.executable.resolve() if args.executable else repo / '_build/default/examples/gallery/main.exe'
    with executable.open('rb') as binary:
        digest = hashlib.file_digest(binary, 'sha256').hexdigest()
    (args.output / 'executable.json').write_text(json.dumps(
        {'path': str(executable), 'sha256': digest}, indent=2) + '\n')
    with (args.output / 'application.log').open('w') as log:
        child = subprocess.Popen([str(executable)], cwd=repo, stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac, args.output)
            mac.close(TITLE)
            if child.wait(timeout=15):
                raise RuntimeError('Gallery failed to exit successfully')
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
    print('GPUIO_MACOS_JAPANESE_IME_OK: actual preedit/candidate/commit, undo/redo, cancel, source restoration, shutdown')


def interrupted(signum, frame):
    raise KeyboardInterrupt(f'Interrupted by signal {signum}; restoring input source')


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, interrupted)
    main()
