#!/usr/bin/env python3
"""Actual streamed Markdown must not bounce preceding transcript rows."""
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat_presentation import Presentation
from test_agent_chat_review import TITLE, CONVERSATION


def exercise(mac):
    mac.release(mac.wait_find(TITLE, 'Explore workspace', 'AXButton'))
    mac.set(mac.app, 'AXFrontmost', mac.true)
    mac.press(TITLE, 'Demo controls')
    mac.press(TITLE, 'Slow stream')
    mac.draft(TITLE, CONVERSATION, 'Check streaming geometry')
    row = mac.wait_find(TITLE, 'Proposed patch', 'AXGroup')
    composer = mac.wait_find(TITLE, 'Message · ' + CONVERSATION, 'AXTextArea')
    samples = []
    try:
        mac.press(TITLE, 'Send')
        started = time.monotonic()
        while time.monotonic() - started < 8:
            try:
                position = mac.rect(row)
                draft = mac.rect(composer)
            except RuntimeError:
                # The preceding card may leave the bounded warm row set as the
                # response grows. AX disappearance of the window is not eviction.
                window = mac.window(TITLE)
                if not window:
                    raise RuntimeError('Chat window disappeared during streaming')
                mac.release(window)
                current = mac.find(TITLE, 'Proposed patch', 'AXGroup')
                if current:
                    mac.release(current)
                    raise
                break
            samples.append((time.monotonic() - started, position[1], draft[1]))
            time.sleep(.01)
        assert len(samples) >= 8 and samples[-1][0] >= .75, samples
        deltas = [b[1] - a[1] for a, b in zip(samples, samples[1:])]
        # This deterministic fixture at the default window size grows through
        # several lines before the preceding card leaves the viewport. The old
        # per-parse notice repeatedly shifted it down/up by 29 logical pixels.
        assert sum(delta < -1 for delta in deltas) >= 3, deltas
        downward = [delta for delta in deltas if delta > 1]
        assert not downward, ('Unexpected downward transcript steps', downward, samples)
        assert max(s[2] for s in samples) - min(s[2] for s in samples) <= .5
        print(f'CHAT_STREAM_GEOMETRY_OK samples={len(samples)} '
              f'observed_seconds={samples[-1][0]:.2f} downward_steps=0 '
              f'growth_steps={sum(delta < -1 for delta in deltas)} composer_stable=True', flush=True)
    finally:
        mac.release(row)
        mac.release(composer)
    mac.close(TITLE)


def main():
    if sys.platform != 'darwin':
        raise RuntimeError('This walkthrough requires macOS AppKit')
    def timeout(_signal, _frame):
        raise TimeoutError('Streaming layout check exceeded 60 seconds')
    signal.signal(signal.SIGALRM, timeout)
    repo = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryFile(mode='w+t') as log:
        child = subprocess.Popen([str(repo / '_build/default/examples/agent_chat/main.exe'),
                                  '--full-motion'], cwd=repo,
                                 stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            signal.alarm(60)
            mac = Presentation(child.pid, child)
            exercise(mac)
            if child.wait(timeout=15):
                raise RuntimeError('Chat exited unsuccessfully')
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
    print('GPUIO_CHAT_STREAM_LAYOUT_APPKIT_OK: real streamed Markdown, stable preceding '
          'row direction and composer, native window/process cleanup', flush=True)


if __name__ == '__main__':
    main()
