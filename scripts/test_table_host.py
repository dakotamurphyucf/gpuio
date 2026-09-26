#!/usr/bin/env python3
"""Run the real retained table host with bounded lifetime and completion evidence."""
import os
import signal
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]


def main():
    process = subprocess.Popen(
        ["./scripts/gpuio", "exec", "cargo", "test", "--locked", "-j2",
         "-p", "gpuio-native", "--features", "native-tests", "--test", "native_table_host"],
        cwd=REPO, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    try:
        output, _ = process.communicate(timeout=120)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        output, _ = process.communicate()
        print(output, end="", flush=True)
        raise SystemExit("native table host timed out; process group terminated and reaped")
    print(output, end="", flush=True)
    if process.returncode:
        raise SystemExit(process.returncode)
    if "GPUIO_NATIVE_TABLE_HOST_OK" not in output:
        raise SystemExit("native table host exited without its completion marker")


if __name__ == "__main__":
    main()
