# Spring, sequence and shared-repeat motion (OCH-25)

Status: implementation design. The OCH-12 duration-based API remains supported.
Spring parameters, the typed program representation, bounded codec and compiled
finite timeline are implemented. Retained playback/paint ownership, group clocks
and view/event transport remain in progress; the expanded rendering pipeline is
not yet available.
Acceptance remains the complete live OCH-25 ticket, followed by integrated chat
showcase OCH-46. Linux GUI follows the existing OCH-17 platform policy.

## Public shape and native ownership

Extend the existing `Animation` vocabulary with abstract `Spring`, `Timing` and
`Stage` modules and configuration constructors for springs and sequences. Reuse
`Target` and the existing eleven numeric properties. A stage contains its target,
delay and either duration/easing or physical spring timing. All stages name the
same property set. At most 32 stages are accepted. The sum of stage maximum
durations and per-stage delays is at most one day. A separate initial delay
is applied only before the first cycle and is independently bounded to one day,
preserving the established API’s full delay/duration range. Public constructors validate
before encoding; the native decoder independently bounds and validates input.

The existing `Config.create` continues to express a single duration-based stage.
A spring constructor expresses a single spring stage, and a sequence constructor
expresses ordered stages. Sequences start at explicit initial values; a standalone
spring may omit initial values and first mounts at its target, matching OCH-12.
The mounted wrapper owns native state. OCaml publishes configurations, not frames.
Keep existing baseline wire operation/fixture semantics; introduce a distinct
advanced-program operation and normalize legacy configuration at the native
boundary rather than silently changing the established record encoding.

## Spring parameters and continuity

`Spring.create ~stiffness ~damping ~mass ?epsilon ?max_duration ()` constructs
validated physical parameters. Stiffness is in [0.01, 10,000], mass in
[0.01, 1,000], damping in [0, 1,000], and positional epsilon in [0.0001, 1].
All numbers are finite. Default epsilon is 0.001 animated units. The maximum
active duration defaults to 10 seconds, is positive, rounds up to milliseconds,
and is capped at 60 seconds. Zero damping is deliberately permitted; the maximum
duration then supplies a finite termination contract.

Use the pinned GPUI `SpringConfig` analytic solver rather than frame-step Euler
integration. Evaluate relative displacement from the target, which avoids losing
small displacements when an absolute logical coordinate is large. Reuse the
same elapsed-time trajectory for preview and paint. Completion time is the
conservative positional/velocity settling envelope, capped by maximum duration;
reaching the cap snaps exactly to the target with zero velocity. Velocity tolerance
is epsilon times the natural frequency, following GPUI's physical convention.

Retarget from the last painted position and velocity of each surviving property.
Discarded previews cannot change continuity. New properties start with zero
velocity; duration-based stages do not claim spring velocity continuity. At a
spring-to-spring stage boundary the preceding stage has settled, so velocity is
zero. Numeric output remains clamped to each property's existing valid domain.
An outward velocity at a clamped boundary is zero for subsequent retargeting.
Overshoot is otherwise preserved. Paused/hidden time is excluded from active time.

The maximum duration also bounds pathological configurations and numerical
precision limits. The GPUI solver operates in f32; bridge data and public target
values remain f64, with exact declared endpoints restored when settling finishes.
These are visual numeric animations, not a scientific simulation API.

## Sequences, control and events

Stage transitions run natively. A slow frame may pass multiple duration-based
stages; process at most the admitted 32 stages, preserving order and elapsed time.
No OCaml round trip gates the next stage. Finite runs emit typed stage completion
observations plus a terminal outcome, only after the corresponding sample is
painted. There are at most 32 stage observations and one terminal outcome per run.
Generation and an ordered observation index reject stale/duplicate deliveries.
Repeating sequences do not emit unbounded per-cycle/stage notifications.

Pause/resume is a separate declarative playback setting, not a new program/run.
Pausing holds the last painted value, suppresses wakes and preserves velocity;
resuming continues active elapsed time. Cancellation holds the last painted value,
clears velocity/timers and emits one terminal cancellation. An explicit new run
restarts from declared initial values; ordinary retargeting preserves the visible
position and spring velocity. Reversing a standalone spring means retargeting it
back to the previous target. Reversing a sequence is an explicit reversed program,
not an implicit reversal of already-delivered stage history. Define these control
operations in the public interface before wiring their transport.

Hidden content pauses independent runs. Reduced motion settles finite programs at
the final stage target without playing intermediate stages; skipped stages must be
distinguishable from normally completed stages in the typed observation contract.
Repeated programs display their initial values without frame requests. Removed
nodes and closed windows release native state, deadline tasks and observations.

## Shared repetition

Expose application-clock repetition and named shared groups. Fixed-duration stages
are required for shared repetition: physical settling duration depends on each
member's range and cannot define a common cycle. Initial values are mandatory.
Members may use different numeric ranges, but a group's stage durations, delays,
repeat direction and period must agree. Reject conflicting group declarations
atomically. Names are application-scoped, bounded UTF-8 values, not Rust pointers.

Use one native monotonic application clock and retained group phase state. A late
member joins the current phase. A hidden or individually paused member requests no
frames and rejoins the group phase on resume; it does not shift other members.
A group pause freezes the shared phase; resuming advances all members together.
Reduced-motion intervals freeze shared phase. Last-member removal drops named
group state; a later new group begins a new lifetime. The application clock itself
has no timer and cannot keep an empty application redrawing.

Bound the registry to 128 named groups and 1,024 live group memberships per
application. Admission and removal must handle atomic tree rollback, cross-window
membership and close/shutdown. No weak-entry tombstone growth is allowed. Test
late joining, individual hide/resume, whole-group pause, mismatched declarations,
registry saturation and repeated window disposal with deterministic clocks.

## Source and acceptance

Pinned upstream commit `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` exposes
`with_spring`, `with_animations` and `repeat_synced` in
`crates/gpui/src/elements/animation.rs`; its analytic step/settle-time solver is in
`crates/gpui/src/spring.rs`. Those sources were inspected locally. GPUIO retains
its own validated state/paint/event ownership to preserve its FFI contract.

Keep baseline OCH-12 deterministic and native tests passing. Add independent
OCaml/Rust codec evidence, deterministic tests for every policy above and actual
native/public spring/sequence/shared-repeat demonstrations. Measure disposal and
whole-window idle behavior, not merely endpoint values. Complete consolidated
macOS/Linux build/unit gates before marking the ticket complete. Arbitrary
keyframe timelines, shared-layout and exit-presence orchestration remain excluded;
color, arbitrary object and percentage/auto interpolation are not introduced.

## Implemented program representation

`Animation.Timing.tween` and `.spring` describe validated stage timing;
`Animation.Stage.create` pairs timing, target and per-stage delay.
`Animation.Program.create` accepts initial values, stages, repetition, initial
delay and `Animation.Clock` selection. `with_playback` changes playback without
changing program data. `restart` increments its token and selects Running;
`reverse` reverses declared intervals and requires initial values. These pure
constructors do not yet provide a renderable advanced view.

The matching `Animation_program_wire`/Rust `animation_program` schema has distinct
Program and Config records. Config adds the admission generation, playback and
restart token. `same_run` ignores admission generation and playback, so a future
retained owner can distinguish pause/resume from a program replacement. The run ID
must remain its start generation while playback-only configurations advance the
admission generation; this is required when wiring callback fences.

The standalone native decoder caps configuration bytes at 16,384, stages at 32,
properties at 11 and group names at 128 UTF-8 bytes before allocation. It validates
property sets, positive shared repeat periods and timed-only shared stages. Heap
accounting includes vector/string capacities. No new transaction operation or
runtime capability has been introduced at this checkpoint. `Program::from_legacy`
provides a validated common representation without changing the old wire bytes.

A finite stage observation carries a zero-based stage index and Played or
Reduced_motion result. Its delivery index is stage index + 1; terminal signals use
33. Signals carry the run generation. The retained bridge still needs to implement
monotone (run generation, delivery index) dispatch and paint-confirmed delivery;
these schema limits alone do not implement delivery.

`motion_timeline::Timeline` compiles numeric stages and spring trajectories once.
It samples active elapsed time without mutating state or emitting callbacks. A
sample includes values, velocity, the completed stage prefix and either Idle,
Frame or a relative Wait deadline. It visits at most 32 admitted stages even when
a slow frame passes multiple boundaries. Constant timed intervals wait for their
boundary without frame polling. Timed stages clear inherited spring velocity;
new properties use declared initial values. It accounts for retained segment and
spring-array storage. The owner still needs to enforce aggregate admission,
paint epochs, cancellation, hidden/reduced policy and repeating clocks.
