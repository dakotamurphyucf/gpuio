#!/usr/bin/env python3
"""Bounded macOS startup diagnostics; no AX objects, input or window activation.

Launch a built executable directly (not a wrapper that spawns the actual app).
Capture only its PID's named, on-screen, normal windows. Completion means that
diagnostic artifacts were collected, not that startup or application UI passed.
"""
import argparse
import ctypes as C
import json
import math
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

from window_pixels import read_png


class WindowCapture:
    def __init__(self):
        self.cf = C.CDLL('/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation')
        self.cg = C.CDLL('/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics')

        def bind(lib, name, result, *args):
            function = getattr(lib, name)
            function.restype, function.argtypes = result, args
            return function

        ptr = C.c_void_p
        self.release = bind(self.cf, 'CFRelease', None, ptr)
        self.make_string = bind(self.cf, 'CFStringCreateWithCString', ptr, ptr, C.c_char_p, C.c_uint)
        self.get_string = bind(self.cf, 'CFStringGetCString', C.c_bool, ptr, ptr, C.c_long, C.c_uint)
        self.type_id = bind(self.cf, 'CFGetTypeID', C.c_ulong, ptr)
        self.string_type = bind(self.cf, 'CFStringGetTypeID', C.c_ulong)()
        self.count = bind(self.cf, 'CFArrayGetCount', C.c_long, ptr)
        self.item = bind(self.cf, 'CFArrayGetValueAtIndex', ptr, ptr, C.c_long)
        self.dictionary = bind(self.cf, 'CFDictionaryGetValue', ptr, ptr, ptr)
        self.number = bind(self.cf, 'CFNumberGetValue', C.c_bool, ptr, C.c_int, ptr)
        self.copy_windows = bind(self.cg, 'CGWindowListCopyWindowInfo', ptr, C.c_uint, C.c_uint)

    def value(self, info, key, *, text=False):
        key = self.make_string(None, key.encode(), 0x08000100)
        try:
            value = self.dictionary(info, key)
            if not value:
                return None
            if text:
                buffer = C.create_string_buffer(16385)
                if (self.type_id(value) == self.string_type
                        and self.get_string(value, buffer, len(buffer), 0x08000100)):
                    return buffer.value.decode()
            else:
                number = C.c_longlong()
                if self.number(value, 4, C.byref(number)):
                    return number.value
            return None
        finally:
            self.release(key)

    def owned_windows(self, pid, titles):
        # Option 1 enumerates on-screen windows. No other PID's name or pixels
        # enter the report. CoreGraphics may omit names if capture is unavailable.
        windows = self.copy_windows(1, 0)
        if not windows:
            raise RuntimeError('CoreGraphics could not enumerate on-screen windows')
        result = []
        try:
            for index in range(self.count(windows)):
                info = self.item(windows, index)
                if (self.value(info, 'kCGWindowOwnerPID') != pid
                        or self.value(info, 'kCGWindowLayer') != 0):
                    continue
                title = self.value(info, 'kCGWindowName', text=True)
                window_id = self.value(info, 'kCGWindowNumber')
                if title in titles and window_id is not None:
                    result.append((window_id, title))
        finally:
            self.release(windows)
        return result


def summarize_pixels(pixels):
    """Inspect the center half, excluding ordinary titlebars/window borders.

    Black is an observation, never a readiness oracle: a deliberately black UI
    is valid, while colored placeholders need not mean the app is ready.
    """
    if pixels.width < 4 or pixels.height < 4:
        raise ValueError('Capture too small to inspect its center')
    x0, x1 = pixels.width // 4, pixels.width * 3 // 4
    y0, y1 = pixels.height // 4, pixels.height * 3 // 4
    rows = b''.join(bytes(pixels.data[(y * pixels.width + x0) * 4:
                                    (y * pixels.width + x1) * 4])
                    for y in range(y0, y1))
    rgb_max = max(max(rows[channel::4]) for channel in range(3))
    alpha_min = min(rows[3::4])
    samples = {pixels.rgb(x, y)
               for y in range(y0, y1, max(1, (y1 - y0) // 32))
               for x in range(x0, x1, max(1, (x1 - x0) // 32))}
    return {'width_px': pixels.width, 'height_px': pixels.height,
            'region_px': [x0, y0, x1, y1], 'center_rgb_max': rgb_max,
            'center_alpha_min': alpha_min,
            'center_all_opaque_black': rgb_max == 0 and alpha_min == 255,
            'center_sampled_distinct_colors': len(samples)}


def stop_child(child):
    """Reap the child and stop descendants in this launch's own process group."""
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        child.wait(timeout=3)
    except subprocess.TimeoutExpired:
        pass
    # The leader may exit before its descendants. Kill the owned group even
    # after wait() succeeds; never use process-name matching or global pkill.
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    child.wait()


def collect(capture, child, args, report, started):
    seen = {}
    deadline = started + args.seconds
    frames = report['frames']
    while time.monotonic() < deadline and len(frames) < args.max_frames:
        if child.poll() is not None:
            report['stop_reason'] = 'child_exited'
            break
        for window_id, title in capture.owned_windows(child.pid, args.title):
            if len(frames) >= args.max_frames or time.monotonic() >= deadline:
                break
            first_seen = seen.setdefault(window_id, time.monotonic() - started)
            path = args.output / f'frame-{len(frames):04d}-window-{window_id}.png'
            frame = {'window_id': window_id, 'title': title,
                     'first_seen_seconds': first_seen,
                     'capture_started_seconds': time.monotonic() - started,
                     'path': path.name}
            frames.append(frame)
            # Start the deadline before Popen; capture subprocesses have their
            # own timeout and cannot silently extend the acquisition interval.
            subprocess.run(['screencapture', '-x', '-o', '-l', str(window_id), str(path)],
                           check=True, timeout=max(0.001, min(5, deadline - time.monotonic())))
            frame['capture_finished_seconds'] = time.monotonic() - started
        time.sleep(max(0, min(args.interval, deadline - time.monotonic())))
    else:
        report['stop_reason'] = ('frame_limit' if len(frames) >= args.max_frames
                                 else 'duration_limit')
    if report['stop_reason'] == 'child_exited':
        raise RuntimeError(f'Child exited during acquisition: {child.returncode}')
    missing = set(args.title) - {frame['title'] for frame in frames
                                if 'capture_finished_seconds' in frame}
    if missing:
        raise RuntimeError(f'No completed capture for requested titles: {sorted(missing)}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path, help='New artifact directory; never overwritten')
    parser.add_argument('--title', required=True, action='append', help='Exact owned window title; repeat for multiple windows')
    parser.add_argument('--seconds', type=float, default=5, help='Acquisition deadline, measured from before launch (0.5–60)')
    parser.add_argument('--interval', type=float, default=0.05, help='Delay after each enumeration/capture pass (0.01–1)')
    parser.add_argument('--max-frames', type=int, default=40, help='Total capture bound across windows (1–100)')
    parser.add_argument('command', nargs=argparse.REMAINDER, help='-- /absolute/built/app [arguments]')
    args = parser.parse_args()
    if args.command[:1] == ['--']:
        args.command = args.command[1:]
    if (not args.command or not math.isfinite(args.seconds) or not 0.5 <= args.seconds <= 60
            or not math.isfinite(args.interval) or not 0.01 <= args.interval <= 1
            or not 1 <= args.max_frames <= 100):
        parser.error('Provide a command and finite timing/frame bounds within the documented ranges')
    if sys.platform != 'darwin':
        parser.error('Window capture requires a real macOS desktop')
    # Load capture APIs before launching. No AX preflight, trust prompt, focus
    # change, keyboard event or app activation is performed by this tool.
    capture = WindowCapture()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    report = {'schema': 1, 'command': args.command, 'titles': args.title,
              'acquisition_seconds': args.seconds, 'interval_seconds': args.interval,
              'max_frames': args.max_frames, 'frames': [], 'status': 'incomplete',
              'scope': 'sampled OS window captures; not physical presentation latency or UI acceptance'}
    child = None

    def terminate(signum, _frame):
        # A supervising timeout's TERM must also pass through finally. Ignore
        # repeated TERM during the bounded cleanup, then restore the handler.
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        raise SystemExit(128 + signum)

    previous_term = signal.signal(signal.SIGTERM, terminate)
    try:
        # Test enumeration before opening an app. An unavailable desktop must
        # not leave the owner with a window this diagnostic cannot observe.
        capture.owned_windows(os.getpid(), set())
        with (args.output / 'child.log').open('w') as log:
            started = time.monotonic()
            child = subprocess.Popen(args.command, stdout=log, stderr=subprocess.STDOUT,
                                     start_new_session=True)
            report['child_pid'] = child.pid
            collect(capture, child, args, report, started)
            report['status'] = 'collected'
    except BaseException as error:
        report['error'] = f'{type(error).__name__}: {error}'
        raise
    finally:
        if child is not None:
            report['child_returncode_before_cleanup'] = child.poll()
            stop_child(child)
            report['child_returncode_after_cleanup'] = child.returncode
        try:
            # Decode after the child closes: analysis cannot delay acquisition
            # or keep a window open. Preserve partial captures on failure too.
            for frame in report['frames']:
                if 'capture_finished_seconds' in frame:
                    frame.update(summarize_pixels(read_png(capture, args.output / frame['path'])))
        except BaseException as error:
            report['analysis_error'] = f'{type(error).__name__}: {error}'
            report['status'] = 'incomplete'
            raise
        finally:
            (args.output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
            signal.signal(signal.SIGTERM, previous_term)
    print(f'GPUIO_STARTUP_CAPTURE_COLLECTED: {len(report["frames"])} samples; {args.output / "report.json"}')


if __name__ == '__main__':
    main()
