# Native file dialogs

Read the [source walkthrough](main.md) for the model/functions, Bonsai syntax,
native ownership, interaction trace and optional diagnostics.
Source: [main.ml](main.ml); dependencies/PPX: [dune](dune).

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/file_dialogs/main.exe
_build/default/examples/file_dialogs/main.exe
```

Launch opens a native window. No external service or assets are required.

Open reads a selected UTF-8/NUL-free text file with a 64KiB limit; save only selects
a destination and writes nothing. Optional native diagnostics:

```sh
_build/default/examples/file_dialogs/main.exe --capabilities-self-test
_build/default/examples/file_dialogs/main.exe --self-test
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_file_dialog --no-run
```

Set `PICKER_DRIVER` to the exact Cargo executable path printed by the last command:

```sh
python3 scripts/test_file_dialog_read.py --driver "$PICKER_DRIVER"
```

The macOS harness drives the real picker to select repository LICENSE, performs
explicit Eio read and reaps its child. Accessibility permission is required.
The capability test shows no picker; the other modes open native panels/windows.
