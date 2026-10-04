#!/usr/bin/env python3
"""Public OCaml canvas acceptance through a child app's macOS AX/keyboard APIs."""
import argparse
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import time
from test_agent_chat import Mac

TITLE = 'GPUIO · Canvas Lab'


def ready(mac, label, role='AXStaticText'):
    end = time.monotonic() + 30
    boolean = mac.cf.CFBooleanGetValue
    boolean.restype, boolean.argtypes = C.c_bool, [C.c_void_p]
    while time.monotonic() < end:
        if mac.child.poll() is not None:
            raise RuntimeError(f'Canvas app exited: {mac.child.returncode}')
        node = mac.find(TITLE, label, role)
        if node:
            value = mac.attr(node, 'AXEnabled')
            try:
                if value and boolean(value):
                    return node
            finally:
                if value:
                    mac.release(value)
            mac.release(node)
        time.sleep(0.05)
    mac.dump(TITLE)
    raise RuntimeError(f'Canvas object did not become ready: {label}')


def screenshot(mac, path, *, title=None):
    copy_windows = mac.cg.CGWindowListCopyWindowInfo
    copy_windows.restype, copy_windows.argtypes = C.c_void_p, [C.c_uint, C.c_uint]
    dictionary = mac.cf.CFDictionaryGetValue
    dictionary.restype, dictionary.argtypes = C.c_void_p, [C.c_void_p, C.c_void_p]
    get_number = mac.cf.CFNumberGetValue
    get_number.restype, get_number.argtypes = C.c_bool, [C.c_void_p, C.c_int, C.c_void_p]
    def number(info, name):
        key = mac.string(name)
        try:
            value = dictionary(info, key)
            result = C.c_longlong()
            if value and get_number(value, 4, C.byref(result)):
                return result.value
        finally:
            mac.release(key)
        return None
    def named(info):
        if title is None:
            return True
        key = mac.string('kCGWindowName')
        try:
            value = dictionary(info, key)
            buffer = C.create_string_buffer(4096)
            return (value and mac.type_id(value) == mac.string_type
                    and mac.get_string(value, buffer, len(buffer), 0x08000100)
                    and buffer.value.decode() == title)
        finally:
            mac.release(key)
    windows = copy_windows(1, 0)
    if not windows:
        raise RuntimeError('Cannot enumerate the child window for capture')
    try:
        for index in range(mac.count(windows)):
            info = mac.item(windows, index)
            if (number(info, 'kCGWindowOwnerPID') == mac.pid
                    and number(info, 'kCGWindowLayer') == 0 and named(info)):
                window_id = number(info, 'kCGWindowNumber')
                path.parent.mkdir(parents=True, exist_ok=True)
                subprocess.run(['screencapture', '-x', '-o', '-l', str(window_id), str(path)], check=True, timeout=10)
                return
    finally:
        mac.release(windows)
    raise RuntimeError('No owned canvas window found for capture')


def exercise(mac, image_path):
    node = ready(mac, 'Swift')
    try:
        mac.set(mac.app, 'AXFrontmost', mac.true)
        window = mac.window(TITLE)
        try:
            mac.perform(window, 'AXRaise')
        finally:
            mac.release(window)
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)
    mac.wait_text(TITLE, 'Swift · x 180.0 · y 320.0')
    mac.key(124, flags=1 << 17)  # Shift+Right, targeted to the child process.
    mac.wait_text(TITLE, 'Swift · x 181.0 · y 320.0')
    node = ready(mac, 'Activate Swift', 'AXButton')
    try:
        mac.perform(node, 'AXPress')
    finally:
        mac.release(node)
    mac.wait_text(TITLE, 'Activated: Swift')
    mac.press(TITLE, 'Hide plot')
    node = mac.wait_find(TITLE, 'Show plot', 'AXButton')
    mac.release(node)
    node = mac.find(TITLE, 'Swift', 'AXStaticText')
    if node:
        mac.release(node)
        raise RuntimeError('Hidden plot still exposes native objects')
    mac.press(TITLE, 'Show plot')
    node = ready(mac, 'Swift')
    mac.release(node)
    mac.press(TITLE, 'Reset dataset')
    mac.wait_text(TITLE, 'generation 2')
    node = ready(mac, 'Swift')
    try:
        mac.set(node, 'AXFocused', mac.true)
    finally:
        mac.release(node)
    mac.wait_text(TITLE, 'Swift · x 180.0 · y 320.0')
    if image_path:
        screenshot(mac, image_path)
    mac.press(TITLE, 'Close lab')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--screenshot', type=Path)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/canvas/main.exe'),
                                  '--trace-events'],
                                 env={**os.environ, 'GPUIO_TRACE_CANVAS': '1'},
                                 cwd=repo, stdout=log, stderr=subprocess.STDOUT, text=True)
        mac = None
        try:
            mac = Mac(child.pid, child)
            exercise(mac, args.screenshot)
            if child.wait(timeout=15) != 0:
                raise RuntimeError('Canvas app failed')
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
            log.seek(0)
            output = log.read()
            print(output, end='')
        if 'GPUIO_CANVAS_APP_RETURNED' not in output:
            raise RuntimeError('Missing application shutdown marker')
    print('GPUIO_CANVAS_PUBLIC_NATIVE_OK: native AX selection, targeted keyboard move accepted by OCaml, activation, hide/show, dataset reset and scoped shutdown')


if __name__ == '__main__':
    main()
