# Retained input frames

OCH-41, 2026-10-01. Local implementation under validation; physical macOS
appearance/input/IME/accessibility acceptance remains open.

`View.input_frame` (also `Gpuio_bonsai.View.input_frame`) decorates a direct
`text_input` view. It keeps the existing input node, controller and Rust editing
entity. Leading/trailing ordinary views, loading and reveal controls can change
without replacing the draft, directed selection, undo history or composition.
Apply `View.editor_menu` after the frame helper when both are wanted.

```ocaml
Editor.view editor
|> V.input_frame
     ~config:(Input_frame.create ~clear_label:"Clear password" ~loading ()
              |> Or_error.ok_exn)
     ~leading:(V.text "Key")
     ~on_reveal:(fun () -> toggle_revealed)
|> Or_error.ok_exn
|> V.editor_menu
|> Or_error.ok_exn
```

Here `editor` is the Eio/Bonsai text-input controller and `toggle_revealed` is an
application effect. The application feeds `Password Hidden` or `Password Revealed`
back through the existing input config. The gallery's password card supplies a
complete example. Ordinary input styles control the outer frame; its native
default is a horizontal, centered flex row with a six-logical-pixel gap. The editor
gets the remaining width. Prefix/suffix views use their normal style, event and
focus APIs. Supply explicit labels for interactive decorations.

## Controls and ownership

- Clear is opt-in, single-line only, visible only for nonempty editable text when
  not loading or composing. It records one native undoable clear, resets the caret
  and scroll, and focuses the same field. It has an accessible Button/Click action
  and preserves pointer focus without adding a Tab stop. It is a native action,
  not an OCaml replacement effect based on an older snapshot.
- Clear callbacks bind node identity, weak native focus identity, text revision
  and a frame/edit-policy activation revision. Invocation rechecks current frame,
  read-only/disabled state, visibility/modal eligibility, composition and identity.
  A draft change, editor replacement or intervening frame/edit-policy change
  invalidates old actions. Unrelated tree transactions do not invalidate them.
- Loading uses the existing bounded native spinner/animation implementation and
  sets the editor's accessible busy state. It suppresses clear but leaves typing
  and password reveal available. The application owns the work/loading Boolean.
- Reveal is opt-in through `on_reveal`, accepted only on password inputs. It is a
  regular focus-preserving button that emits a unit intent to the application;
  reduce it against current application state. Rust does not own a separate reveal
  Boolean. Read-only permits reveal; disabled input gates its descendants. Labels
  default to “Show password” and “Hide password”. Password accessibility/clipboard
  rules remain those of the [privacy contract](plain-input-extensions.md).
- Multiline inputs accept prefix/suffix and loading. A clear control or password
  reveal on an unsupported editor mode is rejected during public construction.
  Combobox and picker query fields do not acquire this helper implicitly.

Removing the helper produces the same input with no frame slots. Calling it again
replaces the frame description; it does not stack frame owners. Theme/style and
slot changes must not become implicit text replacements.

## Protocol, admission and resources

Paired `Set_editor_frame`/`SetEditorFrame` is operation 74 in the unpublished
protocol epoch 3. Both runtimes must be rebuilt together. The optional frame
payload contains optional clear label, loading Boolean and gap; legacy
`EditorConfig` bytes are unchanged. Labels are nonblank UTF-8 without NUL, at most
4096 bytes; gap is finite in 0..256 logical pixels. Custom loading/reveal labels
travel through ordinary spinner/button nodes.

The input has four fixed private structural slots: leading, loading, reveal and
trailing, each with zero or one content child. Caller keys remain on their content.
The renderer mounts each content once and skips generic input-child rendering.
Admission validates the complete shape, including dirty ancestors of changed
children, before publishing a transaction. Loading requires a spinner; reveal
requires a button and password policy; clear requires a single-line input. Without
a frame, ordinary editors remain leaves. Picker query admission rejects frames.

The tree charges the frame and label, ordinary descendants use existing tree
budgets, and the spinner uses existing clock/resource ownership. There is no new
FFI callback type, synchronous OCaml paint/edit callback or fork patch. Native
editor observations still travel through the existing asynchronous event queue.

## Validation boundary

See [current evidence](../evidence/input-frame-och41.md). TestPlatform checks are
native retained-state/layout evidence, not OS keyboard, IME candidate-panel,
external AX, VoiceOver or physical GPU presentation acceptance. Formatting,
content hints and ordinary text-area search/layout controls remain separate
[plain-input work](plain-input-extensions.md).

A clear attempt checks empty text against the current format/edit filter through
the same exact replacement contract as an editor command. If rejected, it leaves
the draft, directed selection, revision and undo history unchanged. It does not
call the engine's unguarded `clean` helper, which can collapse selection after a
rejected edit. See the [clear-action regression evidence](../evidence/editor-escape-och41.md).
