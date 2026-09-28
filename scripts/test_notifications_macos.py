#!/usr/bin/env python3
"""Packaged public notification API with real macOS Notification Center actions.

This test explicitly requests permission for com.gpuio.notification-lab. It only
acts on that app's notification rows; other notifications are neither logged nor
activated. Existing denial requires enabling this test app in System Settings.
"""
import argparse
import ctypes as C
import os
from pathlib import Path
import plistlib
import shutil
import signal
import subprocess
import time
from test_agent_chat import Mac

APP_NAME = "GPUIO Notification Lab"
TITLE = "GPUIO · Notification Lab"
def initial(index): return f"GPUIO · Build complete #{index}"
UPDATED = "GPUIO · Build updated #4"


def pids(executable):
    output = subprocess.check_output(["ps", "-axo", "pid=,command="], text=True)
    return [int(fields[0]) for line in output.splitlines()
            if len(fields := line.strip().split(maxsplit=1)) == 2
            and (fields[1] == str(executable) or fields[1].startswith(str(executable) + " "))]


class Notifications:
    def __init__(self, app_name=APP_NAME):
        self.app_name = app_name
        self.mac = Mac(int(subprocess.check_output(["pgrep", "-x", "NotificationCenter"], text=True).strip()))
        self.copy_actions = self.mac.ax.AXUIElementCopyActionNames
        self.copy_actions.restype = C.c_int
        self.copy_actions.argtypes = [C.c_void_p, C.POINTER(C.c_void_p)]

    def actions(self, node):
        value = C.c_void_p()
        if self.copy_actions(node, C.byref(value)) or not value.value:
            return []
        try:
            names = []
            for i in range(self.mac.count(value)):
                buffer = C.create_string_buffer(8192)
                assert self.mac.get_string(self.mac.item(value, i), buffer, len(buffer), 0x08000100)
                names.append(buffer.value.decode())
            return names
        finally:
            self.mac.release(value)

    def find(self, title):
        m = self.mac
        def contains(node, text, depth=0):
            values, children = m.node_values(node)
            try:
                return text in values[1:] or (depth < 8 and any(contains(c, text, depth + 1) for c in children))
            finally:
                for child in children: m.release(child)
        def visit(node, depth=0):
            values, children = m.node_values(node)
            try:
                if values[0] == "AXGroup" and any(self.app_name in v for v in values[1:]) and contains(node, title):
                    return m.retain(node)
                if depth < 18:
                    for child in children:
                        found = visit(child, depth + 1)
                        if found: return found
            finally:
                for child in children: m.release(child)
            return None
        windows = m.children(m.app, "AXWindows")
        try:
            for window in windows:
                found = visit(window)
                if found: return found
        finally:
            for window in windows: m.release(window)
        return None

    def wait(self, title):
        end = time.monotonic() + 10
        while time.monotonic() < end:
            node = self.find(title)
            if node: return node
            time.sleep(.05)
        raise AssertionError(f"Test notification not present: {title}")

    def act(self, title, name):
        node = self.wait(title)
        try:
            actions = self.actions(node)
            action = next((a for a in actions if a == name or a.startswith(f"Name:{name}\n")), None)
            assert action, (name, actions)
            self.mac.perform(node, action)
        finally:
            self.mac.release(node)

    def open_center(self):
        # Escape dismisses any existing presentation before the clock toggle,
        # making this an open operation rather than an accidental close.
        self.mac.key(53)
        time.sleep(.35)
        m = Mac(int(subprocess.check_output(["pgrep", "-x", "ControlCenter"], text=True).strip()))
        def visit(node, depth=0):
            values, children = m.node_values(node)
            try:
                if values[0] == "AXMenuBarItem" and "Clock" in values[1:]:
                    m.perform(node, "AXPress")
                    return True
                return depth < 5 and any(visit(c, depth + 1) for c in children)
            finally:
                for child in children: m.release(child)
        try: assert visit(m.app), "Clock menu item not available"
        finally: m.release(m.app)

    def close(self):
        self.mac.key(53)
        self.mac.release(self.mac.app)


def exercise(binary, artifact):
    artifact.mkdir(parents=True, exist_ok=True)
    unbundled = subprocess.run([str(binary), "--unbundled-check"], capture_output=True, text=True, timeout=20, check=True)
    (artifact / "unbundled.log").write_text(unbundled.stdout + unbundled.stderr)
    assert "unbundled-check-passed" in unbundled.stdout + unbundled.stderr
    print("PASS unbundled public API reports unavailable without aborting", flush=True)
    bundle = artifact / f"{APP_NAME}.app"
    executable = bundle / "Contents/MacOS/gpuio-notification"
    executable.parent.mkdir(parents=True, exist_ok=True)
    assert not pids(executable), "Previous test process still owns this bundle"
    executable.unlink(missing_ok=True)
    shutil.copy2(binary, executable)
    metadata = subprocess.check_output([str(executable), "--print-info-plist"], timeout=10)
    assert plistlib.loads(metadata)["CFBundleIdentifier"] == "com.gpuio.notification-lab"
    (bundle / "Contents/Info.plist").write_bytes(metadata)
    executable.chmod(0o755)
    subprocess.run(["/usr/bin/codesign", "--force", "--sign", "-", "--identifier", "com.gpuio.notification-lab", "--timestamp=none", str(bundle)], check=True, timeout=15)
    subprocess.run(["/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister", "-f", str(bundle)], check=True, timeout=15)
    log = artifact / "application.log"
    log.write_text("")
    proxy = subprocess.Popen(["/usr/bin/open", "-W", "-n", "-g", "-a", str(bundle), "--stdout", str(log), "--stderr", str(log), "--args", "--self-test"])
    app = None
    notifications = None
    def content(): return log.read_text(errors="replace")
    def wait(marker, count=1):
        end = time.monotonic() + 15
        while time.monotonic() < end:
            if content().count(marker) >= count: return
            if proxy.poll() is not None: break
            time.sleep(.05)
        raise AssertionError(f"Missing {marker!r} count={count}\n{content()}")
    try:
        wait("authorization (Ok")
        owned = pids(executable)
        assert len(owned) == 1
        app = Mac(owned[0], proxy)
        app.set(app.app, "AXFrontmost", app.true)
        notifications = Notifications()
        if "authorization (Ok Not_determined)" in content():
            app.press(TITLE, "Post notification")
            wait("posted (Error Not_ready)")
            app.press(TITLE, "Allow notifications")
            notifications.act(f"“{APP_NAME}” Notifications", "Allow")
            wait("permission (Ok Authorized)")
        elif "authorization (Ok Denied)" in content():
            raise AssertionError("Enable notifications only for GPUIO Notification Lab in System Settings, then rerun; no global notification settings are changed by this test")
        else:
            assert "authorization (Ok Authorized)" in content(), content()
        app.press(TITLE, "Post notification")
        wait("posted (Ok", 1)
        node = notifications.wait(initial(1))
        notifications.mac.release(node)
        app.press(TITLE, "Post notification")
        wait("posted (Error Busy)")
        notifications.act(initial(1), "Open workspace")
        time.sleep(.3)
        assert "event " not in content(), "Action escaped readiness gate"
        app.press(TITLE, "Ready for actions")
        wait("event (Action")
        assert "open)" in content(), content()
        print("PASS native named action queued before readiness", flush=True)

        app.press(TITLE, "Post notification")
        wait("posted (Ok", 2)
        notifications.act(initial(2), "AXPress")
        wait("event (Activated")
        print("PASS native default activation", flush=True)

        app.press(TITLE, "Post notification")
        wait("posted (Ok", 3)
        notifications.act(initial(3), "Close")
        wait("event (Closed")
        assert "User)" in content(), content()
        print("PASS native user dismissal", flush=True)

        app.press(TITLE, "Post notification")
        wait("posted (Ok", 4)
        node = notifications.wait(initial(4))
        notifications.mac.release(node)
        app.press(TITLE, "Replace content")
        wait("replaced (Ok ())")
        notifications.open_center()
        node = notifications.wait(UPDATED)
        try:
            actions = notifications.actions(node)
            assert any(a.startswith("Name:Inspect build\n") for a in actions), actions
            assert not any(a.startswith("Name:Open workspace\n") for a in actions), actions
        finally: notifications.mac.release(node)
        notifications.act(UPDATED, "Inspect build")
        wait("event (Action", 2)
        assert "inspect)" in content(), content()
        print("PASS same-lifetime replacement and new native action", flush=True)

        app.press(TITLE, "Post notification")
        wait("posted (Ok", 5)
        node = notifications.wait(initial(5))
        notifications.mac.release(node)
        app.press(TITLE, "Dismiss notification")
        wait("dismissed (Ok ())")
        deadline = time.monotonic() + 10
        while True:
            node = notifications.find(initial(5))
            if node is None: break
            notifications.mac.release(node)
            assert time.monotonic() < deadline, "Explicitly dismissed notification remains visible"
            time.sleep(.05)
        print("PASS explicit OS dismissal", flush=True)
        app.press(TITLE, "Post notification")
        wait("posted (Ok", 6)
        node = notifications.wait(initial(6))
        notifications.mac.release(node)
        window = app.window(TITLE)
        assert window
        button = app.attr(window, "AXCloseButton")
        try:
            assert button
            app.perform(button, "AXPress")
        finally:
            if button: app.release(button)
            app.release(window)
        deadline = time.monotonic() + 5
        while True:
            window = app.window(TITLE)
            if not window: break
            app.release(window)
            assert time.monotonic() < deadline, "Old window did not close"
            time.sleep(.05)
        subprocess.run(["/usr/bin/open", "-a", str(bundle)], check=True, timeout=5)
        wait("window-opened", 2)
        node = app.wait_find(TITLE, "Post notification", "AXButton")
        app.release(node)
        notifications.open_center()
        notifications.act(initial(6), "Open workspace")
        wait("target Closed")
        window = app.window(TITLE)
        assert window, "Stale action affected replacement window"
        app.release(window)
        print("PASS closed-window receipt cannot activate a replacement window", flush=True)

        app.press(TITLE, "Post notification")
        wait("posted (Ok", 7)
        node = notifications.wait(initial(7))
        notifications.mac.release(node)
        app.press(TITLE, "Close notification service")
        wait("service-closed")
        app.press(TITLE, "Post notification")
        wait("posted (Error Closed)")
        deadline = time.monotonic() + 10
        while True:
            node = notifications.find(initial(7))
            if node is None: break
            notifications.mac.release(node)
            assert time.monotonic() < deadline, "Closed service retained its notification"
            time.sleep(.05)
        print("PASS service cleanup and closed admission", flush=True)
        app.press(TITLE, "Quit lab")
        proxy.wait(timeout=10)
        assert not pids(executable), "Test app did not exit"
        print("PASS packaged macOS notification API and real OS actions", flush=True)
    finally:
        if notifications: notifications.close()
        if app: app.release(app.app)
        for pid in pids(executable):
            try: os.kill(pid, signal.SIGTERM)
            except ProcessLookupError: pass
        try: proxy.wait(timeout=5)
        except subprocess.TimeoutExpired:
            for pid in pids(executable):
                try: os.kill(pid, signal.SIGKILL)
                except ProcessLookupError: pass
            proxy.terminate()
            proxy.wait(timeout=5)
        subprocess.run([
            "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
            "-u", str(bundle),
        ], check=False, timeout=10)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, default=Path("_build/default/examples/notification/main.exe"))
    parser.add_argument("--artifact", type=Path, default=Path("scratch/notification-os/acceptance"))
    args = parser.parse_args()
    exercise(args.binary.resolve(), args.artifact.resolve())
