# Desktop drag diagnostic

Read the [source walkthrough](main.md) for the model/functions, Bonsai syntax,
native ownership, interaction trace and optional diagnostics.
Source: [main.ml](main.ml); dependencies/PPX: [dune](dune).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/drag_drop_desktop/main.exe
```

This fixture requires driver-created file/status arguments; use the harness below.

Build the native driver without running it:

```sh
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_drag_drop --no-run
```

Set `DRAG_DRIVER` to the exact Cargo executable path printed by that command.
The harness creates the existing file and release-status file and drives AppKit:

```sh
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER" --desktop
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER" --reenter
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER" --cancel
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER" --remove-source
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER" --close-source
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER" --shutdown
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER" --close-internal
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER" --shutdown-internal
```

These are macOS foreground tests requiring Accessibility permission. They verify
owned children and unchanged fixture contents and reap them after testing.
No dragged file is opened or moved by the example. Read the guide's scenario
identity/lifetime distinctions before adapting this diagnostic into an app.
