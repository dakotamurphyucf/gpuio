#!/usr/bin/env python3
"""Real gallery selection/Copy followed by close, with Rust backtraces enabled.

Regression for a deferred native selection callback reaching a retired document
during App shutdown. This does not qualify AX text ranges or VoiceOver behavior.
The owned child is always reaped before restoring the full captured clipboard.
"""
import argparse
import ctypes as C
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from mac_clipboard import preserved_clipboard
from mac_input_source import foreground_keys
from test_agent_chat import Mac
from test_gallery import TITLE, document_reading_order, expect_focus


def run(executable, log, report):
    result = {'binary_sha256': hashlib.sha256(executable.read_bytes()).hexdigest()}
    env = dict(os.environ, RUST_BACKTRACE='1', RUST_LIB_BACKTRACE='1')
    with preserved_clipboard() as board, log.open('w') as output:
        child = subprocess.Popen([str(executable), '--trace-windows'], env=env,
                                 stdout=output, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Mac(child.pid, child)
            mac.press(TITLE, 'Markdown & code')
            mac.wait_text(TITLE, 'Markdown preview')
            mac.wait_text(TITLE, 'A place for ideas')
            result['reading_order'] = document_reading_order(mac)
            node = mac.wait_find(TITLE, 'Document content', 'AXGroup')
            try:
                mac.set(node, 'AXFocused', mac.true)
                foreground_keys(mac)
                expect_focus(mac, 'Document content', 'AXGroup')
                mac.key(0, flags=1 << 20)  # Select All.
                time.sleep(.2)
                sequence = board.call(board.board, 'changeCount', result=C.c_long)
                mac.key(8, flags=1 << 20)  # Copy, leaving selection active at close.
                deadline = time.monotonic() + 5
                while board.call(board.board, 'changeCount', result=C.c_long) == sequence:
                    if time.monotonic() > deadline:
                        raise AssertionError('Copy did not update the pasteboard')
                    time.sleep(.025)
                copied = board.text()
                assert copied and 'A place for ideas' in copied
                assert '世界' in copied and 'let next_step' in copied
                assert '**Markdown**' not in copied
                result['copied_rendered_text'] = copied
            finally:
                mac.release(node)
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0, 'Gallery failed during native close'
            result['probe_complete'] = True
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
            result['child_exit'] = child.returncode
            report.write_text(json.dumps(result, indent=2, ensure_ascii=False) + '\n')
    result['clipboard_restored'] = True
    report.write_text(json.dumps(result, indent=2, ensure_ascii=False) + '\n')
    print('GPUIO_DOCUMENT_SELECTION_SHUTDOWN_OK: native Copy and normal close with Rust backtraces')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path,
                        default=Path('_build/default/examples/gallery/main.exe'))
    parser.add_argument('--log', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    run(args.executable.resolve(), args.log, args.report)
