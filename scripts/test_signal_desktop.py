#!/usr/bin/env python3
"""Signal Studio packaged links/reopen and native picker/Eio document round trip."""
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import tempfile
import time
from test_signal_studio import Studio, TITLE
from test_canvas import screenshot
from test_agent_chat import Mac


def pids_for(executable):
    output = subprocess.check_output(['ps', '-axo', 'pid=,command='], text=True)
    matches = []
    for line in output.splitlines():
        parts = line.strip().split(maxsplit=1)
        if len(parts) == 2 and (parts[1] == str(executable) or parts[1].startswith(str(executable) + ' ')):
            matches.append(int(parts[0]))
    return matches


def exercise(output):
    root = Path(__file__).resolve().parent.parent
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    # Keep the bundle under the ignored workspace for Launch Services discovery.
    with tempfile.TemporaryDirectory(prefix='bundle-', dir=output) as temporary:
        fixture = Path(temporary)
        bundle = fixture / 'GPUIO Signal Studio.app'
        executable = bundle / 'Contents/MacOS/gpuio-signal'
        executable.parent.mkdir(parents=True)
        binary = root / '_build/default/examples/signal_studio/main.exe'
        with (output / 'self-test.log').open('w') as self_log:
            subprocess.run([str(binary), '--self-test', '--document-path=' + str(fixture / 'snapshot.signal')],
                           stdout=self_log, stderr=subprocess.STDOUT, check=True, timeout=75)
        assert 'metadata, concurrent-edit, reset, invalid-load and busy checks passed' in (output / 'self-test.log').read_text()
        shutil.copy2(binary, executable)
        metadata = subprocess.check_output([str(binary), '--print-info-plist'], timeout=10)
        info = plistlib.loads(metadata)
        assert info['CFBundleIdentifier'] == 'com.gpuio.signal-studio'
        assert info['CFBundleURLTypes'][0]['CFBundleURLSchemes'] == ['gpuio-signal']
        (bundle / 'Contents/Info.plist').write_bytes(metadata)
        log = output / 'application.log'
        log.write_text('')
        proxy = None
        mac = None
        finder = None

        def wait(marker, count=1):
            end = time.monotonic() + 20
            while time.monotonic() < end:
                if log.read_text().count(marker) >= count:
                    return
                time.sleep(.05)
            raise RuntimeError(f'Missing {marker}: {log.read_text()}')

        def send(route):
            subprocess.run(['/usr/bin/open', '-g', '-a', str(bundle), '-u', 'gpuio-signal://' + route],
                           check=True, timeout=10)

        try:
            proxy = subprocess.Popen(['/usr/bin/open', '-W', '-n', '-g', '-a', str(bundle),
                                      '--stdout', str(log), '--stderr', str(log),
                                      '-u', 'gpuio-signal://sample/2', '--args', '--background', '--directory=' + str(fixture),
                                      '--open-uri=gpuio-signal://sample/4', '--open-uris', 'gpuio-signal://sample/1'])
            wait('link gpuio-signal://sample/2')
            recorded = log.read_text()
            assert recorded.index('SIGNAL_STUDIO: ready') < recorded.index('link gpuio-signal:')
            assert recorded.index('link gpuio-signal://sample/4') < recorded.index('link gpuio-signal://sample/1') < recorded.index('link gpuio-signal://sample/2')
            pids = pids_for(executable)
            assert len(pids) == 1, pids
            # Studio normally accepts a Popen whose PID is the app; here open -W
            # is the owned lifetime proxy and the actual bundle PID is separate.
            mac = Studio(proxy, log, pid=pids[0])
            mac.set(mac.app, 'AXFrontmost', mac.true)
            mac.resize(1160)
            mac.wait_text(TITLE, 'Sage · latency 528 ms · quality 57%')
            send('sample/3')
            wait('link gpuio-signal://sample/3')
            mac.wait_text(TITLE, 'Atlas · latency 769 ms · quality 89%')
            send('sample/999')
            wait('link rejected')
            mac.wait_text(TITLE, 'Atlas · latency 769 ms · quality 89%')
            mac.close(TITLE)
            end = time.monotonic() + 8
            while mac.has_window(TITLE) and time.monotonic() < end:
                time.sleep(.05)
            assert not mac.has_window(TITLE)
            subprocess.run(['/usr/bin/open', '-g', '-a', str(bundle)], check=True, timeout=10)
            wait('window opened', 2)
            mac.wait_text(TITLE, 'Atlas · latency 769 ms · quality 89%')
            mac.press(TITLE, 'Reset workspace')
            mac.wait_text(TITLE, 'Increment counter, current value 0')
            mac.press(TITLE, 'Reveal file')
            wait('Save a workspace before revealing it.')
            mac.press(TITLE, 'Increment counter, current value 0')
            mac.wait_text(TITLE, 'Increment counter, current value 1')
            mac.press(TITLE, 'Save workspace')
            field = mac.wait_find(TITLE, 'workspace.signal', 'AXTextField')
            mac.release(field)
            mac.press(TITLE, 'Save')
            wait('Workspace saved.')
            saved = fixture / 'workspace.signal'
            assert saved.is_file() and '(run 1)' in saved.read_text()
            mac.wait_text(TITLE, 'Saved workspace')
            mac.press(TITLE, 'Increment counter, current value 1')
            mac.wait_text(TITLE, 'Unsaved changes')
            mac.press(TITLE, 'Open workspace')
            wait('Save or reset changes before opening another workspace.')
            mac.press(TITLE, 'Reset workspace')
            mac.wait_text(TITLE, 'Increment counter, current value 0')
            mac.press(TITLE, 'Open workspace')
            node = mac.wait_find(TITLE, 'workspace.signal', search_files=True)
            try:
                mac.double_click(node)
            finally:
                mac.release(node)
            wait('Workspace loaded.')
            mac.wait_text(TITLE, 'Increment counter, current value 1')
            mac.wait_text(TITLE, 'Saved workspace')
            screenshot(mac, output / 'loaded.png', title=TITLE)
            mac.press(TITLE, 'Reveal file')
            wait('Reveal requested.')
            finder_pids = pids_for(Path('/System/Library/CoreServices/Finder.app/Contents/MacOS/Finder'))
            assert len(finder_pids) == 1, finder_pids
            finder = Mac(finder_pids[0])
            node = finder.wait_find(fixture.name, 'workspace', contains=True, search_files=True)
            finder.release(node)
            # Only the uniquely named disposable directory belongs to this test.
            finder.close(fixture.name)
            finder.release(finder.app)
            finder = None
            mac.set(mac.app, 'AXFrontmost', mac.true)
            mac.wait_text(TITLE, 'Increment counter, current value 1')
            assert pids_for(executable) == pids
            mac.press(TITLE, 'Quit Studio')
            assert proxy.wait(timeout=15) == 0
            assert not pids_for(executable)
            print('SIGNAL_DESKTOP_OK: packaged cold/warm/rejected links, same-process reopen, native save/open pickers, Eio persistence, dirty/open guard, actual Finder reveal, unsaved reveal fallback, restored model and shutdown', flush=True)
        except BaseException:
            if mac and mac.has_window(TITLE):
                mac.dump(TITLE)
                screenshot(mac, output / 'failure.png', title=TITLE)
            raise
        finally:
            if finder:
                try:
                    window = finder.window(fixture.name)
                    if window:
                        finder.release(window)
                        finder.close(fixture.name)
                finally:
                    finder.release(finder.app)
            if mac:
                mac.release(mac.app)
            for pid in pids_for(executable):
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            if proxy:
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
            registry = '/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister'
            subprocess.run([registry, '-u', str(bundle)], check=True, timeout=10)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('scratch/signal-desktop'))
    args = parser.parse_args()
    def timeout(_signal, _frame):
        raise TimeoutError('Signal Studio desktop walkthrough exceeded 150 seconds')
    signal.signal(signal.SIGALRM, timeout)
    signal.alarm(150)
    try:
        exercise(args.output)
    finally:
        signal.alarm(0)


if __name__ == '__main__':
    main()
