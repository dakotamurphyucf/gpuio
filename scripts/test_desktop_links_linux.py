#!/usr/bin/env python3
"""Exercise generated desktop metadata and real cold/warm Linux URL dispatch.

Re-executes under its own dbus-run-session; requires an X11/Wayland display. Uses temporary
XDG data/config directories; never changes the user's default scheme handlers.
"""
import argparse
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time


def owned_pids(executable):
    output = subprocess.check_output(["ps", "-eo", "pid=,args="], text=True)
    result = []
    for line in output.splitlines():
        fields = line.strip().split(maxsplit=1)
        if len(fields) == 2 and (fields[1] == str(executable) or fields[1].startswith(str(executable) + " ")):
            result.append(int(fields[0]))
    return result


def exercise(binary, artifact):
    if not os.environ.get("DBUS_SESSION_BUS_ADDRESS"):
        raise RuntimeError("Run inside dbus-run-session")
    with tempfile.TemporaryDirectory(prefix="gpuio-desktop-") as directory:
        root = Path(directory)
        # GLib must unescape both Desktop Entry values and quoted Exec arguments;
        # these bytes must designate one literal executable, without a shell.
        executable = root / 'App $`"\\% runtime'
        shutil.copy2(binary, executable)
        applications = root / "data/applications"
        applications.mkdir(parents=True)
        (root / "config").mkdir()
        env = {**os.environ, "XDG_DATA_HOME": str(root / "data"),
               "XDG_CONFIG_HOME": str(root / "config"), "XDG_CACHE_HOME": str(root / "cache")}
        entry = applications / "com.gpuio.desktop-lab.desktop"
        entry.write_bytes(subprocess.check_output([str(binary), "--print-desktop-entry", str(executable), "--self-test"], env=env, timeout=10))
        subprocess.run(["desktop-file-validate", str(entry)], env=env, check=True, timeout=10)
        subprocess.run(["update-desktop-database", str(applications)], env=env, check=True, timeout=10)
        subprocess.run(["xdg-mime", "default", entry.name, "x-scheme-handler/gpuio-desktop-lab"], env=env, check=True, timeout=10)
        default = subprocess.check_output(["xdg-mime", "query", "default", "x-scheme-handler/gpuio-desktop-lab"], env=env, text=True, timeout=10).strip()
        assert default == entry.name, default
        log = root / "application.log"
        log.touch()
        def content():
            return log.read_text(errors="replace")
        def wait_for(marker):
            end = time.monotonic() + 12
            while time.monotonic() < end:
                if marker in content():
                    return
                time.sleep(.05)
            raise AssertionError(f"Missing {marker!r}\n{content()}")
        def send(suffix):
            with log.open("ab") as output:
                subprocess.run(["gio", "open", "gpuio-desktop-lab://" + suffix], env=env,
                               stdout=output, stderr=output, check=True, timeout=10)
        try:
            send("document/cold")
            wait_for("DESKTOP_LAB: link gpuio-desktop-lab://document/cold")
            initial = content()
            assert initial.index("DESKTOP_LAB: waiting") < initial.index("DESKTOP_LAB: ready") < initial.index("DESKTOP_LAB: link ")
            primary = owned_pids(executable)
            assert len(primary) == 1, primary
            send("document/warm")
            wait_for("DESKTOP_LAB: link gpuio-desktop-lab://document/warm")
            wait_for("DESKTOP_LAB: forwarded")
            send("user@document/invalid")
            wait_for("DESKTOP_LAB: rejected gpuio-desktop-lab://user@document/invalid Invalid_authority")
            send("close/window")
            wait_for("DESKTOP_LAB: window-close-requested")
            # Launcher with no URI requests the existing app's reopen policy.
            subprocess.run([str(executable)], env=env, check=True, timeout=10,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            wait_for("DESKTOP_LAB: reopened")
            wait_for("DESKTOP_LAB: window-opened 2")
            send("document/reopened")
            wait_for("DESKTOP_LAB: link gpuio-desktop-lab://document/reopened")
            assert content().count("DESKTOP_LAB: exclusive-receiver") == 1
            assert content().count("DESKTOP_LAB: link gpuio-desktop-lab://document/warm") == 1
            assert owned_pids(executable) == primary
            send("quit/application")
            wait_for("DESKTOP_LAB: link gpuio-desktop-lab://quit/application")
            end = time.monotonic() + 5
            while owned_pids(executable) and time.monotonic() < end:
                time.sleep(.05)
            assert not owned_pids(executable), "application did not exit"
            print("GPUIO_DESKTOP_LINUX_OK: generated metadata, literal Exec path, cold/warm default URL dispatch, rejection, single instance, empty reopen and shutdown")
        finally:
            artifact.parent.mkdir(parents=True, exist_ok=True)
            artifact.write_text(content())
            for pid in owned_pids(executable):
                try:
                    os.kill(pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
            end = time.monotonic() + 3
            while owned_pids(executable) and time.monotonic() < end:
                time.sleep(.05)
            for pid in owned_pids(executable):
                try:
                    os.kill(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("_build/default/examples/desktop/main.exe"))
    parser.add_argument("--log", type=Path, default=Path("scratch/desktop-links-linux.log"))
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("Requires a Linux graphical session")
    if os.environ.get("GPUIO_DESKTOP_BUS_CHILD") != "1":
        env = {**os.environ, "GPUIO_DESKTOP_BUS_CHILD": "1"}
        os.execvpe("dbus-run-session", ["dbus-run-session", "--", sys.executable,
                    str(Path(__file__).resolve()), *sys.argv[1:]], env)
    exercise(args.binary.resolve(strict=True), args.log.resolve())
