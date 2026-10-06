# Metal presentation qualification — OCH-17

Status: bounded core collector and native Metal hook qualification, 2026-10-05.
[Core attribution/ownership tests](../evidence/presentation-core-och17.md) and
[the two-window Metal hook](../evidence/metal-presentation-hook-och17.md) pass locally.
**Repeated optimized GPUI presentation workload acceptance remains open.**
The existing [CPU/submission qualification](performance-qualification.md) remains
valid within its stated scope; it is not evidence of physical frame presentation.

## Observation source

Use `MTLDrawable.addPresentedHandler` and the callback drawable's `presentedTime`.
Apple defines this as a host-clock time for onscreen presentation; zero denotes
an unpresented or skipped frame. Keep GPU-command-buffer completion separate.
A callback's execution time includes scheduling delay and must not replace the
presentation timestamp. These are OS-reported presentation times, not a
photodiode measurement of pixels emitting light.
[Apple timestamp documentation](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime),
[callback documentation](https://developer.apple.com/documentation/metal/mtldrawable/addpresentedhandler(_:)).

The installed macOS SDK declares these APIs available from macOS10.15.4. Its
`CABase.h` specifies that `CACurrentMediaTime` converts `mach_absolute_time` to
seconds. The standalone probe retains submission bounds, GPU execution times,
presentation time and callback host time separately, and validates ordering.
The renderer needs to attach the callback before its existing present/commit
path without changing `presentsWithTransaction`, pacing or drawable ownership.

## Native interface and ownership

Add a separate **default-off presentation diagnostic feature**, layered on the
existing profiler. Enabling CPU histograms alone should continue to have its
previous behavior. No OCaml application API or production transport change is
needed. The qualification extension can export extra versioned diagnostics.

The draft Rust integration surface is a window-scoped `PresentationSession` with
`start(&Window, Limits)`, `snapshot()` and `stop()`. A session contains bounded
measurement state, never a Window/Entity/NSView/CAMetalLayer/MTLDrawable handle.
Snapshots are owned values and include the exact session/window identity and
sample epoch. `stop` prevents new admission; already admitted callbacks may settle
until the driver's declared deadline. Report remaining in-flight frames explicitly.
A second simultaneous session for the same window must return an error rather
than replace another collector silently. Other backends report Unsupported.

The pinned core's `Window::present` brackets `platform_window.draw`. Install a
scoped thread-local frame context around that call, restoring the prior context
on exit/unwind. It carries only measurement state, a sequence, and copies of the
profiler's matching frame/input metadata. The Metal renderer takes that context
when registering its drawable callback. This avoids global FPS attribution,
NSView-address reuse and extra window ownership. Multi-window and nested-context
fixtures must prove attribution before native qualification.

This uses a [small maintained adaptation](gpui-apple-adaptation.md) to the pinned `gpui_apple` renderer,
plus the existing core profiler adaptation. Do not edit shared Cargo git sources
or swizzle Objective-C methods. Extend the existing reconstruction tool for
`gpui_apple`, using the same verified Zed archive/revision, and record the new
patch hash. Update composed-backend manifest generation, all affected lockfiles,
source/distribution inventories and exact reconstruction checks. Normal builds
must remain compilable with the feature disabled on both macOS and Linux.

## Bounded records and clocks

Declare limits before implementation: at most 128 admitted unfinished frames per
session; an optional raw trace holds at most 4,096 completed records. Saturation
increments explicit lost/admission counters. It must never evict an unfinished
frame or silently turn a partial trace into complete evidence. Histogram counters
remain cumulative and bounded independently of raw-trace retention. No file I/O,
OCaml callback, view invalidation or unbounded queue runs on a Metal callback.
Short critical sections protect only measurement data.

A record identifies the session, GPUI frame sequence and drawable identity, then
keeps these values distinct:

- New scene versus resubmission, active/animation state, oldest contributing input
  and number of inputs coalesced into this frame.
- CPU draw/submission boundaries from the existing profiler.
- Host-clock submission bounds and Metal-reported presentation time.
- Callback arrival time, completion/failure/zero-time status, and clock-conversion
  uncertainty if an input timestamp is converted between clock representations.

Do not subtract an `Instant` epoch directly from Metal host seconds. For paired
input-to-presentation latency, bracket a host-clock read with `Instant` reads and
carry its uncertainty, or combine matching same-frame duration intervals with
explicit sample bounds. Validate that correspondence synthetically and under
native input. Never add independently computed percentiles. Keep callback delay
separate and retain cold-start/outlier frames in the raw evidence.

Out-of-order callbacks, zero timestamps, failed acquisition/encoding, missing
callbacks at settlement, histogram overflow, late callbacks after stop and trace
saturation each need explicit accounting. Completed records can be reordered by
their bounded admission sequence; interval computation must not assume callbacks
arrive in presentation order. Closing a window cannot let old callbacks update a
replacement session or retain native window resources.

## Qualification sequence

1. Complete locally: qualify the platform API with the small self-closing Metal probe. It requests
   120 frames at 60Hz only after activation/visibility, retains every record, and
   rejects missing/invalid/zero/duplicate/unordered results. This is calibration,
   not a GPUI performance benchmark or proof of a 60Hz display rate.
2. Complete locally: reconstruct the diagnostic adaptations and scoped Rust
   collector; controlled tests cover ordering, missing/zero/saturated outcomes,
   clock bounds, multi-window identity and close/stop behavior. The native
   two-window probe qualifies the Metal hookup and retirement; it has no input
   workload and establishes no performance budget. Hosted checks remain required.
3. Open: integrate the actual GPUI workload drivers. Preserve the accepted sizes,
   content checks and existing CPU/resource budgets. Declare any new presentation
   thresholds before runs; do not choose thresholds from these calibration data.
4. Open: compare diagnostic versus ordinary builds, then collect three independent
   optimized runs with enough actual presented/input samples. Record display,
   visibility, power, thermal state, interference, missing/dropped counts and
   settlement. Keep startup, active animation, typing and idle phases separate.
5. Recheck idle/close behavior so measurement itself adds no continuous redraws
   or retained native resources. Linux build/unit/consumer checks stay required;
   Linux desktop presentation remains OCH-47.

VoiceOver is unrelated to this measurement. The owner authorized VoiceOver
validation again on 2026-10-05; its separate accessibility evidence is still required.


## Workload transport and predeclared budgets

Declared before the first integrated workload run. The separate
`examples/performance_presented` backend enables presentation diagnostics in the
qualification extension; its four executables copy the existing OCaml workload
sources unchanged. Normal applications and CPU-only qualification retain their
feature selection. The extension retains the v3 command/event protocol: Begin
starts collection alongside the CPU snapshot; Finish stops admission before its
CPU cutoff, settles admitted callbacks for at most two seconds, emits one bounded
`GPUIO_PRESENTATION` schema-1 JSON report, then acknowledges Finished. Settlement
and export are outside the CPU interval. Unmount cancels the task/session.

The Python driver pairs each native report with its CPU phase/counts, rejects
missing/extra/reused sessions, invalid/lost samples, unfinished callbacks and
truncated visibility observations. The 4,096-record raw trace may truncate only
with explicit accounting; cumulative histograms still include all outcomes.
Beginning, end and one-second observations qualify sampled visibility/activation,
not continuous OS visibility. `idle_observation_mode` identifies Begin_idle's
smaller observation buffer; the CPU phase name defines whether a workload is idle.

For each of three full optimized non-idle runs, require at least 1,000 presented
frames, submission-to-presentation p95 ≤33.4 ms and p99 ≤50 ms (two/three nominal
60 Hz periods). The full streaming/typing run also requires at least 1,000 paired
input frames, native input-to-presentation p95 ≤75 ms and p99 ≤125 ms (the existing
CPU input budgets plus a 25 ms compositor allowance). Pair actual same-frame
metadata; never add independently computed percentiles. Input timing begins at
GPUI dispatch and excludes the preceding OS event queue, keyboard hardware and
photon latency. Existing content, CPU, queue and resource budgets remain required.
Scheduled animation intervals are diagnostic: a 20 Hz streaming producer does not
justify a blanket 60 FPS limit. Idle must have zero admitted/presented frames.

Smoke runs validate accounting and actual keyboard correspondence without applying
full performance budgets. Ordinary versus instrumented overhead, three full runs,
resource retirement and required hosted checks remain separate acceptance work.


## Idle-to-active correction — 2026-10-05

The initial blanket zero-skipped-frame gate was too strong for the available
Metal backend: a controlled native-only GPUI probe skips its first few frames
after two seconds of idle, while the identical binary without the delay presents
all180 frames. A standalone AppKit/Metal clear-pass probe, with no GPUI, OCaml or
collector, reproduces two zero-time frames after idle despite successful GPU
completion. The exact OS/display cause is unknown. This is evidence about known
native outcomes, not permission to drop records or treat zero as presentation.
The original runs remain failed against their original gate.

For new workload runs only, report a separate initial idle-to-active transition.
Allow a contiguous prefix of zero-time callbacks **only when none contains input**
and the first positive presentation occurs within **100 ms** of the first
submission. This reuses the existing UI input p99 responsiveness ceiling as a
hard transition bound, rather than fitting a skip percentage to the observed
counts. Retain every raw prefix record and report the conservative transition
latency. All zero outcomes must be accounted for in that prefix, including when
the remaining raw trace truncates. A zero after a positive presentation, any
input-bearing zero, no eventual positive presentation, or a transition exceeding
100 ms still fails. Missing callbacks, invalid clocks, dropped attribution,
saturation, pending frames and histogram overflow remain failures.

Presented-only p95/p99 thresholds and sample floors remain unchanged; skipped
frames never enter those histograms. Idle still requires zero attempts. All actual
native input frames still require matching CPU/presentation counts. This does not
claim a gap-free physical120 Hz stream or general recovery of inputs from skipped
frames. Full optimized repetitions, overhead/resources and display provenance stay
required. No renderer pacing or production application behavior changes for this
correction. Apply the new criteria to fresh runs, not relabelled old artifacts.
