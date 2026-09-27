#!/usr/bin/env python3
"""Exercise real Launch Services URL delivery to the public desktop example.

Build examples/desktop/main.exe first. Creates a disposable application bundle;
does not request default-handler reassignment. Every launch uses an explicit app.
"""

import argparse
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import sys
import tempfile
import time

from test_agent_chat import Mac


def package(binary: Path, root: Path) -> Path:
    bundle = root / "GPUIO Desktop Lab.app"
    contents = bundle / "Contents"
    executable = contents / "MacOS" / "gpuio-desktop"
    executable.parent.mkdir(parents=True)
    shutil.copy2(binary, executable)
    with (contents / "Info.plist").open("wb") as output:
        plistlib.dump({
            "CFBundleExecutable": executable.name,
            "CFBundleIdentifier": "com.gpuio.desktop-lab",
            "CFBundleName": "GPUIO Desktop Lab",
            "CFBundlePackageType": "APPL",
            "CFBundleVersion": "1",
            "CFBundleShortVersionString": "0.1.0",
            "LSMinimumSystemVersion": "14.4",
            "NSHighResolutionCapable": True,
            "CFBundleURLTypes": [{
                "CFBundleURLName": "GPUIO Desktop Lab links",
                "CFBundleURLSchemes": ["gpuio-desktop-lab"],
                "CFBundleTypeRole": "Viewer",
            }],
        }, output)
    return bundle


def owned_pids(bundle: Path) -> list[int]:
    executable = str((bundle / "Contents/MacOS/gpuio-desktop").resolve())
    output = subprocess.check_output(["ps", "-axo", "pid=,command="], text=True)
    found = []
    for line in output.splitlines():
        fields = line.strip().split(maxsplit=1)
        if len(fields) == 2 and (fields[1] == executable or fields[1].startswith(executable + " ")):
            found.append(int(fields[0]))
    return found


def exercise(binary: Path, artifact: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="gpuio-desktop-") as directory:
        root = Path(directory)
        bundle = package(binary, root)
        log = root / "application.log"
        log.touch()
        proxy = None
        mac = None

        def content():
            return log.read_text(errors="replace")

        def wait_for(marker):
            deadline = time.monotonic() + 12
            while time.monotonic() < deadline:
                if marker in content():
                    return
                time.sleep(0.05)
            raise AssertionError(f"Missing {marker!r}\n{content()}")

        def send(suffix):
            subprocess.run(["/usr/bin/open", "-g", "-a", str(bundle), "-u",
                            "gpuio-desktop-lab://" + suffix], check=True, timeout=5)

        def wait_windows(expected):
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                windows = mac.children(mac.app, "AXWindows")
                try:
                    if len(windows) == expected:
                        return
                finally:
                    for window in windows:
                        mac.release(window)
                time.sleep(0.05)
            raise AssertionError(f"Native window count did not become {expected}")

        try:
            proxy = subprocess.Popen([
                "/usr/bin/open", "-W", "-n", "-g", "-a", str(bundle),
                "--stdout", str(log), "--stderr", str(log),
                "-u", "gpuio-desktop-lab://document/cold", "--args", "--self-test",
            ])
            wait_for("DESKTOP_LAB: link gpuio-desktop-lab://document/cold")
            initial = content()
            assert initial.index("DESKTOP_LAB: waiting") < initial.index("DESKTOP_LAB: ready")
            assert initial.index("DESKTOP_LAB: ready") < initial.index("DESKTOP_LAB: link ")
            assert "DESKTOP_LAB: exclusive-receiver" in initial
            pids = owned_pids(bundle)
            assert len(pids) == 1, pids
            mac = Mac(pids[0], proxy)
            # AppKit can defer the AX window tree for a never-activated bundle.
            # Activate only this owned test app before physical window checks.
            mac.set(mac.app, "AXFrontmost", mac.true)
            wait_windows(1)
            send("document/warm")
            wait_for("DESKTOP_LAB: link gpuio-desktop-lab://document/warm")
            mac.wait_text("GPUIO · Desktop Lab", "Document /warm")
            send("metadata/edited")
            wait_for("DESKTOP_LAB: metadata-edited")
            send("metadata/clear")
            wait_for("DESKTOP_LAB: metadata-cleared")
            send("user@document/invalid")
            wait_for("DESKTOP_LAB: rejected gpuio-desktop-lab://user@document/invalid Invalid_authority")
            send("close/window")
            wait_for("DESKTOP_LAB: window-close-requested")
            wait_windows(0)
            send("document/reopened")
            wait_for("DESKTOP_LAB: window-opened 2")
            wait_windows(1)
            mac.wait_text("GPUIO · Desktop Lab", "Document /reopened")
            send("metadata/stale")
            wait_for("DESKTOP_LAB: metadata-stale-closed")
            send("replace/receiver")
            wait_for("DESKTOP_LAB: receiver-replaced")
            send("document/replacement")
            wait_for("DESKTOP_LAB: link gpuio-desktop-lab://document/replacement")
            mac.wait_text("GPUIO · Desktop Lab", "Document /replacement")
            assert owned_pids(bundle) == pids, "Routing started another process"
            send("quit/application")
            assert proxy.wait(timeout=10) == 0
            assert not owned_pids(bundle), "Owned application still running"
            result = content()
            assert "DESKTOP_LAB: deadline" not in result
            for route in ["cold", "warm", "reopened", "replacement"]:
                assert result.count("DESKTOP_LAB: link gpuio-desktop-lab://document/" + route) == 1
            print("GPUIO_DESKTOP_LINKS_OK: packaged cold/warm OS links, readiness, rejection, document metadata/clear/stale-window, window reopen, receiver replacement and shutdown")
        finally:
            if mac is not None:
                mac.release(mac.app)
            artifact.parent.mkdir(parents=True, exist_ok=True)
            artifact.write_text(content())
            for pid in owned_pids(bundle):
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            if proxy is not None:
                try:
                    proxy.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    for pid in owned_pids(bundle):
                        try:
                            os.kill(pid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                    proxy.kill()
                    proxy.wait()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("_build/default/examples/desktop/main.exe"))
    parser.add_argument("--log", type=Path, default=Path("scratch/desktop-links-macos.log"))
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("Requires macOS Launch Services")
    exercise(args.binary.resolve(strict=True), args.log.resolve())
