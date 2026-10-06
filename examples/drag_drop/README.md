# Native drag and drop

Read the [source walkthrough](main.md) for the model/functions, Bonsai syntax,
native ownership, interaction trace and optional diagnostics.
Source: [main.ml](main.ml); dependencies/PPX: [dune](dune).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/drag_drop/main.exe
_build/default/examples/drag_drop/main.exe
```

Launch opens a native window. No external service or assets are required.

```sh
_build/default/examples/drag_drop/main.exe --self-test
```

This opens a window and validates programmatic bridge/render/lifetime behavior.
Read the walkthrough for what this test does and does not check.

For a real macOS gesture, build the native driver without running it:

```sh
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_drag_drop --no-run
```

Set `DRAG_DRIVER` to the exact Cargo executable path printed by that command, then:

```sh
python3 scripts/test_drag_drop.py --driver "$DRAG_DRIVER"
```

The harness launches this example with --gesture-self-test and posts the gesture.
It requires macOS Accessibility access and closes/reaps its child. This is separate
from --self-test, which injects no gesture.
