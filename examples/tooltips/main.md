# How `main.ml` separates tooltip visibility from editor lifetime

[Example README](README.md) · [Source](main.ml)

The first tooltip delegates hover and focus visibility to the native runtime.
The editable tooltip uses application-controlled visibility while retaining its
native editor when hidden. This distinction is the central implementation lesson.

## Graph state and native editor state

`B` abbreviates `Bonsai.Cont`. `B.state false graph` creates the reactive `open_`
value and `set_open` effect constructor. A second state pair starts `mounted` at
`true`. The graph owns these state nodes for this component's lifetime.
`Gpuio_eio.Text_input.create` connects a single-line editor controller to the
window, using a constant configuration wrapped in `B.return`. Its text, selection,
revision, and focus live in native editor state, rather than an OCaml string model.

Opening `B.Let_syntax` enables `let%arr`. Its bindings read current reactive values
and build the view again when those values change. A setter such as
`set_open true` produces an effect; it does not mutate state merely because a
view was constructed. The checkbox attaches a setter effect to `on_toggle`.

`Tooltip.Config.create` validates the label and open-state policy. The first
`View.tooltip` uses the default managed policy and an anchor button with
`Bonsai.Effect.Ignore`: clicking the button has no application action, while
native hover and focus still operate the tooltip. Native focus opens promptly;
hover uses the configured delays. Escape dismisses it.

The second config uses `Controlled open_`. `on_open_change:set_open` turns native
requests into application state changes. Its stable key is `note`, and its content
includes `Text_input.view input`. Hiding it keeps this content mounted. Setting
`mounted` to false omits the entire tooltip node, retiring that native editor.

## A concrete interaction

Toggling “Show editable note” sends an event to the checkbox's effect. Bonsai
updates `open_`; `let%arr` rebuilds the controlled config; the runtime shows the
existing tooltip and editor. Type into the editor, close the tooltip, and reopen
it: visibility changed, but the retained native editor still has its text and
selection. A native dismissal request also travels through `on_open_change` and
`set_open`, so the checkbox reflects the accepted state.

Hidden content cannot receive focus. Retention permits commands such as replacing
its text, but `Text_input.focus` reports `Focus_blocked` while hidden. After the
node is omitted, commands through the old controller report `Stale_editor`.
Use these typed results to distinguish temporarily unavailable focus from a
retired editor instead of treating every command failure as the same condition.

## What the self-test does

`B.map input ~f:Text_input.snapshot` observes whether the native editor has become
available. `B.Edge.on_change` compares observations with typed snapshot equality.
Its callback is itself built with `let%arr`, capturing the current controller,
setters, and `B.Clock.sleep` effect. A `started` reference ensures the diagnostic
sequence starts once after a snapshot is present.

Inside the callback, `E.Let_syntax` enables `let%bind` and `>>=` to sequence
`Bonsai.Effect` commands and consume their results. Unlike `let%arr`, these bind
operations wait for the preceding effect's result. `expect` converts a successful
command result into a snapshot or raises with a typed error's sexp; `expect_error`
asserts the expected error using typed equality.

The sequence checks hidden focus denial, replaces text with `Retained é界` using
`selection:End` and `undo:Reset`, opens the tooltip, then focuses the editor. It
checks text, revision, selection, and focus. Closing and reopening repeats the
retention checks. Finally it removes the tooltip, expects `Stale_editor`, records
completion, and closes the window. The 100 ms sleeps allow native changes to
settle; they are diagnostic timing assumptions, not application synchronization
contracts. Success prints `GPUIO_TOOLTIPS_PUBLIC_OK`.

`App.run` owns the runtime and `App.open_window` mounts the graph in a
520 × 400 window. The ordinary Quit button closes it through an effect.
The diagnostic exercises programmatic commands and visibility; it does not
establish real keyboard, IME, hover-delay, or accessibility acceptance.

## Build, run, and adapt

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/tooltips/main.exe
_build/default/examples/tooltips/main.exe
_build/default/examples/tooltips/main.exe --self-test
```

Use the repository environment and a graphical session as described in
[development setup](../../docs/development.md). The executable's Dune stanza
includes `ppx_jane` and `bonsai.ppx_bonsai` for the syntax above.

Choose managed visibility for simple help. Choose controlled visibility when
other controls must share the same state, and accept native requests through an
effect handler. Keep a stable keyed content node if editor state should survive
hiding; remove the node when retiring it. Do not retain a controller and assume
it remains valid after removing its native view.
