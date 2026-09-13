#!/usr/bin/env python3
"""Post one real AppKit drag to the spawned Bonsai example and check its callbacks.

Requires macOS accessibility permission. AX actions target only the child created
here. System-wide AX hit-testing checks the owner before posting mouse events.
This is an internal application drag, not OS file export. Build examples/drag_drop/main.exe and native_drag_drop first.
"""
import argparse
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver", required=True, type=Path)
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this test covers the AppKit backend")
    repo = Path(__file__).resolve().parent.parent
    example = repo / "_build/default/examples/drag_drop/main.exe"
    driver = args.driver.resolve()
    if not example.is_file() or not driver.is_file():
        parser.error("build the example and driver first")
    with tempfile.TemporaryFile(mode="w+t") as log:
        child = subprocess.Popen([str(example), "--gesture-self-test"], cwd=repo,
                                 stdout=log, stderr=subprocess.STDOUT)
        try:
            subprocess.run([str(driver), "--drive-public", str(child.pid)], cwd=repo,
                           check=True, timeout=20)
            if child.wait(timeout=35) != 0:
                raise RuntimeError("public drag/drop example failed")
        finally:
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            log.seek(0)
            output = log.read()
            print(output, end="")
        if "GPUIO_DRAG_DROP_GESTURE_OK:" not in output:
            raise RuntimeError("missing public gesture acceptance marker")


if __name__ == "__main__":
    main()
