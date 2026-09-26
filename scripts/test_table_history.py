#!/usr/bin/env python3
"""Full native table traversal with live progress and bounded process lifetime."""
import os
import signal
import subprocess
import threading
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
COMMAND = [
    "./scripts/gpuio", "exec", "cargo", "test", "--locked", "-j2",
    "-p", "gpuio-native", "--features", "native-tests", "--test", "native_table_history",
]


def run(arguments, timeout):
    process = subprocess.Popen(
        COMMAND + arguments, cwd=REPO, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, start_new_session=True,
    )
    lines = []

    def consume():
        for line in process.stdout:
            lines.append(line)
            print(line, end="", flush=True)

    reader = threading.Thread(target=consume)
    reader.start()
    try:
        process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        raise SystemExit("native table history timed out; process group terminated and reaped")
    except BaseException:
        # Cancellation must not leave Cargo or its native window behind.
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
        raise
    finally:
        reader.join()
        process.stdout.close()
    return process.returncode, "".join(lines)


def main():
    status, output = run([], 900)
    if status:
        raise SystemExit(status)
    for marker in ("GPUIO_TABLE_HISTORY_OK", "GPUIO_TABLE_HISTORY_WINDOW_RELEASE_OK"):
        if marker not in output:
            raise SystemExit(f"native table history exited without completing {marker}")
    status, output = run(["--", "--verify-failure-exit"], 90)
    if (status == 0 or "TABLE_HISTORY_EXPECTED_FAILURE" not in output
            or "TABLE_HISTORY_FAILURE_CLEANUP_OK" not in output):
        raise SystemExit("native table assertion did not produce verified failure cleanup")
    print("GPUIO_TABLE_HISTORY_FAILURE_EXIT_OK", flush=True)


if __name__ == "__main__":
    main()
