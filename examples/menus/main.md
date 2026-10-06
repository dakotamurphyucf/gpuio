# Menus, command scopes and native context popups

This example uses only public OCaml APIs. From the repository root, build and
start it with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/menus/main.exe
./scripts/gpuio exec _build/default/examples/menus/main.exe
./scripts/gpuio exec _build/default/examples/menus/main.exe --platform-popup
```

The optional flag selects an AppKit context popup on macOS and a drawn fallback
on Linux. Only the draft's context menu changes presentation. The ordinary
dropdown stays drawn, and `menu_bar` uses the platform default. Right-click the
draft or focus it and press Shift-F10. Escape dismisses without invoking an
action. Full platform acceptance is tracked in OCH-41/OCH-17; this walkthrough
does not imply Linux desktop qualification.

## Source map and reading order

This example has one component in [main.ml](main.ml). Read the immutable IDs and
menu definitions first, then `component` (Bonsai state followed by the View), and
finally `let ()` (startup and optional test coordination). [dune](dune) declares
the public libraries and Jane Street/Bonsai PPX used by the executable.
The relevant contracts are [Menu](../../lib/core/menu.mli),
[View](../../lib/core/view.mli), [Command](../../lib/core/command.mli), and
[App](../../lib/eio/app.mli).

## Data and command definitions

`Command.Id` values name actions independently of their label. `file_menu` and
`actions_menu` are immutable trees of references, separators and nested menus.
They do not own callbacks or copies of the application model. A registry in the
nearest enclosing `View.command_scope` supplies command labels, enabled state,
shortcuts and behavior. Duplicate IDs in different nested scopes deliberately
resolve according to the current presentation/focus context.

The outer Run increments by one; the nested scope replaces that ID with Run ten.
The Copy command is a native edit action, so it uses the eligible native editor
and its selection. AppKit presentation does not turn an OCaml closure into a
native synchronous callback.

## Bonsai state and effects

`component` has a graph argument because it constructs a Bonsai computation.
`B.state` owns the run count and enabled flag. The `let%arr` expression combines
their current values into a View and current registry. Run's callback returns
`set_count`; checkbox interaction returns `set_enabled`. These effects update
the model, after which Bonsai recomputes the dependent View values.

`phase` and `observed` support the automated render-acknowledgement exercise.
`B.Edge.on_change` records that Bonsai has observed a phase. They are test
coordination, not required application architecture.

## GPUIO views and editing ownership

`Gpuio_eio.Text_input.create` binds a native editing session to the window and
Bonsai graph. Its text, selection and undo remain native; the View is obtained
with `Gpuio_eio.Text_input.view`. The context wrapper preserves that child while
changing menu presentation. Keep this placement stable: moving an editor to a
different structural location may replace its session.

The surrounding command scope owns padding and spacing. Buttons, menu bar and
dropdown reference the registry rather than duplicating application behavior.
The checkbox changes availability; the key on the inner command scope provides
stable identity while it remains mounted.

## Application startup and scoped work

`App.run` provides Eio capabilities and the application lifetime. `open_window`
mounts the component. Close window returns an effect that requests window
closure; ordinary applications can use this same pattern.

`--self-test` starts one task in the application scope, advances four phases,
waits for native frame acknowledgements, checks increasing revisions and closes
the window. It uses the Eio clock and a bounded timeout, with completion
reported as an effect. This validates publications and teardown, not actual
right-click, OS menu tracking, keyboard selection or screen-reader behavior.

To adapt this example, add a stable command ID, create its registry entry, and
reference the same ID from any desired menu or command button. Keep state in
Bonsai and I/O in a cancellable Eio scope; menu navigation must remain native.

## One interaction, end to end

Choose Run from the context menu. Rust checks that the owner and current command
are still eligible and queues an invocation to OCaml. The registry callback
returns the Bonsai `set_count (count + 1)` effect. Applying it updates the state;
`let%arr` then produces the new `Total runs` text and Run label. GPUIO reconciles
those values into an accepted native transaction, and GPUI renders the updated
window. Acceptance and physical display are separate events; this example does
not use an acknowledgement as a frame-rate measurement.

For a small modification, change the outer Run callback to `set_count (count +
2)`. Its menu entries, button and shortcut will all share the new behavior,
while Run ten in the nested scope remains unchanged. Keep the command IDs
stable when changing labels or counts; menu positions are not command identity.

## Native popup lifecycle probes

`--popup-close-test`, `--popup-retire-test` and `--popup-invalidate-test` are
private acceptance controls used together with `--platform-popup`. Three
seconds after startup, their scoped Eio task respectively requests window close,
removes the context owner from the View, or changes Run's command availability.
These flags are not needed in an ordinary application. The physical driver
first verifies that an AppKit menu is open, then checks the corresponding
transition. A later checkbox roundtrip makes the stale-command check observe a
subsequent accepted application publication rather than an old text snapshot.

```sh
python3 scripts/test_native_popup_macos.py
python3 scripts/test_native_popup_macos.py --lifecycle close
python3 scripts/test_native_popup_macos.py --lifecycle retire
python3 scripts/test_native_popup_macos.py --lifecycle invalidate
```

These tests require macOS Accessibility permission, open a foreground window,
and reap their child process on success or failure. The Copy check preserves
and restores clipboard contents. Their AppKit coverage does not establish
VoiceOver behavior or Linux graphical acceptance.

An independent build against staged installed libraries is available with:

```sh
python3 scripts/test_extension_consumer.py --example menus \
  --workspace scratch/menu-consumer
python3 scripts/test_native_popup_macos.py \
  --binary scratch/menu-consumer/consumer/_build/default/main.exe
```

This installs only into that local workspace, leaving opam switches unchanged.
The same `--binary` option works with each lifecycle mode above.
