# Native declarative animations (OCH-12)

Status: typed configuration and deterministic Rust timing core implemented locally.
View/reconciler/transport/GPUI integration and platform policy detection are still
in progress. The animation capability is not advertised yet. This document does
not claim that an application can already render these transitions.

## Configuration

`Animation.Target.create` validates a nonempty property set: width, height, top,
right, bottom, left, opacity and four corner radii. `Radius` is an OCaml shorthand
for all four corners. Duplicate properties after expansion are errors. Geometry
uses logical pixels, with the same finite bounds as the style API; opacity is in
[0,1]. Numeric targets deliberately exclude `auto` and percentage interpolation.

`Animation.Config.create` accepts a target, optional initial values, duration,
delay, easing and repetition. Initial and target property sets must match.
Duration defaults to 200 ms, delay to zero, easing to linear and repetition to once.
Core `Time_ns.Span` arguments are bounded to one day and rounded up to milliseconds.
The public type is abstract and does not expose the bridge's generation counter.

```ocaml
let target =
  Animation.Target.create [ Width, 240.; Opacity, 1. ] |> Or_error.ok_exn
in
let initial =
  Animation.Target.create [ Width, 0.; Opacity, 0. ] |> Or_error.ok_exn
in
Animation.Config.create
  ~initial
  ~target
  ~duration:(Time_ns.Span.of_ms 180.)
  ~easing:Animation.Easing.ease_out
  ()
```

Easing includes linear, ease, ease-in, ease-out, ease-in-out and validated cubic
Bezier control points. Bezier evaluation inverts the X curve, including endpoint
zero derivatives. X controls are in [0,1]; Y controls may be any finite value. Overshoot is
allowed; resulting properties clamp to their valid domain, such as nonnegative
width and opacity no greater than one. Timing is evaluated once per sample,
then applied to at most eleven properties.

## Timing and ownership

`rust/native/src/motion.rs` owns a run's configuration, origin, last painted values,
pause state and terminal status. It has no scheduler, GPUI entity or OCaml callback.
The host will pass a monotonic clock and interpret the returned scheduling request:

- `Idle`: no animation wake requested.
- `At deadline`: a delayed start needs one deadline wake, without frame polling.
- `Frame`: intermediate values need another native animation frame.

Layout may preview a sample. Only committing that sample at paint updates the
visible value and reports completion. Generation and opaque epoch identity reject
samples prepared before a replacement, visibility change or reduced-motion change.
Older timestamps cannot overwrite newer painted values. Epoch identity allocates
on state changes, not on every sample.

With no initial values, first placement is immediate. A changed target starts at
the last painted value, even if time has advanced further or the old run was in
delay. Newly targeted properties start at the supplied initial value or their target.
An identical snapshot does not restart. A changed snapshot requires a newer bridge
generation. Interruption cancels an unfinished old run once; a completed run does
not later emit cancellation. Zero duration respects delay unless placement is
already immediate or reduced motion applies.

Loop and alternate repetition require explicit initial values and positive duration.
After interruption, the first cycle goes from the painted value to the new target;
subsequent cycles use the declared initial/target range. Loop resets at its cycle
boundary; alternate reverses continuously. Integer duration arithmetic avoids
losing sub-frame precision after long uptime. Repeats have no per-cycle event.

Explicitly hidden runs pause elapsed time and request no animation wake. Showing
one resumes it. Reduced motion immediately settles a visible one-shot run at its
target, with completion at paint. A repeated run displays its initial values and
pauses without an endpoint or frame requests; full motion resumes its elapsed phase.
A finite run completed under reduced motion does not replay when the policy changes.
Cancellation freezes the last painted values and emits at most one endpoint.
Removing/window-closing cancellation still needs host lifecycle wiring.

## Remaining integration and acceptance

The upcoming view adapter must retain one state per animated node, apply numeric
samples to GPUI layout/paint, and make animation-owned properties take precedence
over static/state styles on that wrapper. An outer clipping wrapper will reveal a
sidebar while its inner content retains a fixed width. No frame callback crosses
into Bonsai. Optional typed endpoint delivery must validate window/node identities
and retain deliberate ordering for cancellation of a replaced generation; it must
not accidentally discard valid cancellation as a stale current-run event.

Still required before completion:

- Bounded Rust command decoding, retained-tree accounting and capability negotiation.
- Public View/Bonsai API and reconciliation, generation/handler identity, endpoint
  delivery and stale replacement-widget rejection.
- GPUI frame/deadline scheduling with disposal, explicit ancestor visibility and
  native window lifecycle handling; existing progress indicators share this policy.
- `System`/`Reduce`/`Full` application policy, platform preference detection where
  available and documented fallback behavior.
- Real native/sidebar/repeat/idle tests and public FFI integration; required macOS
  functionality and Linux build/tests under the existing platform gate.

Springs, sequences and synchronized repetition belong to OCH-25. This does not
remove any OCH-12 baseline requirement or OCH-11's shared basic-transition scope.

## Local evidence

Nine deterministic Rust motion tests pass: delay/deadlines, paint-confirmed
endpoints, interruption/reversal, immediate placement, zero duration, invalid
updates, hidden/reduced policy, repeats, long uptime, Bezier inversion/overshoot
and invalidation of previously prepared frames. Extreme finite Bezier Y controls
also produce bounded geometry. Full Rust workspace tests pass.
OCaml expect tests validate property expansion/canonical identity, geometry,
initial range, timing, easing and bridge generation. An independent binary fixture
checks the Rust and OCaml configuration encoders and full OCaml decoding.

These tests exercise the configuration and timing core. Actual GPUI rendering,
platform policy detection, endpoint transport and idle-window acceptance are not
established by them. Local logs are `motion-*.log` in the implementing agent's ignored
scratch directory; hosted validation remains deferred to the consolidated delivery.
