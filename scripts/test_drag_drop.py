#!/usr/bin/env python3
"""Post one real AppKit drag to the spawned Bonsai example and check its callbacks.

Requires macOS accessibility permission. AX actions target only the child created
here. System-wide AX hit-testing checks the owner before posting mouse events.
The default mode tests internal text. --desktop tests an OS file offer to a second
child window. Build the corresponding example and native_drag_drop first.
"""
import argparse
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver", required=True, type=Path)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--desktop", action="store_true")
    modes.add_argument("--reenter", action="store_true")
    modes.add_argument("--cancel", action="store_true")
    modes.add_argument("--remove-source", action="store_true")
    modes.add_argument("--close-source", action="store_true")
    modes.add_argument("--shutdown", action="store_true")
    modes.add_argument("--close-internal", action="store_true")
    modes.add_argument("--shutdown-internal", action="store_true")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this test covers the AppKit backend")
    repo = Path(__file__).resolve().parent.parent
    scenario = next((name for name in ("desktop", "reenter", "cancel", "remove_source", "close_source", "shutdown", "close_internal", "shutdown_internal")
                     if getattr(args, name)), "public")
    desktop = scenario != "public"
    name = "drag_drop_desktop" if desktop else "drag_drop"
    example = repo / f"_build/default/examples/{name}/main.exe"
    driver = args.driver.resolve()
    if not example.is_file() or not driver.is_file():
        parser.error("build the example and driver first")
    with tempfile.TemporaryFile(mode="w+t") as log, tempfile.TemporaryDirectory(prefix="gpuio-drag-") as scratch:
        fixture = Path(scratch).resolve() / "drag fixture.txt"
        fixture.write_text("GPUIO file drag fixture\n")
        release_status = Path(scratch).resolve() / "release-status"
        release_status.write_text("held")
        child_args = [str(fixture), str(release_status)] if desktop else ["--gesture-self-test"]
        if scenario not in ("public", "desktop"):
            child_args.append("--" + scenario.replace("_", "-"))
        driver_mode = "--drive-" + scenario.replace("_", "-")
        child = subprocess.Popen([str(example), *child_args], cwd=repo,
                                 stdout=log, stderr=subprocess.STDOUT)
        try:
            subprocess.run([str(driver), driver_mode, str(child.pid)], cwd=repo,
                           check=True, timeout=20)
            release_status.write_text("released")
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
        if fixture.read_text() != "GPUIO file drag fixture\n":
            raise RuntimeError("file dragging unexpectedly changed the source file")
        markers = {"public": "GESTURE", "desktop": "DESKTOP", "reenter": "REENTRY",
                   "cancel": "CANCEL", "remove_source": "REMOVAL", "close_source": "CLOSE",
                   "shutdown": "SHUTDOWN", "close_internal": "CLOSE_INTERNAL",
                   "shutdown_internal": "SHUTDOWN_INTERNAL"}
        marker = f"GPUIO_DRAG_DROP_{markers[scenario]}_OK:"
        if marker not in output:
            raise RuntimeError("missing public gesture acceptance marker")


if __name__ == "__main__":
    main()
