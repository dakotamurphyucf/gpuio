#!/usr/bin/env python3
"""Real inherited-output tests: no AppKit windows, switches or descriptor-global changes."""
import argparse
import errno
import os
from pathlib import Path
import pty
import select
import subprocess
import tempfile
import time
import tty

EXPECTED = bytes(65 + index % 26 for index in range(2 * 1024 * 1024))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, default=Path('_build/default/test/output/probe.exe'))
    args = parser.parse_args()
    binary = str(args.executable.resolve())
    result = subprocess.run([binary], capture_output=True, timeout=20, check=True)
    assert result.stdout == EXPECTED
    with tempfile.TemporaryFile() as file:
        blocking = os.get_blocking(file.fileno())
        subprocess.run([binary], stdout=file, stderr=subprocess.PIPE, timeout=20, check=True)
        assert os.get_blocking(file.fileno()) == blocking
        file.seek(0)
        assert file.read() == EXPECTED
    with open(os.devnull, 'wb') as sink:
        blocking = os.get_blocking(sink.fileno())
        subprocess.run([binary], stdout=sink, stderr=subprocess.PIPE, timeout=20, check=True)
        assert os.get_blocking(sink.fileno()) == blocking
    with open(os.devnull, 'rb') as sink:
        subprocess.run([binary, '--read-only'], stdout=sink, stderr=subprocess.PIPE,
                       timeout=20, check=True)
    subprocess.run([binary, '--cancel'], stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5, check=True)

    # A private pseudo-terminal exercises a real blocking character descriptor;
    # it is not a foreground window, keyboard/IME or physical-terminal claim.
    master, slave = pty.openpty()
    child = None
    try:
        tty.setraw(slave)
        child = subprocess.Popen([binary, '--character-responsive'], stdout=slave, stderr=subprocess.PIPE)
        # Leave the PTY undrained until an independent Eio timer runs. A blocking
        # write on the scheduler domain would prevent this marker entirely.
        assert select.select([child.stderr], [], [], 5)[0], 'Character output blocked the Eio scheduler'
        assert b'OUTPUT_TIMER_FIRED' in child.stderr.readline()
        os.close(slave)
        slave = None
        output = bytearray()
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if not select.select([master], [], [], .1)[0]:
                continue
            try:
                chunk = os.read(master, 65536)
            except OSError as error:
                if error.errno == errno.EIO:
                    break  # Linux PTY EOF; macOS returns an empty read.
                raise
            if not chunk:
                break
            output.extend(chunk)
            if len(output) > len(EXPECTED):
                raise AssertionError('Duplicated output')
        assert output == EXPECTED, len(output)
        _, stderr = child.communicate(timeout=5)
        assert child.returncode == 0, stderr
    finally:
        if child is not None:
            if child.poll() is None:
                child.kill()
            child.wait()
            if child.stderr:
                child.stderr.close()
        os.close(master)
        if slave is not None:
            os.close(slave)
    print('OUTPUT_SINK_OK: exact repeated 2MiB pipe/file/PTY writes, devnull, EBADF, nonblocking-pipe cancellation, borrowed-fd mode/lifetime')


if __name__ == '__main__':
    main()
