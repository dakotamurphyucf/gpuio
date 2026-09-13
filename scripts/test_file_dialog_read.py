#!/usr/bin/env python3
"""Drive the public macOS picker example through its real accessibility controls.

Build the example and native_file_dialog harness first. Pass the latter's exact
Cargo executable path as --driver. Requires accessibility access, like the native
file-panel suite. Only the child PID created here is targeted.
"""
import argparse
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this test exercises the AppKit backend on macOS")
    repo = Path(__file__).resolve().parent.parent
    example = repo / "_build/default/examples/file_dialogs/main.exe"
    driver = args.driver.resolve()
    if not example.is_file() or not driver.is_file():
        parser.error("build the public example and native_file_dialog harness first")
    with tempfile.TemporaryFile(mode="w+t") as log:
        child = subprocess.Popen(
            [str(example), "--read-self-test", str(repo)],
            cwd=repo, stdout=log, stderr=subprocess.STDOUT,
        )
        try:
            subprocess.run(
                [str(driver), "--drive-picker", str(child.pid), "LICENSE", "Read test file"],
                cwd=repo, check=True, timeout=40,
            )
            if child.wait(timeout=35) != 0:
                raise RuntimeError("public file-dialog example failed")
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
        if "GPUIO_FILE_DIALOG_READ_OK:" not in output:
            raise RuntimeError("public example did not report selection/read acceptance")


if __name__ == "__main__":
    main()
