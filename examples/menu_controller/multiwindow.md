# Independent windows and native menu targets

This example opens two windows with independent Run counters, menus and text
editors. It demonstrates what belongs to each window and why a native editing
command must retain its original target while a menu is open.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/menu_controller/main.exe
./scripts/gpuio exec _build/default/examples/menu_controller/main.exe --two-windows
```

On macOS the default popup uses AppKit. Linux uses the drawn fallback; add
`--drawn` to request that renderer explicitly on either platform. The native
tracking details below describe AppKit, not Linux desktop qualification.

Read the [startup walkthrough](main.md) and [main.ml](main.ml) for startup, [multiwindow.mli](multiwindow.mli) for the
component boundary, then [multiwindow.ml](multiwindow.ml) for its implementation.
The ordinary single-window example has a separate [walkthrough](component.md).

## State and ownership

`main` calls `App.open_window` twice. Each call creates its own Bonsai graph and
passes its own `App.Window.t` to `Multiwindow.create`. No controller or mutable
reactive value is shared between graphs. The `name` argument supplies the window
label and initial editor contents; it is not a native resource identifier.

Bonsai owns three pieces of application state: the Run counter starts `0`,
status starts `Ready`, and first-editor visibility starts `true`. `B.state` returns a reactive value
and an effect that updates it; `B.toggle` supplies the Boolean state and toggle
effect. `let%arr` builds a derived view when these values change. Constructing
the graph is separate from evaluating that derived view. The `and` bindings
declare reactive dependencies, not threads. Constructing a callback effect does
not run it.

`Text_input.create` runs once per editor during graph construction. Rust owns
the mounted editor's live text, selection and undo state. `initial_text` seeds
it; rebuilding the view does not repeatedly replace the user's text. The two
controllers have distinct identities even though both windows use the labels
“First editor” and “Second editor.” Labels are accessible names, not global IDs.

Removing the first editor replaces its view with a text message. This retires
its native instance while leaving the second editor and menu mounted. Restoring
it mounts a new instance; do not treat that as persistence of a retired native
editor's unsaved state. See the [editor contract](../../docs/design/native-editor.md)
for application-owned persistence and native lifetime semantics.

## GPUIO views and commands

`V.command_scope` contains a registry local to the window. Both windows use the
same typed `run_id` and `copy_id`, but resolution occurs in their own scopes.
Run delivers an OCaml effect; Copy uses `Command.native ... Copy`, so native
selection and clipboard work stay on the native side.

`V.context_menu` wraps the two editors. Its stable key and observation callback
come from `Menu_controller.create`. The Open and Close buttons submit correlated
commands through that controller. Open is disabled until a native subscription
has been observed. The validated position is measured in logical pixels relative
to the window's content. An accepted Show response means the request was admitted;
it is not proof that pixels have appeared or that the user selected an item.

The local `menu_command` helper uses `E.map` to convert the controller reply
to status text, then `E.bind` to run the status setter after completion. These
are effect transformations, unlike Bonsai `let%arr` view derivation. The `focus`
helper uses `E.Let_syntax`/`let%bind` to await `Input.focus` and formats its typed
error or `applied` result. The local `editor label` helper creates each separate
controller with a constant `B.return` config and a name-prefixed mount seed.

The focus buttons call `Text_input.focus` and display its typed result. Activate
and Observe use the `window_command` helper to await `App.Window.command`: Observe shows native window activation,
which is different from an editor retaining a focus handle. A window can retain
its focused editor while the OS considers another window active. Activation
responses report current observed state; OS transitions can complete later.

The layout uses ordinary GPUIO text, buttons, a column, padding and gaps. Bonsai
provides the dependencies and effects; GPUIO provides presentation and the typed
native commands. Nothing here requires application Rust code.

## Follow a Run and a Copy

Open popup submits a Show request bound to this menu's current subscription.
Rust validates its owner and snapshots the native rows. Selecting Run produces
a command event; OCaml runs `set_count (count + 1)`, Bonsai recomputes the derived
view, and the resulting update changes the counter in that window only.

Copy has a different path: the popup captures the native editor target and focus
identity. Before applying a selected Copy, Rust checks that both still match.
If focus moved to the other editor, or the original editor disappeared, the old
popup cannot silently copy from the replacement target. Reopen the menu after
changing focus to act on the new target. Losing window activation cancels native
tracking without restoring focus into the inactive window.

Only one AppKit popup may track at a time. An inactive window's Show request is
Unavailable. Closing the owner must release its tracking state so the other
window can subsequently open a menu and dispatch its own commands.

## Runtime and adaptation

`App.run` owns the application runtime and asynchronous bridge delivery. This
example performs no application file or network I/O and creates no Eio fibers.
The Close button force-closes this demonstration window. Real applications with
unsaved work should use `App.Window.request_close` and a close-decision handler.

To add a window-local command, create a typed command ID, add its definition to
`commands`, and add `Menu.Command id` to the menu. Put its state in this graph.
For application-wide data, introduce an explicit shared application model and
per-window subscriptions rather than reusing a controller from another graph.

The macOS driver `scripts/test_menu_multiwindow_macos.py` uses public buttons,
native menu inspection and keyboard input. It preserves the existing clipboard
and closes/reaps its application. It exercises inactive-window rejection,
independent command routing, selected-editor changes/removal, activation loss,
and owner-window close with recovery. It does not establish VoiceOver behavior,
Linux GUI behavior, or cancellation before a queued popup begins tracking.

After building, launch the separate foreground diagnostic from the repository
root with `python3 scripts/test_menu_multiwindow_macos.py`. It requires macOS
Accessibility permission and accepts `--binary` for an already-built consumer.
No diagnostic was run merely by reading this guide. Public contracts:
[Menu_controller](../../lib/eio/menu_controller.mli),
[Text_input](../../lib/eio/text_input.mli),
[Command](../../lib/core/command.mli) and [App](../../lib/eio/app.mli).
