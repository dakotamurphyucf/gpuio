#!/usr/bin/env python3
"""Public chart-family labels/legends via macOS AX, with background screenshots.

This checks real native text and window lifecycle, not keyboard data navigation.
"""
import argparse
from pathlib import Path
import subprocess
import time

from test_agent_chat import Mac
from test_canvas import screenshot

TITLE = 'GPUIO · Chart Studio'
FAMILIES = [
    ('Line', '48 values', ['Atlas', 'Nova']),
    ('Area', '24 values', ['Active capacity']),
    ('Bar', '24 values', ['Completed evaluations']),
    ('Pie', '4 values', ['Reasoning', 'Code', 'Research', 'Other']),
    ('Radar', '5 values', ['Quality', 'Speed', 'Cost', 'Context', 'Reliability', 'Atlas']),
    ('Candlestick', '24 values', ['Rise · hollow', 'Fall · filled']),
    ('Sankey', '4 values', ['Incoming', 'Reasoning', 'Tools', 'Complete']),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--foreground', action='store_true', help='Activate the child when background windows do not receive frames')
    parser.add_argument('--app', default='_build/default/examples/charts/main.exe')
    parser.add_argument('--output', type=Path, default=Path('.cache/chart-text-acceptance'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    with (args.output / 'application.log').open('w') as log:
        child = subprocess.Popen([args.app, *([] if args.foreground else ['--background'])], stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Mac(child.pid, child)
            for family, status, labels in FAMILIES:
                if family != 'Line':
                    mac.press(TITLE, family)
                mac.wait_text(TITLE, status)
                mac.wait_text(TITLE, 'Chart legend')
                for label in labels:
                    mac.wait_text(TITLE, label)
                if family in ['Line', 'Area', 'Bar', 'Candlestick']:
                    for tick in ['0', '23']:
                        node = mac.wait_find(TITLE, tick, 'AXStaticText')
                        mac.release(node)
                time.sleep(0.1)
                screenshot(mac, args.output / f'{family.lower()}.png', title=TITLE)
                mac.press(TITLE, 'Update data ↗')
                time.sleep(0.15)
                mac.wait_text(TITLE, status)
                for label in labels:
                    mac.wait_text(TITLE, label)
            mac.close(TITLE)
            assert child.wait(timeout=15) == 0
            print('CHART_TEXT_OK: seven families, native labels/legends, update, OS close', flush=True)
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


if __name__ == '__main__':
    main()
