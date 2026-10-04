# Shared scrollbar timing and scheduling — OCH-41

Local checkpoint, 2026-10-03, macOS checkout, base `83eb87e` plus the milestone
working tree. Builds on the [value/geometry foundation](scrollbar-foundation-och41.md)
and [accepted implementation contract](../design/scrollbar-presentation.md).
**The native scrollbar element, handle/input/accessibility integration, View
bridge and gallery controls remain unfinished.** The scheduler tests paint a
small fixture canvas, not a functional scrollbar. OCH-41 remains In Progress.

## Implemented

`scrollbar_lifecycle` owns bounded per-axis timing with caller-supplied monotonic
time. Immutable samples do not start transitions until accepted paint commits
them. Fade/slide and width interruption starts from the last committed visual;
remaining opacity/slide distance scales visibility duration. Repeated paint does
not restart a matching target. Reduced motion snaps visual channels; Always
skips visibility motion but retains finite width changes. Idle hold has no frame
loop. Hover, capture and explicit range focus keep an eligible bar visible;
release starts a fresh hold. A fully hidden bar cannot acquire a pointer capture.

Activity deadlines have opaque identities independent of intermediate frames.
Replaced activity, policy changes, hidden/inactive/removed axes and close reject
stale deadlines and paint samples. Ineligible activity is ignored rather than
replayed on reappearance. Invalid policy updates leave the old model untouched.

`scrollbar_clock` connects this model to GPUI's executor and window callbacks.
Each non-cloneable Owner retains one cancellable idle task and at most one queued
frame slot. Rendered drivers, timers and frame callbacks refer to it weakly;
updates invalidate obsolete drivers without accumulating frame callbacks. The
prepare/finish pair disarms owners omitted from actual painting. Closing or
dropping an owner releases its timer; ordinary idle/settled states request no
frames. The future renderer must call these hooks and supply real visibility,
modal, disabled, active-window and measured-overflow eligibility.

A deadline that expires between sample and paint is still scheduled at zero
delay, so its hide notification cannot be lost. Early platform timer delivery
clears the completed task before rearming only the remaining interval. The first
TestPlatform run found an invalid render-phase lookup during this rearm; the
adapter now captures the target entity ID during paint and carries only that ID
into callbacks. A separate test correction explicitly flushes GPUI's deferred
entity collection after dropping its scene; polling the executor alone does not
perform that App update.

## Verification

All commands use the repository environment with `GPUIO_JOBS=2`:

| Command after `./scripts/gpuio exec` | Result |
| --- | --- |
| `cargo test --offline --locked -j2 -p gpuio-native --lib scrollbar_lifecycle` | Eight deterministic lifecycle tests pass |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib scrollbar_` | 21 focused tests passed after the early-timer fix; the subsequent full suite includes an additional layout/paint deadline-race regression |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib` | 773 passed; two existing private-D-Bus tests skipped outside their isolated bus |
| `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings` | Pass |
| `cargo fmt --all -- --check` | Pass |
| `dune build -j2 examples/gallery/main.exe` | Pass, rebuilding the native backend and linking the gallery; no desktop launch |

The five GPUI TestPlatform scheduler tests exercise real queued frame callbacks
and executor timers: exact expiry without idle polling, 200 repeated updates
with one frame slot, omission, close/drop, stale drivers and policies, early
rearm, and an idle deadline crossing between sampling and paint. Eight pure
timing tests cover phase boundaries, interrupted/rejected previews, alternate
entrance, visibility modes, focus/capture, reduced motion, backward time, invalid
updates and stale identity. These are native scheduling tests, not physical
macOS input, IME, VoiceOver or GPU qualification. No OS windows were opened.

No Core or protocol code changed in this checkpoint; the previous full OCaml
and 357-test protocol evidence remains applicable to those unchanged contracts.
No dependency pins or vendor files changed. Native handle ownership and balanced
list drag hooks, actual painting/hit-testing, AX/keyboard, public admission/quota
accounting and ordinary/list/tree/table integration are still required.
