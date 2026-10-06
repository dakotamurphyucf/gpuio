# Feedback state: current command availability and one notification identity

[feedback_state.ml](feedback_state.ml) and its [interface](feedback_state.mli)
define a pure reducer for workflow feedback. It drives the stage/availability of
Advance and one saved-preview notification in [feedback_page.ml](../feedback_page.ml).
This is demo state, not a real save operation, background job or file/network request.
The helper creates no Bonsai graph, timer, toast or native resource.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Commands & feedback**, advance the preview, toggle command availability
and invoke Save preview repeatedly. Dismiss the notification or let its native
five-second timeout complete. There is no separate executable or model-specific
launch flag. The [gallery README](../README.md) covers prerequisites and platform
limits; this review does not add native toast, input or accessibility acceptance.

Read `Stage`, internal `t`, `initial`, `Action`, then `apply`. Public `Stage.t` is
Idle/Working/Halfway/Complete, with `Stage.label` giving human text. Internal
`Stage.next` cycles through those four values back to Idle. Abstract model `t`
stores stage, enabled Boolean, optional notification serial and next-identity counter.
It starts Idle, enabled=true, notification=None, serial=0.

Advance consults the **current** enabled flag and is a no-op when disabled;
Toggle_enabled flips it. Notify advances serial and replaces notification with that
new identity. Notify is independent of enabled: the flag controls workflow Advance,
not Save preview. Dismiss serial clears only an equal current notification, so a
late close/timeout for an older toast cannot remove its replacement. Leave clears
notification while retaining stage, enabled and serial. At `Int.max_value`, Notify
is a no-op rather than wrapping and reusing an identity.

## From command dispatch to native feedback

The page wraps `State.apply` with `B.state_machine0`, yielding a reactive model and
an injection effect. Its `let%arr` reads model/palette to derive GPUIO presentation.
The command registry marks Advance enabled according to `State.is_enabled` and
injects Advance on invocation. The reducer rechecks that latest availability, so
an already-captured invocation cannot bypass a later disabled state. A registry
entry for Save preview injects Notify; it does not write a document or filesystem.

The stage becomes Progress.Value determinate 0, indeterminate, determinate 0.5 or
1 for Idle/Working/Halfway/Complete. `State.notification` becomes zero or one
`V.toast` with key `preview-save-N` and `Toast.Timeout.after` five seconds.
Its `on_dismiss` callback captures serial N and injects Dismiss N. Native timeout,
hover/focus behavior and dismissal belong to GPUIO's toast; this model schedules
no OCaml timer. Typed toast policies are in
[toast.mli](../../../lib/core/toast.mli), and command composition in
[command.mli](../../../lib/core/command.mli).

For a concrete replacement, Notify creates toast 1. Before its timeout, another
Notify creates toast 2 and removes 1 from the derived view. A queued Dismiss 1
arrives: `Option.equal Int.equal` compares it with current Some 2 and keeps toast 2.
Dismiss 2 clears the current notification. Leaving the page injects Leave from
its lifecycle effect, ensuring a transient save confirmation does not return with
retained workflow stage. The page's layered sample-toasts demo has another model;
this helper's guarantee is one saved-preview notification, not every toast on the page.

The [existing gallery expect test](../../../test/gallery/gallery_test.ml) checks
latest enabled state, stage advancement, notification replacement, duplicate/old
dismissal and Leave preserving stage while serial identities keep increasing.
No new tests or GUI runs were performed for this guide. To add a stage, extend
Stage.t, `label`/`next` and the page's exhaustive progress mapping together. To turn
Save preview into real work, keep completion/error as typed model actions and use
an Eio-owned producer separately; do not treat Notify as proof of a completed save.
