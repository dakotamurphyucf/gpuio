#!/usr/bin/env python3
"""Native table acceptance markers and a real nonzero failure-exit check."""
import os
import signal
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
COMMAND = [
    "./scripts/gpuio", "exec", "cargo", "test", "--locked", "-j2",
    "-p", "gpuio-table-adapter", "--features", "native-tests", "--test", "native_table",
]
MARKERS = (
    "TABLE_CANDIDATE_OK", "TABLE_KEYED_SELECTION_OK", "TABLE_EVENT_IDENTITY_OK",
    "TABLE_ANCHORS_OK", "TABLE_ENTITY_RELEASE_OK",
)


def run(arguments):
    process = subprocess.Popen(
        COMMAND + arguments, cwd=REPO, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, start_new_session=True,
    )
    try:
        output, _ = process.communicate(timeout=90)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        output, _ = process.communicate()
        print(output, end="", flush=True)
        raise SystemExit("native table timed out; terminated its process group")
    return subprocess.CompletedProcess(process.args, process.returncode, output)


def main():
    normal = run([])
    print(normal.stdout, end="", flush=True)
    if normal.returncode != 0:
        raise SystemExit(normal.returncode)
    for marker in MARKERS:
        if marker not in normal.stdout:
            raise SystemExit(f"native table exited without completing {marker}")
    failure = run(["--", "--verify-failure-exit"])
    if (failure.returncode == 0 or "TABLE_NATIVE_FAILED" not in failure.stdout
            or "TABLE_FAILURE_CLEANUP_OK" not in failure.stdout):
        print(failure.stdout, end="", flush=True)
        raise SystemExit("native assertion did not produce a verified failing process")
    print("TABLE_FAILURE_EXIT_OK: intentional assertion closed its window and failed the process")


if __name__ == "__main__":
    main()
