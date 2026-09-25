# Spring, sequence and shared-repeat motion (OCH-25)

Status: locally implemented and validated on macOS; consolidated hosted gates
and merge remain pending. The OCH-12 duration-based API remains supported.
Spring parameters, typed programs, bounded decoding, compiled timelines, atomic
session admission, retained clocks and the public `View.animate_program` API are
implemented. Actual mounted/public checks cover geometry, controls, retained-list
and panel visibility, deferred overlays, cross-window clocks and bounded workloads.
Capability `4294967296` advertises advanced programs; the current aggregate is `17179869183`.
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
advanced-program operation. The established duration-based path retains its
existing native owner and record encoding.

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
not an implicit reversal of already-delivered stage history. Public controls are
`Program.with_playback`, `restart` and `reverse`.

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
The internal clock registry also supports group/application pause, which freezes
shared phase. The public declarative controls pause individual programs; an
application-wide or named-group imperative control API is not exposed.
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
`reverse` reverses declared intervals and requires initial values. Mount these values through `View.animate_program` (or its Bonsai counterpart).

The matching `Animation_program_wire`/Rust `animation_program` schema has distinct
Program and Config records. Config adds the admission generation, playback and
restart token. `same_run` ignores admission generation and playback, so the
retained owner distinguishes pause/resume from a program replacement. The run ID
remains its start generation while playback-only configurations advance the
admission generation; callback fences preserve that distinction.

The standalone native decoder caps configuration bytes at 16,384, stages at 32,
properties at 11 and group names at 128 UTF-8 bytes before allocation. It validates
property sets, positive shared repeat periods and timed-only shared stages. Heap
accounting includes vector/string capacities. Operation 37 mounts the configuration
under the advanced-program capability. `Program::from_legacy`
provides a validated common representation without changing the old wire bytes.

A finite stage observation carries a zero-based stage index and Played or
Reduced_motion result. Its delivery index is stage index + 1; terminal signals use
33. Signals carry the run generation. The retained bridge implements
monotone (run generation, delivery index) dispatch and paint-confirmed delivery.

`motion_timeline::Timeline` compiles numeric stages and spring trajectories once.
It samples active elapsed time without mutating state or emitting callbacks. A
sample includes values, velocity, the completed stage prefix and either Idle,
Frame or a relative Wait deadline. It visits at most 32 admitted stages even when
a slow frame passes multiple boundaries. Constant timed intervals wait for their
boundary without frame polling. Timed stages clear inherited spring velocity;
new properties use declared initial values. It accounts for retained segment and
spring-array storage. Session admission and the retained owner enforce aggregate
quotas, paint epochs, cancellation, hidden/reduced policy and repeating clocks.

## Retained owner and clock primitives

`motion_program::State` now owns the admitted configuration, run generation, last
painted frame, completed-stage watermark, monotone time, pause interval and an
opaque paint epoch. Preview samples cannot change position, velocity or delivery.
Paint rejects replaced epochs, older timestamps and invalid shared-clock samples.
Only an accepted paint commits the completed prefix and terminal observation.
Wake validity deliberately ignores paint timestamps: a newer unrelated paint must
not cancel a still-current deadline. It still checks the run/epoch, visibility,
playback and shared-clock validity. The adapter must also match its current
deadline and rearm it if the captured wake token was invalidated.
Played means that normal active time passed a stage, confirmed by this paint;
it does not promise a separate visible frame for every stage on a slow display.

Playback-only snapshots advance configuration admission without changing the run
ID. Overlapping hidden, explicit pause and reduced-motion intervals are excluded
from independent active time. Explicit pause holds the painted frame even while
reduced motion is selected; resume then applies the current motion policy. Reduced
finite runs mark only their remaining stages Reduced_motion and finish at the
final target. Finished runs do not replay when full motion returns.

Cancellation freezes the painted position, clears velocity and emits one Requested
terminal signal. Running alone does not revive a cancelled run. A program first
mounted as Cancelled is dormant and emits no fabricated cancellation. Increasing
the restart token starts from declared initial values. Decreasing the token for
an unchanged program is rejected; a newly constructed, different program may use
its default token and retarget from the painted frame. This preserves ordinary
target changes after an earlier explicit restart.

Repeats use precompiled forward/reverse intervals and integer duration modulo.
A reversed interval retains its own duration and stage delay. The global initial
delay occurs once; per-stage delays occur each cycle. Repeats emit no stage/cycle
observations. Constant cycles and physical cycles whose settling time collapses
to zero stay idle rather than polling frames or dividing by zero.

`motion_clock::Registry` atomically replaces a logical window's shared membership.
It checks the complete cross-window schedules and the 128-group/1,024-member
limits before changing clocks or membership. Group IDs include a positive i64
lifetime generation; exhaustion rejects admission instead of wrapping. Existing
groups preserve phase, late members join that phase, and removing the final member
releases the named clock. A later same-name group gets a new identity; old controls
cannot pause it. Application-clock phase persists for the application lifetime.

Shared clocks contain no timer. Group/application pause and reduced motion freeze
elapsed time. Source snapshots include a validity token, invalidated by policy
changes or clock disposal, so an already prepared frame cannot commit an obsolete
phase. A named-group program rejects an application-clock snapshot. Names share
immutable storage; sampling does not allocate a new name string. Individual hidden
or paused members hold their painted frame and rejoin shared phase when resumed.

## Mounted adapter and public event batches

`View.animate_program ?key ?style ?on_event program children` mounts a distinct
`Animation_program` node. This preserves the established `View.animate` endpoint
contract and existing wire tags. Targets own their corresponding style fields.
The new wire operation is appended as tag 37, node kind 32 and event 41; independent
OCaml/Rust fixtures pin both the full transaction and an event envelope.

`Animation.Program.Event.t` contains a run ID and a nonempty ordered list of
`Stage_completed (zero_based_index, Played | Reduced_motion)`, `Finished`, or
`Cancelled reason` observations. Native painting emits at most 33 observations
as one mailbox event. The decoder bounds the list before allocating its entries;
mailbox accounting includes its payload. Reconciliation validates the whole batch,
checks window/node/handler/revision/generation, removes already-delivered prefixes
using a run/index watermark, and invokes the latest accepted closure once. Repeats
emit no cycle callbacks. Unmount/window disposal discards callbacks.

Session admission owns an independently borrowed `motion_host::Store`, sharing
one application monotonic origin with all windows. Application/system motion
policy updates reach this store before paint, including when all widgets are
hidden; a weak binding does not retain the application. Tree validation, list-pin
checks and capacity reservation happen before a final admission callback. Only
then may clocks/membership and the tree commit. A rejected tree, group conflict,
stale generation, duplicate program update or quota failure preserves both old
states. Style-only updates do not scan all retained tree nodes for motion.

The application admits at most 1,024 advanced owners and 128 MiB of conservative
compiled-storage reservations, in addition to 128 groups/1,024 shared memberships
and existing tree/session quotas. Reservations cover the maximum first/forward/
reverse compiled tracks, configuration copies and adapter bookkeeping. This is an
admission unit, not RSS. A replacement may additionally compile one bounded owner
before releasing its previous tracks. Removal, window close and shutdown release
reservations and shared membership.

The GPUI owner holds at most one cancellable deadline. Timer callbacks validate
the captured lifetime/policy sample and owned deadline; an unrelated newer paint
does not cancel a still-valid delayed start. Samples and timer closures do not own
obsolete timelines or keep removed widgets alive. Only a live paint requests a
subsequent animation frame. Pause, reduced motion, hidden content and disposal
clear pending work according to the retained owner contract.


## Visibility and acceptance boundaries

Visibility follows actual paint participation, not retained-tree membership or
speculative virtual-list measurement. The root paint resets owner presence; each
painted owner marks itself present even when a completed/cancelled sample no longer
commits new state. A sweep deferred until the effect cycle ends runs after GPUI's
deferred popup paint. Owners not painted pause independent time and cancel their
pending deadlines. Returning rows resume their remaining active delay; shared
members rejoin their current phase. The sweep itself never requests a frame.

This policy covers display/visibility-hidden content and nonpainted virtual rows.
It does not promise pixel-level occlusion detection for partly clipped/overscan
content or animations intentionally entering from outside their bounds. Completion
is distinct from visibility: a finished visible owner can restart immediately.

Native macOS checks additionally paint 1,024 concurrent owners, exercise an input
control, retarget the owners, then dispose them and verify zero reservations and
idle rendering. The evidence reports debug wall times separately from frame CPU,
end-to-end OS input latency and process RSS, which that test does not measure.
