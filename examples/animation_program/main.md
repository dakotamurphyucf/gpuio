# Ordered native stages, playback and a shared clock

[main.ml](main.ml) declares a tween-then-spring reveal plus repeating bars sharing
one activity phase. Build/run commands are in the [README](README.md), dependencies
and Jane Street/Bonsai PPX in [dune](dune). No external assets or services are
needed; use the isolated [toolchain](../../docs/development.md) and current
[platform policy](../../docs/platform-release-policy.md).

Read `target`/`stage`/`tween`/`spring`, immutable `sequence` and `shared`, then
`component` and startup. All constructors validate finite geometry/timing.
Sequence starts width 24, tweens to 140 over 300 ms, then springs to 280 with stiffness
140 / damping 18 / mass 1. Shared starts 40, alternates to 160 over 700 ms using named
`Animation.Clock.group "activity"`. Group clocks are app-scoped and require
infinite timed repetition with explicit initial values; new members join current
phase. See [Animation](../../lib/core/animation.mli) for stage/clock constraints.

`component` registers lifecycle activation to set ready, and `let%arr` reads
reactive program/members from `B.Expert.Var`. This derives a view; `and` lists
dependencies, not threads. `change f` returns a deferred thunk setting program
from the observed value. Pause/Resume update playback preserving native run;
Cancel is terminal until a new program/restart and holds last painted value.
Restart increments a token and must retain the returned program for later
restarts. Reverse creates a newly reversed declared program, not undoing prior
events. The stable key sequence preserves wrapper ownership while commands change.

`UI.animate_program` submits the validated program to native rendering. Native
stages/springs evaluate without OCaml frame callbacks. `card_style` gives height,
color/radius/Shrink0 while program owns width. A second column uses stable keys
0 / 1 for shared members, with `List.init members` bounded by the toggle to one or
two. Removing second member retires its native wrapper; shared clock/state in
other member continues. `on_event` stores ordered native observation batches in
refs. Infinite repeats emit no per-cycle/stage events; they do not create an
OCaml frame-event backlog.

Press Pause then Resume for a trace: native click delivers an effect to OCaml,
its thunk updates program playback, Bonsai derives new configuration, GPUIO
submits it, and native timing stops/resumes the same run. Completion yields one
batch of stage outcomes and Finished, delivered asynchronously to its callback
effect. A stage boundary is paint-confirmed but does not promise a separate
physical frame for every stage. Reduced motion settles finite directed endpoints
and holds infinite initial policy endpoints without ongoing frame requests.

`App.run` owns GPUI OS thread and the OCaml Eio UI domain, mounting 680×420.
Ordinary policy is System; there is no application I/O task. --self-test forces
Full, starts a window-scoped fiber with clock/20-second timeout, waits for ready
and native frames through Eio.Promise, pauses and verifies 200 ms without new
events, resumes original program and waits for run 1 finish, adds member 2 and
checks repeat events stay empty. It sets Reduce and restarts sequence, checking
run 4 batch exactly Stage_completed(0,Reduced_motion),
Stage_completed(1,Reduced_motion), Finished. Completion force-closes the window;
success prints GPUIO_ANIMATION_PROGRAM_PUBLIC_OK. Scope cancellation suppresses
queued task completion; see [Scope](../../lib/eio/scope.mli) and [App](../../lib/eio/app.mli).
This is event/bridge validation, not actual geometry, display latency, keyboard,
VoiceOver or Linux GUI acceptance.

To add another stage, keep initial and every stage's property set identical and
respect bounded durations/encoding. Do not add springs to the shared-clock
program, whose timed-stage constraint is different. Evolve returned restart
identity rather than repeatedly restarting the original constant in application
controls. Keep native per-frame work out of Bonsai and bound diagnostic event
refs if adapting to a persistent application.
