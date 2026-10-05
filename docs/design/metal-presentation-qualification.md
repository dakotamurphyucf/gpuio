# Metal presentation qualification — OCH-17

Status: design and standalone API calibration, 2026-10-05. **The GPUI renderer
integration and measured GPUI presentation workloads are not implemented yet.**
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

This requires a small maintained adaptation to the pinned `gpui_apple` renderer,
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

## Acceptance work still required

1. Qualify the platform API with the small self-closing Metal probe. It requests
   120 frames at 60Hz only after activation/visibility, retains every record, and
   rejects missing/invalid/zero/duplicate/unordered results. This is calibration,
   not a GPUI performance benchmark or proof of a 60Hz display rate.
2. Implement/reconstruct the diagnostic adaptations and scoped Rust collector;
   test callback ordering, missing/zero/saturated outcomes, clock bounds,
   multi-window identity and close/stop behavior with controlled fixtures.
3. Integrate the actual GPUI workload drivers. Preserve the accepted sizes,
   content checks and existing CPU/resource budgets. Declare any new presentation
   thresholds before runs; do not choose thresholds from these calibration data.
4. Compare diagnostic versus ordinary builds, then collect three independent
   optimized runs with enough actual presented/input samples. Record display,
   visibility, power, thermal state, interference, missing/dropped counts and
   settlement. Keep startup, active animation, typing and idle phases separate.
5. Recheck idle/close behavior so measurement itself adds no continuous redraws
   or retained native resources. Linux build/unit/consumer checks stay required;
   Linux desktop presentation remains OCH-47.

VoiceOver is unrelated to this measurement and remains on the owner's hold.
