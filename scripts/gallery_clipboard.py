"""Actual macOS plain-text writes from the public gallery and copied feedback."""
import re
import time

from mac_clipboard import preserved_clipboard
from test_gallery import TITLE, focus_gallery_control


def exercise(mac, images=None):
    del images
    def expect(board, text):
        deadline = time.monotonic() + 5
        while board.text() != text:
            if time.monotonic() > deadline:
                raise RuntimeError('Native clipboard did not contain the expected test value')
            time.sleep(.025)
    with preserved_clipboard() as board:
        mac.press(TITLE, 'Images & icons')
        node = mac.wait_find(TITLE, 'Current approval:', 'AXStaticText', contains=True)
        try:
            values = [mac.text(node, name) or '' for name in ('AXValue', 'AXTitle', 'AXDescription')]
            count = next(int(match.group(1)) for value in values if (match := re.search(r'Current approval: (\d+)', value)))
        finally:
            mac.release(node)
        mac.press(TITLE, 'Copy literal')
        expect(board, 'Hello from GPUIO · λ 世界\n')
        mac.release(mac.wait_find(TITLE, 'Copied literal', 'AXButton'))
        mac.release(mac.wait_find(TITLE, 'Copy literal', 'AXButton'))
        # Real key events target only the owned PID, after native AX focus.
        focus_gallery_control(mac, 'Copy current approval', 'AXButton')
        mac.key(49)  # Space, native focused-button activation.
        expect(board, f'Approval {count} · λ 世界\n')
        mac.release(mac.wait_find(TITLE, 'Copied current approval', 'AXButton'))
        mac.press(TITLE, 'Advance approval value')
        mac.release(mac.wait_find(TITLE, 'Copy current approval', 'AXButton'))
        mac.press(TITLE, 'Copy current approval')
        expect(board, f'Approval {count + 1} · λ 世界\n')
        # Retiring the branch revokes feedback; returning starts idle.
        mac.press(TITLE, 'Presentation')
        mac.wait_text(TITLE, 'Ready when you are')
        mac.press(TITLE, 'Images & icons')
        mac.release(mac.wait_find(TITLE, 'Copy current approval', 'AXButton'))
        mac.press(TITLE, 'Copy literal')
        expect(board, 'Hello from GPUIO · λ 世界\n')
    print('GPUIO_GALLERY_CLIPBOARD_OK: literal/current Unicode text, native key/AX, timed feedback, branch retirement and original representations restored', flush=True)
