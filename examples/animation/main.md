# Declaring endpoints while Rust renders the frames

[main.ml](main.ml) reveals/clips a fixed-width sidebar. The [README](README.md)
gives exact build/run commands; [dune](dune) links Core/GPUIO/Bonsai/Eio with Jane
Street/Bonsai PPX. Use the isolated [toolchain](../../docs/development.md).
Native launch requires a desktop; no assets or external work are needed. The
[platform policy](../../docs/platform-release-policy.md) separates Linux build
coverage from deferred desktop acceptance.

Read `target`, `component`, then the launcher. `target width opacity radius`
validates an `Animation.Target.t` with Width, Opacity and Radius. Properties must
be unique, finite and within their contract ranges; dimensions are logical pixels.
`Or_error.ok_exn` treats invalid fixed demo config as a programming error.
`component` registers a Bonsai activation effect setting diagnostic `ready`,
then `let%arr` reads external reactive `phase_var`. Bonsai graphs persist;
`let%arr` derives a reactive view from current ordinary values and does not start
motion itself or perform I/O. `B.Expert.Var` is application-controlled reactive
state; the Toggle button returns `E.of_thunk` that sets phase to phase+1 when run.

Even phase targets width 240, odd phase width 72; opacity/radius target 1/12.
`Animation.Config.create` supplies initial 0/0/0, normal duration 220 ms and
ease_in_out easing. The stable `UI.animate ~key:"sidebar"` wrapper preserves
native animation identity on retarget. Its inner text stays width 240 with
Shrink 0; overflow clipping belongs to the outer panel, so the text does not
reflow at each revealed width. Theme token colors resolve through window theme.
`UI.column`/`row` and validated padding/gaps lay out the button and panel.

Click Toggle during motion: native button action crosses asynchronously to the
OCaml UI domain; the thunk updates phase, Bonsai derives a new target, and GPUIO
submits it. Rust starts from last painted values, renders intermediate frames
and sends Finished or Cancelled Replaced for runs. `on_event` returns an effect
recording each event in the diagnostic ref; no callback runs per frame. Run IDs
are scoped to that retained wrapper, not global IDs. See the
[animation contract](../../lib/core/animation.mli). Native accepted/paint-confirmed
endpoints differ from physical screen presentation.

`App.run` owns GPUI on the OS main thread and the OCaml Eio UI domain for graphs
and effects, opening a 640×260 window. Ordinary motion policy is System and no
application producer is started. --self-test forces Full initially, starts a
window-scoped Eio fiber with explicit clock and 15-second timeout, and polls
diagnostic refs every 5 ms. It waits for graph activation and a native render
callback bridged through Eio.Promise, retargets phase 1, then requires run 1 either
finished or cancelled and run 2 finished. It changes theme, sets Reduce and phase 2;
that phase uses a 60-second duration so immediate reduced-motion settlement is
observable. It requires run 3 finished and exactly three events, then force-closes
through App.Window.close. Success prints GPUIO_ANIMATION_PUBLIC_OK.

This diagnostic checks native events and runtime policy, not pixel trajectories,
OS input, VoiceOver or Linux desktop acceptance. Scope cancellation suppresses
queued completions and cleans window resources; read [Scope](../../lib/eio/scope.mli)
and [App](../../lib/eio/app.mli). The event ref records terminal events in ordinary
launch too, so this finite example is not a bounded telemetry system.

To animate height instead, update initial/target with the same property set and
review inner layout/clipping. Keep wrapper key stable when retargeting; a new key
means native removal/new lifetime. Do not interpolate in a Bonsai timer or use
terminal event as evidence of physical display. For repeated interactive actions
from multiple producers, use a latest-model reducer rather than captured phase
values, and bound any retained event history for long-running applications.
