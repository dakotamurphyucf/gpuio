#!/usr/bin/env python3
"""Signal Studio's real macOS run alerts, current-window routing and cleanup.

Explicitly requests permission only for com.gpuio.signal-studio. Never reads or
acts on another application's notification contents. Existing denial requires
allowing this test identity in System Settings; global settings stay unchanged.
"""
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import time
from test_signal_studio import Studio, TITLE
from test_signal_desktop import pids_for
from test_notifications_macos import Notifications
from test_canvas import screenshot

APP_NAME = 'GPUIO Signal Studio'
REGISTRY = '/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister'


def title(run):
    return f'Signal Studio · Run {run:02d} complete'


def exercise(output):
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    binary = Path('_build/default/examples/signal_studio/main.exe').resolve(strict=True)
    with (output / 'unavailable.log').open('w') as log:
        subprocess.run([str(binary), '--notification-unavailable-check'],
                       stdout=log, stderr=subprocess.STDOUT, check=True, timeout=30)
    assert 'unbundled alerts fallback passed' in (output / 'unavailable.log').read_text()
    bundle = output / f'{APP_NAME}.app'
    executable = bundle / 'Contents/MacOS/gpuio-signal'
    executable.parent.mkdir(parents=True, exist_ok=True)
    assert not pids_for(executable), 'Previous owned test process is still live'
    executable.unlink(missing_ok=True)
    shutil.copy2(binary, executable)
    metadata = subprocess.check_output([str(binary), '--print-info-plist'], timeout=10)
    assert plistlib.loads(metadata)['CFBundleIdentifier'] == 'com.gpuio.signal-studio'
    (bundle / 'Contents/Info.plist').write_bytes(metadata)
    subprocess.run(['/usr/bin/codesign', '--force', '--sign', '-', '--identifier',
                    'com.gpuio.signal-studio', '--timestamp=none', str(bundle)], check=True, timeout=20)
    subprocess.run([REGISTRY, '-f', str(bundle)], check=True, timeout=10)
    log = output / 'application.log'
    log.write_text('')
    proxy = subprocess.Popen(['/usr/bin/open', '-W', '-n', '-g', '-a', str(bundle),
                              '--stdout', str(log), '--stderr', str(log)])
    mac = None
    notifications = None

    def wait(marker, count=1):
        end = time.monotonic() + 20
        while time.monotonic() < end:
            if log.read_text().count(marker) >= count:
                return
            if proxy.poll() is not None:
                break
            time.sleep(.05)
        raise RuntimeError(f'Missing {marker}\n{log.read_text()}')

    def alerts():
        mac.set(mac.app, 'AXFrontmost', mac.true)
        found = mac.find(TITLE, 'Close alerts', 'AXButton')
        if found:
            mac.release(found)
        else:
            mac.press(TITLE, 'Alerts')
        mac.wait_text(TITLE, 'Keep track of your runs')

    def close_alerts():
        found = mac.find(TITLE, 'Close alerts', 'AXButton')
        if found:
            try:
                mac.perform(found, 'AXPress')
            finally:
                mac.release(found)

    def absent_notification(run):
        end = time.monotonic() + 10
        while time.monotonic() < end:
            node = notifications.find(title(run))
            if node is None:
                return
            notifications.mac.release(node)
            time.sleep(.05)
        raise AssertionError('Owned notification remains visible after cleanup')

    try:
        wait('alert authorization (Ok')
        owned = pids_for(executable)
        assert len(owned) == 1, owned
        mac = Studio(proxy, log, pid=owned[0])
        notifications = Notifications(APP_NAME)
        mac.resize(1160)
        alerts()
        if 'alert authorization (Ok Denied)' in log.read_text():
            raise AssertionError('Allow notifications only for GPUIO Signal Studio in System Settings, then rerun.')
        undetermined = 'alert authorization (Ok Not_determined)' in log.read_text()
        mac.press(TITLE, 'Enable alerts')
        if undetermined:
            notifications.act(f'“{APP_NAME}” Notifications', 'Allow')
        wait('Run completion alerts are enabled.')
        alerts()
        screenshot(mac, output / 'alerts-enabled.png', title=TITLE)
        mac.press(TITLE, 'Notify current run')
        wait('Run 00 notification submitted.')
        notifications.act(title(0), 'Open workspace')
        wait('notification workspace activated')
        print('PASS explicit permission and actual named action', flush=True)

        close_alerts()
        mac.press(TITLE, 'Increment counter, current value 0')
        mac.wait_text(TITLE, 'Increment counter, current value 1')
        alerts()
        mac.press(TITLE, 'Notify current run')
        wait('Run 01 notification submitted.')
        node = notifications.wait(title(1))
        notifications.mac.release(node)
        close_alerts()
        mac.close(TITLE)
        end = time.monotonic() + 8
        while mac.has_window(TITLE) and time.monotonic() < end:
            time.sleep(.05)
        assert not mac.has_window(TITLE)
        notifications.act(title(1), 'AXPress')
        wait('notification workspace activated', 2)
        wait('window opened', 2)
        mac.wait_text(TITLE, 'Increment counter, current value 1')
        assert pids_for(executable) == owned
        print('PASS default action opens current model after old window closes', flush=True)

        mac.press(TITLE, 'Increment counter, current value 1')
        mac.wait_text(TITLE, 'Increment counter, current value 2')
        alerts()
        mac.press(TITLE, 'Notify current run')
        wait('Run 02 notification submitted.')
        node = notifications.wait(title(2))
        notifications.mac.release(node)
        close_alerts()
        mac.press(TITLE, 'Increment counter, current value 2')
        mac.wait_text(TITLE, 'Increment counter, current value 3')
        alerts()
        mac.press(TITLE, 'Notify current run')
        wait('Run 03 notification updated.')
        notifications.open_center()
        node = notifications.wait(title(3))
        notifications.mac.release(node)
        close_alerts()
        mac.press(TITLE, 'Stream runs')
        wait('stream complete')
        wait('Run 15 notification updated.')
        node = notifications.wait(title(15))
        notifications.mac.release(node)
        print('PASS same-notification replacement and actual stream completion alert', flush=True)

        alerts()
        notifications.open_center()
        node = notifications.wait(title(15))
        notifications.mac.release(node)
        mac.press(TITLE, 'Dismiss alert')
        wait('Notification dismissal requested.')
        absent_notification(15)
        mac.press(TITLE, 'Notify current run')
        wait('Run 15 notification submitted.')
        node = notifications.wait(title(15))
        notifications.mac.release(node)
        close_alerts()
        notifications.open_center()
        node = notifications.wait(title(15))
        notifications.mac.release(node)
        mac.press(TITLE, 'Quit Studio')
        assert proxy.wait(timeout=15) == 0
        assert not pids_for(executable)
        absent_notification(15)
        print('SIGNAL_NOTIFICATIONS_OK: unbundled fallback, explicit permission, real named/default actions, current-window routing, latest-run replacement/stream, dismiss and application cleanup', flush=True)
    finally:
        if notifications:
            notifications.close()
        if mac:
            mac.release(mac.app)
        for pid in pids_for(executable):
            try:
                os.kill(pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        try:
            proxy.wait(timeout=5)
        except subprocess.TimeoutExpired:
            for pid in pids_for(executable):
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            proxy.kill()
            proxy.wait()
        subprocess.run([REGISTRY, '-u', str(bundle)], check=True, timeout=10)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('scratch/signal-notifications'))
    args = parser.parse_args()
    def timeout(_signal, _frame):
        raise TimeoutError('Signal Studio notifications exceeded 180 seconds')
    signal.signal(signal.SIGALRM, timeout)
    signal.alarm(180)
    try:
        exercise(args.output)
    finally:
        signal.alarm(0)


if __name__ == '__main__':
    main()
