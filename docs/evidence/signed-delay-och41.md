# Signed initial animation delays — OCH-41

Local macOS arm64 follow-up based on `ac483113ba9975fe199ccbd7f93fdbe0d1fdd85d`,
2026-10-05. This closes the signed-initial-delay API gap identified in the
[motion review](../catalog/motion-review.md); it does not close the remaining
iteration/direction or broader catalog/release requirements.

`Config.create` and `Program.create` accept initial delays within ±one day,
rounded away from zero to milliseconds. Negative delays advance active elapsed
time rather than subtracting from the clock origin. Stage delays stay nonnegative;
shared clocks retain their zero-initial-delay rule. `Program.with_initial_delay`
preserves playback and restart identity while revalidating admission limits.
Existing signed int64 wire fields carry the value without a new tag or layout.
Old decoders that reject negative values are not compatible receivers for this
addition; OCaml and native packages must continue to be upgraded together.

The [baseline](../design/animations.md) and [program](../design/animation-programs.md)
contracts specify first placement, retarget jumps, repeat boundaries, suspension,
restart, reduced motion and paint-confirmed completion. Advancing across a finite
stage produces the ordinary `Played` observation after paint; it does not mean
each skipped stage had a separate rendered frame.

## Deterministic validation

- Public expect tests cover signed bounds, fractional rounding, unchanged stage
  and shared-clock restrictions, independent baseline/program byte fixtures and
  preservation of playback/restart identity when editing delay.
- Native tests start at clock origin zero, cross multiple stage/cycle boundaries,
  finish an entire program on its first paint, retain delayed-stage deadlines,
  exclude hidden/paused time, reapply the offset on restart and retain the reduced
  motion policy. Spring samples and velocity match the same analytic trajectory
  sampled at the advanced time. Legacy single-stage motion is exercised too.
- `cargo test -p gpuio-protocol -p gpuio-native` passes 1,341 tests, with two
  existing skips. Strict Clippy for both packages/all targets passes. Commands
  use `GPUIO_JOBS=2 ./scripts/gpuio exec` and the repository toolchain.
- Full OCaml `dune build -j 2 @runtest @fmt examples/gallery/main.exe` passes.
  An initial new-test compile error used the public playback comparator on the
  wire type; the corrected test uses `W.Playback.equal`. The original log is
  retained. A premature second attempt hit the Dune lock and executed no tests.

## Installed consumer

The public Motion page now offers 0, −350 and −1,500 ms resize initial delays and
a sequence skip control. Replay restores zero delay without resetting the
monotonic restart token. The walkthrough checks intermediate and final geometry
in both directions, beyond-end settlement, ordered skipped-stage observations,
and the existing complete Motion interaction sequence. Exact timing is checked
with deterministic clocks rather than inferred from desktop polling intervals.

The fresh installed consumer passes the full local macOS walkthrough (session
33623, exit 0), including both new signed-delay checks, existing easing curves,
interruption, pause/resume/cancel/reverse, reduced motion, shared phase across
members, a second window, page retirement/remount and shutdown. Its binary SHA-256
is `495723c69030e27d9957ec3f4630fbf6248204939e1ec8d14aade1e0bf29ec42`.
The owned app process was confirmed absent afterward. No screenshots, VoiceOver,
physical presentation latency, Linux GUI or whole-release acceptance is claimed.

[Commands, logs, source snapshot, independent fixture and metadata](signed-delay-och41/validation.tar.gz)
are retained with a [verified manifest](signed-delay-och41/manifest.json).

## Hosted checkpoint distinction

Run [37369539408](https://github.com/dakotamurphyucf/gpuio/actions/runs/37369539408)
at the older `4529746` checkpoint failed Linux's progress decoder negative test:
it still used easing tag 6 as an invalid sentinel after that tag became valid.
The local `ac48311` commit already corrects that test, and its spinner counterpart,
to tag 255. This full local suite includes both corrections. The macOS job was
cancelled before acquiring a hosted runner; its annotations report ARM runner
capacity constraints. The fresh extracted-app receiver was skipped. Corrected
hosted execution remains required; no check is waived.
