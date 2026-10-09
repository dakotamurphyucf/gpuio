# One command ID, several native presentations

[main.ml](main.ml) demonstrates a command registry, command buttons, nested shadowing
and primary-K shortcut. Read IDs/shortcut, component setup/registries, view and
launcher. [README](README.md) lists exact build/run commands; [dune](dune) links
Core/GPUIO/Bonsai/Eio and Jane Street/Bonsai PPX. No assets or external work are
needed. Use [isolated setup](../../docs/development.md) and the current [platform
policy](../../docs/platform-release-policy.md).

`Command.Id.of_string` validates run/copy/quit IDs; `Shortcut.create ~modifiers:
[Primary]` maps K to Command on macOS, Control on Linux. IDs are independent of
labels. The [command contract](../../lib/core/command.mli) requires unique IDs within
a registry but allows nested scopes to shadow them. Fixed configuration failures use
`Or_error.ok_exn`; external data should handle validation results.

`component` builds a persistent Bonsai graph: `B.state` owns a count that starts at 0
and an enabled flag that starts true. A Text_input controller owns one native Draft
placement; Rust owns its live text/caret/IME/history. Reactive state plus setters are
not ordinary values until `let%arr` reads them to derive a view. `and` lists
dependencies, not threads. Setter/effect construction does not execute interaction.
Diagnostic phase is an external `B.Expert.Var`; `Edge.on_change` with `Int.equal`
records observed phase through a deferred thunk, independently of interactive state.

The outer registry Run label includes count, Run is enabled when the user flag is
true and phase is not 1. Its shortcut invokes `set_count (count + 1)`. Native Copy
executes against eligible focused native editor without OCaml editing roundtrip. Quit
returns a force-close thunk. Inner registry shadows only run with label Run ten and
count + 10. The `View.command_scope` key `inner` retains scope identity; phase 3
removes that scope. Command buttons reference IDs and resolve
label/availability/behavior from their scope rather than duplicating callbacks.
Checkbox modifies outer enabled state; inner Run is independently enabled.
Padding/gaps are validated logical pixels.

Focus inner Run ten and activate Primary-K: native focus/scoping resolves the inner
registry, delivers its invocation asynchronously, callback returns setter effect,
Bonsai updates count and derives all current command labels/counter text, and GPUIO
submits native update. Focus outer button/editor to resolve outer Run. Native
availability/focus/IME gates are distinct from declared shortcuts. Transaction/render
acceptance differs from physical screen display.

`App.run` owns GPUI on OS thread and OCaml Eio UI domain for graphs/effects, opening
a 660 × 440 window. Ordinary launch creates no application I/O producer. Close uses
`App.Window.close` and cancels window scope; use `App.Window.request_close` with a
decision handler for unsaved drafts. See [App](../../lib/eio/app.mli) and
[Text_input](../../lib/eio/text_input.mli).

`--self-test` starts an application-scoped Eio fiber with explicit clock/15-second
timeout. Local rendered sets phase, waits at 5 ms intervals for Edge observation,
bridges `App.Window.request_frame` to `Eio.Promise` and returns native revision.
Phases 0, 1, 2 and 3 exercise initial/disabled/re-enabled/removed scope; typed Int64
comparisons require increasing revisions. Completion effect checks result, marks
completed and force-closes; success prints GPUIO_COMMANDS_PUBLIC_OK. This does not
invoke keyboard shortcuts, Copy, real AX/VoiceOver or physical presentation; it
checks accepted view/render changes and teardown. Scope cancellation suppresses
queued result delivery; see [Scope](../../lib/eio/scope.mli).

To add Save, validate one stable ID, add registry callback and command button. For
asynchronous work, callback should start scoped Eio work with explicit capabilities
outside `let%arr`. For bursts/multiple producers, use a state machine reducing
increments against latest model rather than captured rendered counts. Keep native
edit commands on native editor ownership and do not interpret shortcut declaration
alone as tested platform key dispatch.