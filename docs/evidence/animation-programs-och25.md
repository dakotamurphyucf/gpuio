# Advanced animation implementation evidence (OCH-25)

## Spring parameter and trajectory foundation

Local macOS arm64 checkpoint, 2026-09-25. This is partial implementation, not
OCH-25 acceptance. The advanced-program transport, mounted rendering integration,
sequence scheduler, playback controls, shared groups and demonstrations remain.
The existing OCH-12 API and wire encoding are unchanged. No additional capability
is advertised from these foundation tests.

`Animation.Spring.create` validates physical parameters and finite maximum active
duration. The independent OCaml/Rust fixture `animation-spring.hex` establishes
field order and bin_prot encoding; it does not yet prove an end-to-end advanced
program transaction. Public constructors and native `Spring::is_valid` test finite
numeric bounds, NaN/infinity, permitted zero damping and duration boundaries.

`motion::spring::Trajectory` uses the pinned GPUI analytic step and conservative
settling-time solver, evaluated relative to the target. Its immutable samples
include position, velocity and completion. A caller can construct a retargeted
trajectory from a painted sample without any discarded preview changing state.
The eventual retained owner must enforce paint, generation and lifetime rules;
those are not supplied by this numerical helper alone.

Seven Rust tests pass:

- Under-, critically and over-damped springs settle with exact endpoints and zero
  terminal velocity; both displacement and velocity are checked before completion.
- Undamped motion overshoots and finishes at its explicit maximum active duration.
- Retargeting preserves the supplied sampled position/velocity and initially
  continues momentum even when the new target is behind the current motion.
- Frame partitioning gives equivalent motion within native numerical tolerance;
  small changes around large absolute coordinates retain displacement precision.
- Opacity clips to its legal domain, suppresses outward retargeting velocity and
  allows overshoot to return into range.
- Invalid parameters and states are rejected before invoking the solver.
- Admitted parameter extremes and velocity bounds remain finite and terminate.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio` and the isolated project toolchain:

```sh
./scripts/gpuio exec cargo test --locked -p gpuio-native --lib motion::spring -j 2
./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test animation -j 2
./scripts/gpuio exec dune runtest -j 2 test/view_api
./scripts/gpuio exec cargo test --locked -p gpuio-native --test motion --test animations -j 2
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
./scripts/gpuio check-fmt
```

All pass: seven spring tests, four protocol animation tests, the OCaml view API
expect suite, nine baseline motion tests and the atomic tree animation lifecycle
test. These checks open no GUI windows. No Linux or hosted execution is claimed.
The [implementation design](../design/animation-programs.md) records remaining
native scheduling, event, group and lifetime requirements.

## Typed programs, bounded decoding and compiled timelines

The next local checkpoint adds public Timing/Stage/Clock/Playback/Program types,
immutable pause/restart/reverse configuration, a matching wire schema and a bounded
standalone native decoder. This is still partial OCH-25 implementation: it does
not yet mount an advanced program, emit its observations or own shared-clock state.
No advanced runtime capability is advertised.

The independent `animation-program.hex` fixture contains a delayed tween followed
by a delayed physical spring, explicit initial values, paused playback, a restart
token and a positive admission generation. Both OCaml and Rust construct it
independently. Rust round-trips it and rejects every truncated prefix, trailing
bytes, malformed numeric/property data, excess stages/properties and overlong
configuration/group names. Shared-clock validation admits only timed positive
repeating cycles and rejects missing initial values or an initial delay.

The OCaml expect tests also prove immutable playback/restart configuration,
32-stage boundaries, cycle duration limits, UTF-8 group validation and reversing
a sequence twice restores its complete configuration. A compatibility test covers
the old API's one-day initial delay plus one-day duration: the new common
representation preserves both instead of incorrectly limiting their sum to one day.
The pre-existing duration API's binary fixture is unchanged.

Six native timeline tests cover mixed tween/spring timing, separate initial/stage
delays, a late sample crossing multiple stages, spring velocity on retarget,
initial values for newly added properties, cleared velocity for timed stages,
constant-interval deadline scheduling, 32 zero-duration stages, immediate first
placement without initial values, invalid input and retained-byte accounting.
Sampling is immutable. These tests prove numerical stage traversal; the retained
owner must still confirm paint before delivering any completed-stage prefix.

Validation commands at this checkpoint, through the isolated jobs=2 wrapper:

```sh
./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test animation --test animation_program -j 2
./scripts/gpuio exec cargo test --locked -p gpuio-native --lib motion -j 2
./scripts/gpuio exec cargo test --locked -p gpuio-native --test motion --test animations -j 2
./scripts/gpuio exec dune runtest -j 2 test/view_api
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
./scripts/gpuio check-fmt
```

These pass locally on macOS: ten protocol tests, fourteen motion-related native
unit tests, the nine baseline motion tests, atomic tree lifecycle test and OCaml
view API suite. The expect-test correction was whitespace layout only; the values
were reviewed. No GUI or Linux result is claimed from this checkpoint.

The full isolated `dune build -j 2 @all @runtest` also passes with the new exported
types and timeline code. All validation processes exited; none opened GUI windows.

## Retained owner, shared clocks and deterministic lifecycle

The local native primitives now implement paint-confirmed stage/terminal delivery,
pause/resume, hidden/reduced policy, cancellation, restart and repeat evaluation.
This checkpoint is still not GPUI rendering acceptance: the session/view/event
adapter, aggregate admission and actual native/public examples remain required.

Fourteen owner tests prove ordered bounded stage prefixes; duplicate/stale paint
rejection; unchanged run identity across playback updates; overlapping hidden/pause
intervals; reduced-motion skips; terminal cancellation/restart; painted spring
velocity on retarget; integer repeat phase after long uptime; unequal reverse
interval durations; initial versus repeated stage delays; late shared phase;
clock-policy/disposal fences; invalid-update rollback and compiled-data disposal. A delayed wake remains valid after a newer paint,
while obsolete prepared paints cannot rewind a subsequent retarget.
Constant repeats request no wake, including on their first sample. Physical cycles
with zero effective settling time remain idle. Retained samples contain frame data
and validity tokens, and do not keep obsolete compiled timelines alive.

Six registry tests cover pause/reduced overlap, logical windows joining a shared
group, cross-window schedule conflict rollback, independent application/group
pause, 128-group and 1,024-member saturation, 256 creation/removal cycles without
name tombstones, close invalidation and generation exhaustion. These use logical
window identifiers; they do not open or test actual OS windows.

One local debug run of 16 create/retarget/dispose cycles with 32 spring stages and
all 11 properties took 36.396791 ms. Maximum accounted retained data was 161,720
bytes for one owner after retarget (about 158 KiB). Each owner was dropped before
the next cycle, with a weak reference proving compiled data was released. This is
configuration compilation/lifecycle timing, not frame latency or process RSS.
Aggregate memory admission must still be connected before rendering is enabled.

Final local commands use the isolated jobs=2 wrapper:

```sh
./scripts/gpuio exec cargo test --locked -p gpuio-native --lib motion -j 2 -- --nocapture
./scripts/gpuio exec cargo test --locked -p gpuio-native --test motion --test animations -j 2
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
./scripts/gpuio check-fmt
./scripts/gpuio exec dune build -j 2 @all @runtest
```

These pass: 34 motion-related unit tests, nine baseline motion tests, the atomic
animation tree lifecycle test, Clippy, formatting and full Dune build/expect tests.
An overly strict floating-point equality in a new reverse-interval test was changed
to a 1e-9 logical-unit tolerance after inspecting its 3e-14 rounding difference.
A test-only Clippy suggestion was also fixed. No production behavior was relaxed.
All processes exited; no GUI windows opened. Hosted/Linux and full ticket acceptance
remain pending.


## Atomic admission, mounted GPUI adapter and public Bonsai API

The implementation now connects `View.animate_program` and
`Gpuio_bonsai.View.animate_program` through distinct wire kind 32, operation 37
and batched event 41 to a retained GPUI owner. Existing duration-based animation
wire bytes and native behavior remain covered by their regression tests.

The public `Animation.Program.Event` delivers a nonempty ordered observation batch
(up to 33 items). The decoder bounds its count before allocating entries, validates
order and run identity, and reconciliation removes duplicate prefixes while
preserving the latest accepted callback. Playback updates preserve the native run
ID. Independent transaction and event fixtures pass OCaml/Rust checks, including
truncation and invalid data. Mailbox byte accounting includes the batch payload.

Three session admission tests prove cross-window group conflicts, invalid tree
rollback, stale/duplicate program updates, bounded events, 1,024-owner admission,
128-MiB compiled-storage rejection, and release on close/shutdown. Admission runs
after tree/list-pin validation and allocation reservation. Shared clocks use a
separately borrowed application store, so rendering never mutably borrows Session.
The application motion policy updates clocks before rendering, including when no
widget is visible. Constant-policy renders take a fast path without walking groups.

The mounted `native_animation_program` check passes locally on macOS. It uses a
real GPUI/AppKit window and checks actual tween/spring geometry, ordered sequence
stages, pause/resume with preserved run identity, late shared-phase joining,
reduced-motion placement, settled/reduced idle render counts, autonomous real-time
repetition, a real delayed wake, unmount and window-close cleanup. Weak-owner and
zero-reservation assertions verify release; repeats produce no cycle events.
The baseline `native_animation` executable also passes on the same local build.

The first mounted run caught missing visited-node bookkeeping: a program was
being hidden before its sample could commit. Marking rendered program nodes as
visited fixed it. Idle assertions allow an already-requested platform frame to
drain for 80 ms, then require an unchanged render count for the measured interval;
this does not permit ongoing idle polling. Two raw-bridge examples were updated
for the new exhaustive event case. A new expect fixture required whitespace-only
layout correction; expected values were reviewed.

`examples/animation_program/` is a runnable public Bonsai/Core/Eio example with
pause, resume, restart, reverse, cancel and shared activity members. Its local
`--self-test` passes actual FFI/event delivery, playback run preservation, reduced
stage batches, absence of repeat callbacks and scoped window shutdown. It reports
`GPUIO_ANIMATION_PROGRAM_PUBLIC_OK`. Native checks report
`GPUIO_ANIMATION_PROGRAM_OK` and `GPUIO_ANIMATION_PROGRAM_TIMER_CLOSE_OK`.
The macOS CI workflow and informational X11/Wayland scripts now include both the
native and public program checks; hosted results are still pending.

Validation commands (all through the isolated jobs=2 toolchain):

```sh
./scripts/gpuio exec cargo test --workspace --locked -j 2
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests,native-image-tests,native-canvas-tests -j 2 -- -D warnings
./scripts/gpuio exec dune build -j 2 @all @runtest
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_animation_program --test native_animation -j 2
_build/default/examples/animation_program/main.exe --self-test
./scripts/gpuio check-fmt
```

The workspace Rust suite and combined-feature Clippy pass. Full Dune build/expect
tests passed after updating the exhaustive example matches; the final check also
covers the new public demo and bounded-reader assertion. Formatting and final
source validation are recorded with the checkpoint. All local GUI checks terminate
and close their windows. No Linux GUI or hosted acceptance is claimed.

OCH-25 remains in progress: expand actual hidden/list/tab and two-window group
lifecycle coverage, validate the remaining control/retarget/observer cases through
the mounted public path, review runtime bounds under realistic simultaneous
workloads, and advertise the completed capability only after acceptance. The
polished chat integration remains OCH-46. Full milestone hosted gates and merge
remain pending.
