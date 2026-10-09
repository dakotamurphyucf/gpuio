# Reading the native editor application

[main.ml](main.ml) demonstrates a native single-line field and multiline composer
in two separate windows. In ordinary launch you type, select, undo/redo and submit;
the label records the last submission. The long middle section is an optional
command regression test, not additional interactive buttons. The
[README](README.md) gives build/run and native-test commands. No external assets
or network service are needed. Use the isolated toolchain in the
[development guide](../../docs/development.md); native launch requires a working
desktop. The [platform policy](../../docs/platform-release-policy.md) states the
macOS-first release scope and the separate Linux desktop qualification boundary.

Read the aliases and the first part of `component`, skip temporarily from
`B.Edge.on_change` to the final view, and then read the entry point. Return to the
self-test after understanding normal ownership. [dune](dune) links `gpuio.protocol`
in addition to Core, GPUIO, the Bonsai/Eio adapters, Bonsai and `eio_main`: the
self-test compares expert native node IDs. `ppx_jane` and `bonsai.ppx_bonsai` supply
the reactive and effect syntax. The smaller
[counter walkthrough](../getting_started/README.md) introduces state-only views.

`B` is `Bonsai.Cont`, `E` is `Bonsai.Effect`, `Input` is the value-contract module
`Gpuio.Text_input`, and `Controller` is its window-scoped Eio/Bonsai adapter. A
Bonsai graph is the persistent computation structure; a reactive value changes
as state or observations change. `component ~self_test ~completed ~mode window
graph` allocates three application states with `B.state`: `submission` starts
“Press Enter to submit”, `shown` starts true, and `mount_text` starts empty with
explicit `String.equal` equality. Each state call returns the reactive value and
a setter producing an effect. The local `started` and `submissions` refs are
per-component bookkeeping for the test, not native editing state; `completed` is
shared by the launcher to count both finished tests.

`Input.Config.create` validates the requested `Single_line` or `Multiline` mode,
label and placeholder. `~auto_focus:(not self_test)` prevents test auto-focus.
The immutable configuration becomes reactive via `B.return config` when passed
to `Controller.create`. `on_submit` is also reactive because it depends on the
current setter:

```ocaml
let on_submit =
  let%arr set_submission = set_submission in
  fun value ->
    E.Many
      [ E.of_thunk (fun () ->
          submissions := Input.Submission.text value :: !submissions)
      ; set_submission ("Submitted: " ^ Input.Submission.text value)
      ]
```

`let%arr` unwraps current values and derives a reactive result; it does not run
this handler. The callback receives an `Input.Submission.t`, an exact native
text/revision captured outside composition. `E.Many` combines the bookkeeping
and label-update effects. `E.of_thunk` defers the ref mutation until the effect
runs. Constructing a view/effect alone must not perform that mutation.

`Controller.create window ~config ~on_submit graph` represents one native editor
placement. Rust owns the draft, selection, IME composition and undo history.
Bonsai owns the application label and whether the editor is shown. The final
`let%arr` derives `View.column`: submission text, `Controller.view` when `shown`,
and a Close button. The editor is 400 logical pixels wide with padding and a
border built using validated `Style`/`Length`/`Color` APIs. Its `~initial_text`
value is a seed for the next native mount. Changing `mount_text` does **not**
replace a live draft. To edit live text, issue an explicit controller command.
Place a controller's view only once; duplicate native placement is rejected.

For an ordinary interaction, type “Hello” and press Enter. Native code handles
text, caret and composition synchronously; an observation is later delivered
asynchronously to the OCaml controller. At submission, native code captures the
exact current text/revision and sends a submission event. `on_submit` extracts
“Hello”, records it in the test list and requests `submission = "Submitted:
Hello"`. Bonsai derives a new view and GPUIO submits it for native rendering.
This application leaves the editor draft intact: observation and label updates
are not clearing commands. In the composer, Shift+Enter inserts a newline;
Tab/Shift+Tab navigate controls. An accepted transaction or command reply is
not evidence that a physical frame or an IME candidate panel was presented.

The [value interface](../../lib/core/text_input.mli) defines abstract revisions,
snapshots, submissions and typed command errors. `Selection` uses **UTF-8 byte
offsets**, preserves anchor/head direction and validates character boundaries
when applied. In `é界`, byte boundaries are 0, 2 and 5; the test selection
`anchor:5 head:2` selects `界` backwards. Do not interpret those as character
indices or UTF-16 offsets. See the
[controller contract](../../lib/eio/text_input.mli) for commands and lifetime.

## The optional self-test

From the repository root, after building:

```sh
_build/default/examples/text_input/main.exe --self-test
```

This opens two background-requested windows (`~focus:false`), disables auto-focus,
runs asynchronous native commands and closes both. It is command/lifecycle
validation; it does not substitute for real foreground typing, candidate-panel
or VoiceOver testing. Success prints `GPUIO_EDITOR_PUBLIC_OK` and its scenario
summary. Assertions and `expect` deliberately fail the diagnostic on unexpected
results; application code should instead handle `Command_error.t` explicitly.

`observed = B.map editor ~f:Controller.snapshot` derives the optional last native
snapshot. It is `None` before mount. `B.Edge.on_change` subscribes to changes using
`Option.equal Input.Snapshot.equal`; its callback starts the test only after an
observation exists, guarded by `started` so later changes do not restart it.
`B.Clock.sleep` supplies timer effects for bounded waits. `B.peek editor` lets
later effect steps obtain the current reactive controller, returning `Active`
or `Inactive`, rather than using only a controller captured before remount.

Inside that callback, `E.Let_syntax` changes the meaning of the syntax:
`let%bind` runs an effect and passes its completed result to the next step.
`>>= expect` checks the typed `Result` reply, turning errors into diagnostic
failures. Commands wait for native replies; they are not synchronous local
mutations. The sequence exercises these boundaries:

1. `replace` seeds `é界` with selection `End` and undo `Reset`; `select` applies
   the backward byte range. Replacement with `Record`, `Undo` and `Redo` checks
   restoration of both text and selection.
2. `Controller.submit` reads an exact native submission and invokes `on_submit`.
   The test checks that the single recorded text is `changed`.
3. Replacement guarded by the first snapshot revision fails `Stale_revision`
   after later edits. Clearing succeeds explicitly; changing `mount_text` to
   `recovered é界` then reading native text verifies that the live draft remains
   empty. `read_snapshot` asks native code for current state, unlike consulting
   a potentially older observation.
4. `replace_if_unchanged editor unchanged` checks both the observed lease and
   revision: the first reset succeeds and reusing the old snapshot fails.
5. Setting `shown` false removes native placement. A command against the old
   controller fails `Stale_editor`. Restoring `shown` mounts a new editor using
   `mount_text`; undo, selection and IME state from the destroyed editor do not
   survive.
6. `await_remount` sleeps 50 ms between at most 41 observations and checks that
   the expert node ID changed. The old snapshot cannot reset the new lease even
   if revisions coincide. The fresh snapshot can reset it to `fresh reset`.
   Completion increments `completed` and force-closes that window.

The `Input.Expert.node`/`Gpuio_protocol.Node_id.equal` inspection is diagnostic
identity checking; ordinary application logic uses the abstract controller and
guarded commands. The waits are native-test settling intervals, not a recommended
production polling pattern. No application file/network work is started.

## Runtime and a small adaptation

The entry point detects `--self-test` from `Sys.get_argv`, creates a completion
counter, and calls `App.run`. The runtime owns GPUI on the OS main thread and one
OCaml Eio UI domain for initialization, graphs and effects. `List.iter` opens
one 460×320 logical-pixel window per mode with separate components. Window scopes
own controller subscriptions and pending command cleanup. Both the Close button
and diagnostic use `App.Window.close`, a force-close that cancels the window
scope; use `request_close` with a decision handler for unsaved-draft confirmation.
When the last window closes, the default runner exits, and self-test checks that
both components completed. Read the [application contract](../../lib/eio/app.mli)
for ownership, shutdown and propagation of callback failures.

To make a submitted message clear after application acceptance, extend
`on_submit` to run the acceptance operation and then
`Controller.clear_if_unchanged` with the original `Submission.t`. You will need
to arrange that callback with access to the controller (rather than referencing
the later `editor` binding directly). Handle `Stale_revision` by retaining newer
user typing and `Stale_editor` by leaving a replacement editor untouched; active
composition may also reject clearing. The adapter requires the exact submitted
lease/revision even if the text later looks identical. For I/O acceptance, pass
Eio capabilities and choose an application/conversation scope intentionally;
do not tie a long-lived send to a transient editor placement or put I/O in
`let%arr`. This demo has no external-send success, streaming or persistence to
claim, and its snapshots/submissions contain real draft text.
