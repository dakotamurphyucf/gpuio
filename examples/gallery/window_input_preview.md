# Focused-input metadata walkthrough

[window_input_preview.ml](window_input_preview.ml) has no separate interface.
[runtime_page.ml](runtime_page.ml) mounts it on Runtime. `B` is `Bonsai.Cont`, `E`
`Bonsai.Effect`, `V` `Gpuio_bonsai.View`, `App` `Gpuio_eio.App` and `Editor` its text-input
adapter. Graph hosts reactive notice/editor controllers; `let%arr` derives current UI and
command effects.

A local editor helper creates three separate single-line native placements: ordinary draft,
Hidden password with synthetic demo-secret, and read-only field. These configurations are
constants; native Rust owns their text/focus/history. Notice starts with Primary+Shift+I
instructions. One registry command binds that chord at Override priority, so inspection can run
while an editor keeps focus instead of clicking a button away from it.

Native shortcut delivery invokes `inspect`, which asynchronously calls
`App.Window.focused_input`. Reply Ok Some supplies typed kind and exact mounted identity;
`Window.Input.same_text_input` compares it to the captured controllers’ last snapshots. The
returned effect updates notice, and `let%arr` derives a new readout for native reconciliation.
Constructing the query effect does not inspect focus immediately. Identity matching may fall
back to “Another native input” if snapshots have not caught up; it never matches merely by
label/text.

Ok None reports no eligible native input; errors are formatted explicitly. The
[window contract](../../lib/core/window.mli) describes metadata identity, not a value-read
capability. Querying focus does not read password/plaintext, selection or clipboard. A delayed
result describes native query execution, not guaranteed focus when the notice is later painted.
Read-only fields can still be text inputs.

Focus each field and invoke Command+Shift+I on macOS/Ctrl+Shift+I on Linux to compare kinds;
typing remains native. Bonsai owns notice, adapters own editor leases and GPUIO owns focus/query
delivery. No worker/resource scope exists. Adapt with exact lease identity checks and handled
errors; do not treat focused metadata as authorization to read values or infer current focus
indefinitely from one observation. Synthetic password seed remains caller data; display privacy
is not memory erasure.

## Run

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These commands were not executed for this documentation change. The component has no standalone
executable/self-test; compilation does not establish native interaction or platform acceptance.
See [gallery instructions](README.md) and [development](../../docs/development.md).
