"""Disposable hosted-runner diagnostic, not a substitute for required CI."""
import os
import shutil
from pathlib import Path
import subprocess
from test_agent_chat import Mac
import test_notifications_macos as test

assert os.environ.get("GITHUB_ACTIONS") == "true", "Hosted disposable runner only"


def dump():
    names = ["NotificationCenter", "UserNotificationCenter", "CoreServicesUIAgent",
             "System Settings", "gpuio-notification", "usernoted"]
    for name in names:
        result = subprocess.run(["pgrep", "-u", str(os.getuid()), "-x", name],
                                capture_output=True, text=True, timeout=5)
        print("DIAGNOSTIC_PROCESS", name, result.returncode, result.stdout.strip(), flush=True)
        for pid in result.stdout.split():
            mac = Mac(int(pid))
            budget = [500]
            def visit(node, depth):
                if depth > 16 or budget[0] <= 0:
                    return
                budget[0] -= 1
                values, children = mac.node_values(node)
                try:
                    print("AX_DIAGNOSTIC", name, depth, repr(values), flush=True)
                    for child in children:
                        visit(child, depth + 1)
                finally:
                    for child in children:
                        mac.release(child)
            try:
                visit(mac.app, 0)
            finally:
                mac.release(mac.app)


original_wait = test.Notifications.wait

def diagnosed_wait(self, title):
    try:
        return original_wait(self, title)
    except Exception:
        dump()
        raise

test.Notifications.wait = diagnosed_wait
binary = Path(".cache/previous/notification-main.exe").resolve()
shutil.copy2(Path(".cache/previous/notification-os/GPUIO Notification Lab.app/Contents/MacOS/gpuio-notification"), binary)
binary.chmod(0o755)
try:
    test.exercise(binary,
                  Path(".cache/ci/notification-probe").resolve())
finally:
    print("DIAGNOSTIC_FINISHED", flush=True)
