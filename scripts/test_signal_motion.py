#!/usr/bin/env python3
"""Signal Studio native spring geometry, sequence events and shared-clock paint."""
import argparse
import json
from pathlib import Path
import re
import signal
import subprocess
import time
from test_signal_studio import Studio, TITLE
from test_agent_chat_combined import Combined
from test_canvas import screenshot


class Motion(Studio):
    def bounds(self, label, role='AXGroup'):
        node = self.wait_find(TITLE, label, role)
        try:
            return self.rect(node)
        finally:
            self.release(node)

    def heights(self, seconds, *, after_press=None):
        node = self.wait_find(TITLE, 'Signal history and inspector', 'AXGroup')
        values = []
        try:
            if after_press is not None:
                self.press(TITLE, after_press)
            deadline = time.monotonic() + seconds
            while time.monotonic() < deadline:
                values.append(self.rect(node)[3])
                time.sleep(.015)
        finally:
            self.release(node)
        return values

    def absent(self, label, role):
        node = self.find(TITLE, label, role)
        if node:
            self.release(node)
            raise AssertionError(f'Inactive content still exposed: {label}')

    def exercise(self, output, reduced):
        self.wait_log('SIGNAL_STUDIO: ready')
        self.set(self.app, 'AXFrontmost', self.true)
        self.resize(1160)
        time.sleep(.4)
        baseline = self.bounds('Signal history and inspector')[3]
        # Capture the transition before querying the rest of the AX tree:
        # those round trips can consume the whole spring on a hosted desktop.
        closing = self.heights(1.6, after_press='Hide inspector')
        self.wait_text(TITLE, 'Show inspector')
        self.absent('Lock control', 'AXButton')
        collapsed = closing[-1]
        assert abs(baseline - collapsed - 172) < 1, (baseline, closing)
        opening = self.heights(1.6, after_press='Show inspector')
        assert abs(opening[-1] - baseline) < 1, (baseline, opening)
        if not reduced:
            assert any(collapsed+1 < height < baseline-1 for height in opening), opening
            assert any(collapsed+1 < height < baseline-1 for height in closing), closing
        else:
            # AXPress queues an asynchronous application event; the old layout
            # can precede the accepted state. Reduced motion permits the two
            # endpoints, but must never traverse an intermediate height.
            assert all(min(abs(h-baseline), abs(h-collapsed)) < 1
                       for h in opening + closing), (opening, closing)
        self.press(TITLE, 'Hide inspector')
        self.heights(1.6)
        self.press(TITLE, 'Show inspector')
        early = self.heights(.08)
        self.press(TITLE, 'Hide inspector')
        reversed_values = self.heights(1.6)
        assert abs(reversed_values[-1] - collapsed) < 1, reversed_values
        self.absent('Lock control', 'AXButton')
        self.press(TITLE, 'Show inspector')
        self.heights(1.6)
        self.wait_text(TITLE, 'Lock control')
        before_run = len(self.log_path.read_text())
        self.press(TITLE, 'Increment counter, current value 0')
        self.wait_text(TITLE, 'Increment counter, current value 1')
        time.sleep(.45)
        observations = self.log_path.read_text()[before_run:]
        mode = 'Reduced_motion' if reduced else 'Played'
        run_events = '\n'.join(line for line in observations.splitlines() if 'motion run ' in line)
        assert re.findall(r'Stage_completed (\d+) (\w+)', run_events) == [('0', mode), ('1', mode)], run_events
        assert 'Finished' in run_events, run_events
        print(f'SIGNAL_SPRING_SEQUENCE_OK reduced={reduced} height_delta={baseline-collapsed:.2f} '
              f'intermediates={sum(collapsed+1 < h < baseline-1 for h in opening)} '
              f'interrupted_height={early[-1]:.2f}', flush=True)

        self.press(TITLE, 'Stream runs')
        self.wait_text(TITLE, 'Pause stream')
        window = self.window(TITLE)
        try:
            window_bounds = self.rect(window)
        finally:
            self.release(window)
        bounds = [self.bounds(label, 'AXStaticText') for label in ('Canvas activity', 'Chart activity')]
        values = []
        for index in range(8):
            path = output / f'activity-{index}.png'
            screenshot(self, path, title=TITLE)
            pair = []
            for rect in bounds:
                pixels = Combined.pixels(self, path, rect, window_bounds)
                # Both glyphs have the same RGB color and background. The top
                # green-channel percentile estimates text intensity while avoiding
                # dependence on subpixel text placement or surrounding chart paint.
                greens = sorted(pixels[1::4])
                pair.append(greens[int(len(greens)*.98)])
            values.append(pair)
            time.sleep(.075)
        assert all(abs(a-b) <= 8 for a, b in values), values
        for column in range(2):
            spread = max(row[column] for row in values) - min(row[column] for row in values)
            assert (spread <= 2 if reduced else spread >= 25), (reduced, values)
        self.wait_log('stream complete')
        self.wait_text(TITLE, 'Stream runs')
        # Playback pauses when the application stops streaming. It retains its
        # painted opacity; READY is not required to snap to a particular phase.
        time.sleep(.35)
        paused = []
        for index in range(2):
            path = output / f'paused-{index}.png'
            screenshot(self, path, title=TITLE)
            current = self.bounds('Canvas activity', 'AXStaticText')
            paused.append(Combined.pixels(self, path, current, window_bounds))
            time.sleep(.3)
        assert paused[0] == paused[1], 'Paused activity kept changing'
        (output / 'measurements.json').write_text(json.dumps({
            'reduced': reduced, 'baseline_height': baseline, 'collapsed_height': collapsed,
            'opening': opening, 'closing': closing, 'reversed': reversed_values,
            'shared_activity_intensity': values,
        }, indent=2) + '\n')
        print(f'SIGNAL_SHARED_CLOCK_PAINT_OK reduced={reduced} intensities={values}', flush=True)
        self.resize(650)
        self.wait_log('layout compact')
        self.wait_text(TITLE, 'Increment counter, current value 13')
        self.resize(1160)
        self.wait_log('layout wide', 2)
        self.wait_text(TITLE, 'Lock control')
        self.press(TITLE, 'Quit Studio')


def run(output, reduced):
    output = output.resolve() / ('reduce' if reduced else 'full')
    output.mkdir(parents=True, exist_ok=True)
    log_path = output / 'application.log'
    binary = Path('_build/default/examples/signal_studio/main.exe').resolve(strict=True)
    with log_path.open('w') as log:
        child = subprocess.Popen([str(binary), '--motion-check',
                                  '--reduced-motion' if reduced else '--full-motion'],
                                 stdout=log, stderr=subprocess.STDOUT)
        mac = None
        try:
            mac = Motion(child, log_path)
            mac.exercise(output, reduced)
            assert child.wait(timeout=15) == 0
        except BaseException:
            if mac and mac.has_window(TITLE):
                mac.dump(TITLE)
                screenshot(mac, output / 'failure.png', title=TITLE)
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('scratch/signal-motion'))
    args = parser.parse_args()
    def timeout(_signal, _frame):
        raise TimeoutError('Signal Studio motion walkthrough exceeded 180 seconds')
    signal.signal(signal.SIGALRM, timeout)
    signal.alarm(180)
    try:
        run(args.output, False)
        run(args.output, True)
    finally:
        signal.alarm(0)
    print('SIGNAL_MOTION_OK: full/reduced spring geometry/interruption, native sequence stages, '
          'synchronized painted activity, paused paint, responsive retention and cleanup', flush=True)


if __name__ == '__main__':
    main()
