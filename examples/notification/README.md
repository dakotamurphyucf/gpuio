# Notification Lab

Public `Gpuio.Notification` values and `Gpuio_eio.Notification` operations with
real OS delivery. macOS is currently implemented; the Linux backend remains in
progress and currently returns `Unsupported`. This example is separate from
in-app toast components.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/notification/main.exe
python3 scripts/test_notifications_macos.py
```

The macOS test creates an app bundle in ignored `scratch/notification-os`, emits
its Info.plist through `Gpuio.Desktop_package`, applies an ad-hoc bundle signature
with the matching application identifier and registers that bundle. A copied
linker-signed executable alone is insufficient: on macOS 14.5 its signing identity
remained `main.exe`, and authorization was rejected before the prompt. No signing
certificate is needed for the local test; distribution signing is separate.

The test explicitly requests notification permission for **GPUIO Notification
Lab** if it is undecided. If previously denied, enable this application in System
Settings → Notifications before rerunning. It does not change Focus settings or
other applications' permissions. Native notification presentation is OS policy;
submission alone never proves that a person saw a banner.

The native walkthrough verifies queued actions before readiness, duplicate-tag
backpressure, default and named actions, user dismissal, content/action replacement,
explicit removal, a closed-window target followed by a replacement window, and
service cleanup. It only activates this lab's notification rows and reaps its app.
Actual notification actions may activate the app or open Notification Center.

To explore manually, run the packaged bundle left in the scratch artifact folder.
The normal mode starts delivery immediately; `--self-test` waits for **Ready for
actions** and exits after a three-minute deadline. **Close notification service**
permanently disables that service for the current app run; restart to use it again.
All application I/O runs through Eio, while native notification work stays in Rust.

See [the contract](../../docs/design/os-notifications.md) and
[acceptance evidence](../../docs/evidence/os-notifications-och28.md).
