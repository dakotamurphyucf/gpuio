# Native text input

This example opens two windows through the public Bonsai/Eio API: a single-line
input and a multiline composer. Rust owns editing state; Bonsai observes native
snapshots and submissions. The controls demonstrate explicit replacement,
selection, undo/redo and revision-guarded clearing.

Build without opening windows:

```sh
./scripts/gpuio build examples/text_input/main.exe
```

For a deliberately requested interactive session, run
`_build/default/examples/text_input/main.exe`. Enter submits, Shift+Enter inserts
a newline in the composer, and Tab/Shift+Tab navigate controls. The `--self-test`
mode checks two-window command behavior, UTF-8 selection, undo/redo, revision
rejection, unmount and closing. It also opens windows. Local foreground runs are authorized for fast iteration;
avoid unnecessary activation when a check can run in the background.

The separate Rust `native_editor` test additionally exercises real macOS
NSTextInputClient composition/commit and accessibility calls, clipboard,
grapheme deletion, native key policy, focus, and auto-grow. Compile it locally
without opening windows:

```sh
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_editor --no-run
```

CI executes the test on macOS and runs the non-Mac scenarios in informational
Linux GUI jobs. Physical IME candidate-panel and comprehensive screen-reader
validation are separate from these native callback checks.
