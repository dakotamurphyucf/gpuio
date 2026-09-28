#!/usr/bin/env python3
"""Enable the actual Notification Center agent on disposable hosted macOS CI.

The runner image disables this launch agent. Merely opening its application
does not register the Mach service used for permission prompts and notifications.
This restores the OS service only; tests still request and exercise permission.
"""
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import time


def main():
    if (sys.platform != 'darwin' or os.environ.get('GITHUB_ACTIONS') != 'true'
            or os.environ.get('RUNNER_ENVIRONMENT') != 'github-hosted'):
        raise SystemExit('Refusing to change notification services outside hosted macOS CI')
    uid = os.getuid()
    if Path('/dev/console').stat().st_uid != uid:
        raise SystemExit('The CI user must own the active desktop session')
    path = Path('/System/Library/LaunchAgents/com.apple.notificationcenterui.plist')
    agent = plistlib.loads(path.read_bytes())
    label = agent.get('Label')
    if label != 'com.apple.notificationcenterui.agent':
        raise SystemExit(f'Unexpected Notification Center launch agent label: {label!r}')
    if 'com.apple.notificationcenterui.main' not in agent.get('MachServices', {}):
        raise SystemExit('Notification Center agent has no expected Mach service')
    domain = f'gui/{uid}'
    service = f'{domain}/{label}'
    subprocess.run(['launchctl', 'enable', service], check=True, timeout=10)
    registered = subprocess.run(['launchctl', 'print', service], capture_output=True,
                                text=True, timeout=10)
    if registered.returncode:
        subprocess.run(['launchctl', 'bootstrap', domain, str(path)], check=True, timeout=10)
    # No -k: never terminate a running service to perform this setup.
    subprocess.run(['launchctl', 'kickstart', service], check=True, timeout=10)
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        state = subprocess.run(['launchctl', 'print', service], capture_output=True,
                               text=True, check=True, timeout=5).stdout
        if 'state = running' in state and 'com.apple.notificationcenterui.main' in state:
            print(f'CI_NOTIFICATION_AGENT_READY {service}', flush=True)
            return
        time.sleep(.1)
    raise SystemExit('Notification Center did not reach running state with its Mach service')


if __name__ == '__main__':
    main()
