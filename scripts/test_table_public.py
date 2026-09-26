#!/usr/bin/env python3
"""Run the built public Bonsai/Eio table example with bounded process lifetime."""
import os
import signal
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]


def main():
    command = ["./scripts/gpuio", "exec", "_build/default/examples/table/main.exe", "--self-test"]
    if "--background" in sys.argv[1:]:
        command.append("--background")
    process = subprocess.Popen(command, cwd=REPO, text=True, stdout=subprocess.PIPE,
                               stderr=subprocess.STDOUT, start_new_session=True)
    try:
        output, _ = process.communicate(timeout=120)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        output, _ = process.communicate()
        print(output, end="", flush=True)
        raise SystemExit("public table timed out; process group terminated and reaped")
    print(output, end="", flush=True)
    if process.returncode:
        raise SystemExit(process.returncode)
    if "GPUIO_TABLE_PUBLIC_OK" not in output:
        raise SystemExit("public table exited without completion marker")


if __name__ == "__main__":
    main()
