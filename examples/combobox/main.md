# Editable choices: application selection and native query

[main.ml](main.ml) lets you type a query, choose one of two reasoning modes, and
see the chosen ID and observed query separately. These are demo labels; no agent
or backend runs. Read the [README](README.md) for build/run commands and
[dune](dune) for Core/GPUIO/Bonsai/Eio dependencies and the Jane Street/Bonsai PPX.
Use the isolated setup in the [development guide](../../docs/development.md).
There are no external assets. Native launch needs a desktop; the
[platform policy](../../docs/platform-release-policy.md) separates the
macOS-first release from Linux build checks and desktop qualification.

Read `id` and `options`, then the initial state/configuration and final view in
`component`, then the launcher. Return to `B.Edge.on_change` for the optional
self-test. `id` validates `Choice.Id.t` strings. `options` creates immutable
choices with stable IDs `fast` and `deep` and validates their collection. Labels
and positions are not identity; duplicate IDs are invalid. `Or_error.ok_exn`
treats these hard-coded configuration errors as programming failures. Handle
validation results for external data; see [Choice](../../lib/core/choice.mli).

`component ~self_test ~completed window graph` constructs a persistent Bonsai
computation graph. `B.state None` owns the application selection, a
`Choice.Id.t option`; `B.state true` owns whether the editor is placed. Each
returns a reactive value and a setter producing a deferred effect. A reactive
value represents a computation that can change; it is not an ordinary OCaml
option until a `let%arr` expression reads its current value.

```ocaml
let on_select =
  let%arr set_selected = set_selected in
  fun selection -> set_selected (Some (Combo.Selection.id selection))
```

`let%arr` derives a reactive callback from the setter. The callback receives
`Combo.Selection.t`, an intent carrying both the enabled choice ID and the exact
query snapshot at activation. It requests application state change; creating
the effect does not execute it. The separate `config` computation reads
`selected` and creates a validated `Combo.Config.t`. Default substring filtering
matches Unicode-lowercased labels against native query text in collection order;
it does not normalize Unicode or fold accents. Enter requests a highlighted
choice rather than submitting arbitrary free text. See
[Combobox](../../lib/core/combobox.mli) for these contracts.

`Controller.create window ~config ~on_select graph` allocates one window-bound
controller. Rust owns query text, caret, IME composition, filtering interaction
and popup state; Bonsai owns selected ID and the available choices. The final
`let%arr` reads the controller, selected ID and visibility and derives a
`View.column` with padded spacing, a 36-logical-pixel-high `Controller.view`,
status text and Close. `and` declares additional reactive dependencies, not
parallel threads. `Option.value_map` renders `none` for absent application
selection and an empty query before a native snapshot exists.
`Controller.snapshot` is the latest asynchronous observation, not an immediate
native read or replacement command. Place this controller view only once.

For a concrete interaction, type `deep` and activate “Detailed reasoning”.
Native code updates the query/filter and sends observations; the query line
updates from `Input.Snapshot.text`. Activation sends a `Selection.t` to OCaml.
`on_select` returns `set_selected (Some deep_id)`, Bonsai updates selection, and
the derived config and “Selected: deep” line return to native rendering through
GPUIO. The handler does not replace query text with the option label: selection
and draft are independent. Actual physical presentation occurs separately from
transaction admission. Composition remains native and prevents choice activation
while an IME mark is active.

## Optional command diagnostic

After building, run from the repository root:

```sh
_build/default/examples/combobox/main.exe --self-test
```

This opens a window and retains `~auto_focus:true`; it is not a background-only
check. Success prints `GPUIO_COMBOBOX_PUBLIC_OK` with its scenario summary.
`B.map editor ~f:Controller.snapshot` derives the observation.
`B.Edge.on_change`, using `Option.equal Input.Snapshot.equal`, starts once the
first snapshot exists. A component-local `started` ref prevents repeated starts.
The normal application has no other lifecycle callback or application I/O task.

Inside the callback, `E.Let_syntax` makes `let%bind` run an effect and wait for its
result before continuing; `>>= expect` turns typed command errors into diagnostic
failures. First, `Controller.replace ~selection:End ~undo:Reset "é界"` receives a
native snapshot. `Combo.Expert.selection` constructs an intent for `deep` using
that snapshot. It **does not** simulate native keyboard choice activation or
invoke `on_select`; it tests the boundary value for the conditional command.
`replace_if_unchanged` replaces the query with “Detailed reasoning” only for
that exact native lease and revision. Reusing the intent returns
`Stale_revision`; `Undo` restores `é界`. Hiding the placement with `set_shown
false`, then waiting 100 ms via `B.Clock.sleep`, makes a subsequent `focus` on
the old controller return `Stale_editor`. The diagnostic sets the shared
completion ref and force-closes the window. Its assertions are test policy;
real applications should handle [typed errors](../../lib/eio/combobox.mli).

The launcher reads `--self-test`, calls `App.run`, mounts one 440×320 logical
pixel window, and verifies completion after runtime exit. GPUI owns the OS main
thread; Eio initialization, Bonsai and effects run on the OCaml UI domain.
Window/controller lifetimes own native subscriptions and pending command cleanup.
There is no file/network work, external cancellation or asynchronous search in
this demo. The Close effect uses force-close `App.Window.close`, cancelling the
window scope. Use `request_close` and a close handler if your application needs
a decision; see [App](../../lib/eio/app.mli).

To show the chosen label as query text, extend the selection handler to accept
the application ID and issue `Controller.replace_if_unchanged` with that same
`Selection.t` after acceptance. Arrange a callback with controller access rather
than referencing the later `editor` binding directly. Preserve newer typing on
`Stale_revision`, replacement editors on `Stale_editor` and active composition
on `Composing`; do not replace from a later unguarded snapshot. Adding a third
mode is simpler: add a unique ID/label to `options` and keep every present
selection a member of the full collection, even when filtering hides it.
For remote search, `Unfiltered` lets the application supply results, but the
application must manage ordering and Eio task cancellation itself.

The separate Rust `native_controls` scenario exercises real filtering, keyboard
selection dispatch, accessibility callbacks and macOS composition. This OCaml
self-test covers native command acknowledgements and unmount behavior; neither
source review nor those acknowledgements establishes physical IME candidate-panel,
VoiceOver or Linux desktop acceptance.
