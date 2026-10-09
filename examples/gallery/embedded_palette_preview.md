# A command browser inside the workspace

This example keeps a searchable command list beside an editable note. Selecting
an action leaves the browser open. It demonstrates an ordinary application
component written entirely in OCaml; no Rust extension is needed.

```sh
./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Feedback → Commands in your workspace**. Press **Focus command browser**,
search for `Add`, and confirm repeatedly to increase the count. Clear the query
to find the other commands. **Hide command browser** hides the mounted widget;
showing it again retains its query. This example uses the macOS-qualified native
palette path; Linux desktop qualification remains separate. Ordinary launch does
not run the optional automation in `scripts/test_gallery.py`.

## Read the component

Start with the [interface](embedded_palette_preview.mli), then
[`component`](embedded_palette_preview.ml). It receives a window, a reactive
application color palette and a Bonsai graph. Here `Palette` means the gallery's
[presentation helpers](palette.mli); `Command_palette` means the native chooser.
The [Feedback page](feedback_page.ml) constructs this component. The
[application](application.ml) owns the window and Eio runtime.

`Bonsai.Cont` creates a graph of values that are recomputed when their dependencies
change. `state_machine0` stores the step count and applies the `Add | Reset`
actions; its `inject` function produces an effect that requests a state change.
A second state machine counts cancellation requests, and `B.toggle` stores the
hidden flag. `let%arr` reads these changing values and constructs the next view.
It does not run commands during rendering.

`Command.Registry` defines three stable command IDs. Two commands inject Bonsai
actions. The third uses native `Select_all`, which acts on the eligible workspace
editor rather than selecting the palette query. The note is owned by
`Gpuio_eio.Text_input`; its native text/selection are not duplicated in the count
model. Commands resolve inside `V.command_scope`.

`V.command_palette ~presentation:Embedded` puts the chooser in normal layout.
Its native query, highlighted row and keyboard navigation remain inside GPUIO.
`Controller.create` supplies a stable widget key and receives native snapshots
through `Controller.observe`. The focus button issues an asynchronous `Focus`
command; this demonstration discards its result, so a production UI that needs
to report focus failure should inspect the returned result.

## Follow an action

Confirm **Add a step** in the native list. GPUIO resolves the command and delivers
its invocation to OCaml. `inject Add` updates the count; Bonsai recomputes the
dependent `let%arr`, and the next view changes **Steps added**. The stable chooser
key keeps the query session intact. Accepting that view is distinct from its
eventual physical display on the screen.

With `Clear_query_first`, Escape first clears a nonempty query. A later Escape
requests cancellation, incrementing the cancellation count. The component does
not unmount itself on that request. `Selected` and `Outside_pointer` dismissal
observations do not change this example's model. A native editor's IME composition
has its own Escape handling; this walkthrough is not an IME acceptance report.

There is no background search or file I/O here. The Eio-backed controllers bind
resources to the window/component lifetime. Hiding uses a style flag and retains
the mounted session; leaving the Feedback branch deactivates the component. Do
not use hidden state as a substitute for cancellation of unrelated application
tasks. The [external search example](external_palette_preview.md) shows explicit
task ownership instead.

To add **Subtract a step**, extend `Action.t` and its reducer, define a stable
command ID/registry entry, then add that ID to the chooser's command list. Keep
the reducer pure, IDs unique and the controller outside `let%arr`. See the
[embedded palette contract](../../docs/design/palette-embedded.md),
[command interface](../../lib/core/command.mli) and
[controller interface](../../lib/eio/palette_controller.mli).
