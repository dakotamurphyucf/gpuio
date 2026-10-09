# Password privacy walkthrough

Read [password_preview.ml](password_preview.ml) and its [interface](password_preview.mli). [pages.ml](pages.ml) mounts it in **Text editing → Options** using synthetic seed `Sample-λ-42`.

`B` aliases `Bonsai.Cont`, `V` aliases `Gpuio_bonsai.View`, and `Editor` aliases `Gpuio_eio.Text_input`. `graph` hosts the state and controller computations. Palette, configuration and toggles are reactive values rather than ordinary fixed OCaml values. Each `let%arr` reads current inputs and derives its result when they change; `V` describes the UI while the editor adapter connects it to native state.

Reactive toggles own revealed/read-only/disabled/loading (false) and edit-menu enabled (true); `B.state false` owns submitted status. Configuration `let%arr` maps reveal state to Text_input privacy Password Hidden/Revealed and explicitly supplies Password content hint. Editor.create creates one retained single-line placement. The reactive on_submit callback ignores its Submission payload and sets only a boolean, so this component does not display or log submitted plaintext.

Native reveal-control activation runs the supplied `toggle_revealed` effect, updates the model, and makes configuration `let%arr` derive Revealed privacy; the outer `let%arr` derives explanatory text and GPUIO updates the existing editor. Constructing the callback/effect does not reveal it immediately. Native Enter submission similarly runs set_submitted true, then derives the acknowledgement readout. Neither path creates authentication or server work.

[`Input_frame`](../../lib/core/input_frame.mli) wraps Editor.view with Key prefix, native clear button and optional checking spinner/busy semantics. Applications own reveal state via on_reveal. Clear requires editable/nonempty/noncomposing text and is suppressed by loading; it records undo and returns editor focus. Loading leaves typing available, so “checking” is visual demo state rather than an activation/validation gate. [`Editor_menu`](../../lib/core/editor_menu.mli) supplies native Cut/Copy/Paste/Select all using current editor policy without synchronous OCaml edit callbacks.

Edit, reveal, hide, clear and Undo to exercise the same editor/history. Hidden password text blocks copying/cutting; revealed text can be copied. Right-click or Shift-F10 opens the enabled native menu. Read-only and disabled are distinct policies. The content hint is system metadata, not guaranteed autofill, privacy enforcement or validation; see [content hints](content_hint_preview.md).

GPUIO owns native buffer/history/focus/privacy behavior, the Eio adapter owns its lease and Bonsai owns options/status. No asset registration or background worker exists. Password display is not memory erasure: the synthetic source seed remains caller data. Adapt with actual authentication effects and explicit secret-handling policy; avoid mirroring plaintext into readouts/logs. Store only the state your application needs, and do not assume native text/history survives destruction/remount.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not executed checks for this documentation change. There is no standalone executable or self-test for this component. Compilation does not establish native keyboard/IME/focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
