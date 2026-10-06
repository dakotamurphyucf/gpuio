# Native targets, ordered programs and `shared` clock ownership

[motion_page.ml](motion_page.ml) and its [interface](motion_page.mli) describe
native animations from Bonsai state. Read target/timing/stage helpers, three
program fixtures, label functions, component state/lifecycle and four cards.
`B = Bonsai.Cont` owns reactive descriptors, `E` deferred setters/logs, `V` view
wrappers and `A = Animation` validated values. Native frames do not recompute
Bonsai and no OCaml timer/Eio producer drives these animations.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe -- --trace-motion
python3 scripts/test_gallery.py --section motion --trace-motion
```

Choose Motion & rhythm, change preference, reverse the resize mid-flight, run/
pause/replay sequences and join `shared` members. [README](README.md) records native
frame/pixel and multiwindow checks separately from compilation/Linux GUI scope.
No OS preference, build or native harness was changed/run for this review.

## App preference and reactive program descriptions

Application creates one `Animation.Preference` Var for all windows and passes it
here. `set_preference` changes that source and calls `App.set_motion`; it is an
application override, not changing the OS reduced-motion setting. System follows
native policy, Reduce/Full select explicit modes. `let%arr` observes that Var,
palette and local models; buttons supply deferred effects. Style constructors
validate checked-in logical dimensions/colors, not dynamic renderer callbacks.

Local state holds expanded target, endpoint notice, resize easing/delay, `sequence`
program/notice, repeating/second-member flags and `counted` program/status. The
`sequence`/`counted` start Paused. Departure pauses both current programs, stops `shared`
members and clears `sequence` observation text; native wrappers unmount and their
callbacks retire. Other branch preferences/targets remain independent. No separate
scope handle is acquired by this page; mount/window/application clocks own native
work. [View](../../lib/core/view.mli) explains callback/hidden/disposal semantics.

## Simple resize targets and negative initial delays

`target` builds Width/Radius together; `tween` uses validated ease-in-out durations;
`stage` pairs timing with those properties. `V.animate` has stable resize-preview
key and height 48. Its Config targets width/radius 96/24 collapsed or 310/12 expanded,
1000 ms duration and selected easing/initial delay. Native target fields own those
numeric styles; changing direction retargets from painted values. Callback sets
Finished or Interrupted, including cancellation of an earlier replaced config.
It records latest outcome text, not a full event history or measured pixels.

Delay choices 0, -350 and -1500 ms advance initial timeline rather than sleeping
backward. Easing choices include CSS/cubic curves, a validated linear hold/jump
curve with repeated input boundary and four-step jump positions. Native frames
apply timing; source values alone do not certify an intermediate frame occurred.

## Sequence, restart and direction are explicit operations

`sequence` starts width/radius 64/24, tweens 550 ms to 180/8, springs to 310/16
(stiffness 150, damping 16, mass 1, max duration 2 seconds), then tweens 550 ms to 120/24.
`V.animate_program` has stable `sequence`-preview key. Replay removes initial offset
and increments restart token, resetting playback Running. Skip uses negative one-day
initial delay and restart; skipped boundaries can report Played on first accepted
paint without intermediate frames. Pause/Resume only change playback; Cancel holds
last painted value terminally until restart/new program. Reverse uses `Program.reverse`
to reverse declared intervals while retaining their timing, creating a new program;
it does not invert already-delivered events.

The callback optionally logs typed `Program.Event` with --trace-motion and stores
Run_id plus its ordered batch of observations. observation_label distinguishes
Played/Reduced_motion stage completion, Finished and Cancelled. Latest closure and
run identity handle asynchronous reconciliation; displayed notice is diagnostic
output, not a scheduler. See [Animation](../../lib/core/animation.mli) for encoding,
stage/initial-value/`shared`-clock bounds and restart exhaustion.

## Counted policies and `shared` members

`counted` starts 64/24 and uses a 500 ms ease-in-cubic stage to 260/12. Buttons select
three Normal/Reverse, two Alternate, three Alternate_reverse, zero cycles or Infinite
Reverse, then restart. Finite/Infinite direction transforms timeline progress before
easing; it differs from legacy Repeat.Alternate reversing declared intervals.
Finite policies emit one Finished after paint, not per-cycle/stage events; Infinite
never finishes. The `counted` callback updates status only on Finished. Pause/Resume/
Cancel retain the returned program/restart identity for subsequent changes.

`shared` starts 56/12, alternates a 1200 ms timed stage to 260/24 on group
`gallery-activity`. Starting conditionally mounts one or two independently keyed
wrappers; a later member joins group phase instead of starting its own timer.
Removing a member disposes that wrapper, and stopping/departure removes all local
members. Clock is application-wide, so matching mounted members in another window
can still exist. Reduced motion holds infinite members at the applicable start
and settles finite runs according to endpoint policy; it is handled natively.

Trace: Start `shared` motion → view mounts member 0 on application group clock →
Join second member mounts member 1 → both paint in common phase → Stop removes
local wrappers without cancelling unrelated application services. A finite Replay
instead changes restart token and yields ordered observations for that run.

For adaptation, keep stable keys, retain returned restart values and avoid setting
styles that conflict with animation-owned targets. Put actual model/service work
outside native animation callbacks and qualify callback acceptance versus measured
frames. Infinite animation must have a clear mount/stop owner; source review and
logging alone do not prove timing, reduced-motion or physical pixel acceptance.
