# Runtime diagnostics and window commands

Read [runtime_page.ml](runtime_page.ml) and its [interface](runtime_page.mli).
[pages.ml](pages.ml) routes Runtime to `component app window palette graph`. `B` is
`Bonsai.Cont`, `E` `Bonsai.Effect`, `V` `Gpuio_bonsai.View`, and `App` `Gpuio_eio.App`. `graph`
hosts reactive state; `let%arr` derives UI from current snapshot/notice values.

Snapshot starts None, notice “No native command requested”. `refresh` maps a setter into an
effect: `E.of_thunk` reads `App.diagnostics` and the latest stored window geometry only when
executed, then stores Some result. `B.Edge.lifecycle` runs it on activation; Refresh resource
counts invokes it explicitly. No recurring timer samples the application.

Native Observe this window activation executes the asynchronous window Observe command; its
reply runs `E.Many` to update notice and refresh diagnostics, then reactive derivation updates
native readouts. A refresh alone reads cached geometry; it does not issue Observe or guarantee a
new native measurement. Minimize requests native state and reports success/error; it is not
evidence that rendering or foreground input was qualified.

Choose a file invokes the native File_dialog.open_ effect and handles
cancel/selected-list/error. It reports count only, reads no file contents and starts no
filesystem worker. Resource readouts include
windows/documents/assets/charts/canvases/scopes/tasks/pending requests/queued commands and
summed reserved source bytes. These are application registration/accounting snapshots, not
process RSS or GPU memory. Logical content/outer dimensions come from optional window
observations.

The [focused-input preview](window_input_preview.md) and
[window-selection preview](window_selection_preview.md) are independent children with their own
commands/state. GPUIO owns native window/dialog behavior; Bonsai owns snapshots/notices and App
owns services/request lifetimes. There is no page resource scope to dispose. Adapt with explicit
sampling policy and meaningful diagnostic labels, handling asynchronous errors. Do not interpret
changing counts as total-memory measurement or file selection as file loading.

## Run

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These commands were not executed for this documentation change. The component has no standalone
executable/self-test; compilation does not establish native interaction or platform acceptance.
See [gallery instructions](README.md) and [development](../../docs/development.md).
