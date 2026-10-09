# macOS v1 performance qualification — OCH-17

Status: initial qualification targets, declared 2026-10-04 before the optimized
acceptance runs. No workload is certified by this plan. Historical development
build timings remain diagnostics. Changes to targets require an explicit dated
rationale and a new run; do not retroactively turn a failing result into a pass.

## Focused v1 scope — owner direction, 2026-10-08

The owner requested concentrating performance qualification on the important
paths where scale or frequent updates make performance a user-facing risk.
This dated scope revision supersedes the earlier requirement to qualify the full
four-workload timing matrix for every candidate. It does not change production
limits, relabel historical failures as passes, or lower the retained core budgets.

| Release requirement | Focused acceptance |
| --- | --- |
| Long variable-height lists | Keep the 10,000-record full traversal/growth/anchor workload, its existing draw/presentation and memory budgets, and three independent valid optimized trials. Resolve observable jitter and unexplained idle redraws on this path. |
| Rapid updates while typing | Keep four streams at 20 updates/s for 120 seconds with native typing at 10 keys/s, existing content/input/queue/draw/presentation and memory budgets, and three independent valid optimized trials. |
| Idle and bounded resources | Keep settled focused/unfocused idle and representative repeated-window cleanup/retirement checks. Reuse valid existing full lifecycle evidence after an explicit change-impact review; rerun affected paths when necessary. No separate exhaustive performance matrix per component. |
| Large tables and Markdown/code documents | Preserve existing three-trial timing results, exact revisions and known limits. Keep content, virtualization/admission, bounded-memory and cleanup correctness coverage. Additional full timing batches are diagnostic unless a reproducible regression implicates these paths or a relevant change invalidates their coverage. |
| Startup/chart historical outliers | Retain the failures and investigated visibility behavior. Unproven original causes remain documented. They are follow-up diagnostics rather than separate root-cause completion gates; a reproducible current user-facing stall, blank window or critical-workload regression still requires a fix. |
| Measurement overhead | Validate collector correctness/retirement and compare ordinary versus instrumented behavior for the two core workloads. Reuse a representative comparison when it exercises the same collector configuration; add measurements only for a demonstrated validity gap. Do not expand this into a second exhaustive matrix. Quantify uncertainty rather than assume zero overhead. |

There are two mandatory timing workloads, with shared idle/resource checks.
Prior passing table/document results are retained, not repeated merely to satisfy
an obsolete blanket matrix requirement. Required correctness CI remains intact.
The subsequent developer-preview decision separates hosted timing availability
from physical core-workload performance: stable-release hosted qualification
moves to OCH-164. The [preview CI classifier](../evidence/hosted-metal-preview-och17.md)
distinguishes complete all-zero-time reports from failures while preserving
correctness/cleanup checks. Portable validation passes; native and hosted validation
remain. Crashes, malformed reports and ownership failures must still fail.
[The closeout checklist](../milestone-07-closeout.md) tracks the remaining work.

## Reference environment

The available reference is MacBookPro18,2, Apple M1 Max, 32 GiB unified memory,
macOS 14.5 arm64. The built-in display reports 1728×1117 logical pixels,
3456×2234 physical pixels and a 120 Hz mode. Mode metadata is not measured FPS.
Record the actual mode, power/thermal state, application size/scale and concurrent
activity for each run. Use a 1200×800 logical content area at 2× scale where the
workload supports it. Run one owned GUI workload at a time, with no simultaneous
GPUIO compilation. Record interference rather than discarding outliers silently.

Use an optimized paired OCaml/Rust build, exact revision and executable hash.
Keep stock OCaml 5.3, Bonsai v0.17 and the pinned dependencies. Record diagnostics
features separately from ordinary release builds and compare their overhead.
The initial responsiveness target is a 60 Hz work budget on this display; it is
not a promise of 120 Hz animation or measured physical presentation.

## Workloads and targets

The focused scope above determines which rows are mandatory timing gates.
The table retains the original workload definitions and budgets for interpreting
existing reports and running targeted diagnostics.

Each workload must verify content, event ordering and cleanup as well as timing.
Use deterministic text/data generation and publish exact sizes/seeds with its
runner. For mandatory timing workloads, warm up explicitly, then record three
independent measured runs. Report
all runs and outliers. Aggregate CPU, native submission and physical observations
must remain separate.

| Workload | Required exercise | Initial target |
| --- | --- | --- |
| Loaded variable-height list | 10,000 loaded records; mixed 64/256/2048-byte UTF-8 bodies; repeated complete forward/backward traversal, row growth and anchor changes | Native draw p95 ≤16.7 ms, p99 ≤33.4 ms; process peak RSS ≤1 GiB |
| Logical paged table | 100,000 logical records, 64 columns, bounded loading; cover every row range, both ends, selection and reveal | Same draw targets; process peak RSS ≤1 GiB; loaded/mounted counts stay within the declared window/cache bounds |
| Growing document | Grow three deterministic Markdown/code sources to 8, 8 and 4 MiB (20 MiB retained in aggregate), while navigating/selecting/copying; verify the 8 MiB per-source limit, reset and profile removal | Interactive draw targets as above; process peak RSS ≤1.5 GiB; report append→publication and preparation separately |
| Streaming and typing | Four independent streams, 20 updates/s each, 128 UTF-8 bytes/update for 120 s; native composer receives 10 keys/s while history grows | Input→submitted-frame p95 ≤50 ms, p99 ≤100 ms, no missing/duplicate text; draw targets above; process peak RSS ≤1 GiB |
| Window/resource lifecycle | Three warm-up cycles then 30 open/exercise/close cycles, including documents/images/canvas/extensions | Exact owned registrations, pending operations and native entities retire; final ten-cycle RSS baseline growth ≤64 MiB, with all samples retained |
| Settled idle | Stop streams/animations and blur caret; wait 2 s, then observe 60 s, both focused and unfocused | Zero newly drawn application frames; timers/measurement polling must not invalidate the window |

RSS limits apply to these defined workloads, not arbitrary applications or the
library's public admission limits. Report macOS physical footprint and GPU/IOSurface
observations separately where available; neither RSS nor charged source bytes is
complete GPU memory. Ownership counters must reach their specified zero even if
an allocator retains pages. Explain any plateau instead of calling it a leak or
claiming it harmless without evidence. Measure full-history traversal; sampling
four positions is insufficient for that acceptance item.

Require at least 1,000 draw samples for draw percentiles and 1,000 relevant input
samples for input percentiles. Fewer samples are an incomplete measurement, not
a pass. Report sample counts, maximums and all excluded/coalesced input events.
Cold start, first publication and inactive/occluded periods are separate named
phases. A render callback that waits while minimized is not a CPU-frame sample.

## Instrumentation boundaries

The pinned GPUI `profiler` feature exposes cumulative per-window histograms for
native draw, dirty→platform submission, active-animation submission intervals,
input→frame and input events coalesced per frame, plus mid-draw dropped-event
counts. Aggregate histograms populate without enabling the global trace ring.
An opt-in qualification collector should use histogram deltas at phase boundaries
and retain the original histograms/counts; subtracting percentiles is invalid.
Global `FrameTimingCollector` events can be overwritten and are insufficient
alone for lossless acceptance. The snapshot APIs need no adaptation. Native text
services can bypass ordinary platform-key dispatch, however: the GPUIO profiler
adaptation also brackets direct text replacement, composition, unmark and paste
callbacks. Only callbacks that invalidate the window contribute a timestamp;
nested callbacks use their enclosing input interval. This adds no redraws and is
inactive without the optional profiler. See
[the GPUI adaptation](gpui-core-adaptation.md).

Collect on the Rust UI thread without notifying or refreshing views. Keep native
sampling separate from OCaml UI-domain/reconciliation, serialization/upload,
worker preparation and queued-event delivery. Record command count/bytes/peak
from the existing mailbox metrics. Production queue capacities remain unchanged:
64 commands, 128 responses and 128 input events, with separately enforced byte
limits. A sampled empty queue does not prove its peak was zero.

Input histograms measure native input dispatch through submitted frames, not
keypress hardware latency or photons. CPU draw and platform submission similarly
do not establish GPU completion or physical FPS. Actual compositor/presentation
measurement and representative physical interaction remain separate release
evidence. Never relabel requested render callbacks as physical paint.

## Remaining implementation

The opt-in Rust integration API `gpuio_native::performance` is gated by Cargo
feature `performance-diagnostics` (default off). `Snapshot::capture(&window)`
reads cumulative native counters without scheduling frames or enabling tracing;
`after.since(&before)` rejects mismatched windows, reversed timestamps and
regressed buckets. Each interval exposes sorted raw buckets, nearest-rank
percentiles, sample counts and dropped/coalesced input counts. Empty distributions
have no percentile. Duration bucket bounds are nanoseconds, with upstream HDR
quantization; counts-per-frame are dimensionless. Capture overhead must be
measured separately. This is a Rust qualification/integration API, not an OCaml
application API or an automatic report writer. Window owners choose phase
boundaries and capture before retirement. Snapshots contain no window ownership.

The optional feature enables the pinned GPUI profiler and locks its existing
optional `hdrhistogram` dependency at 7.6.0. Default application dependency graphs
and runtime behavior remain unchanged; profiled distribution notice inventories
must include the additional dependency. Required CI runs the collector's pure
and TestPlatform tests on macOS/Linux. These do not establish physical latency.

Complete reproducible workload drivers and report export, validate collectors
against deterministic/synthetic traces, measure instrumentation overhead, then
run the focused scope above and investigate relevant failures. Retain platform/hardware/build metadata,
raw samples, phase markers, resource counters and cleanup status in each report.
Current chart staged diagnostics help locate delays but do not satisfy this
matrix or its predeclared targets. Linux desktop performance stays in OCH-47.

## Document workload correction — 2026-10-04, before measurement

The original single-source 20 MiB target contradicted the accepted 8 MiB
per-document API/protocol limit in [documents](documents.md). Preserve that
production limit and the 64 MiB aggregate source quota. The workload instead
retains three growing sources of 8, 8 and 4 MiB together, exercises each native
viewport, and verifies that an append past 8 MiB is rejected without mutation.
This is aggregate large-document coverage, not support for a single 20 MiB
document. Three full runs now pass this aggregate formulation; see
[document evidence](../evidence/growing-document-och17.md). Draw
and RSS targets are unchanged. Preparation remains bounded: Markdown above
64 KiB uses source fallback, so report that state explicitly rather than claiming
20 MiB of rich Markdown parsing.

## Explicit idle visibility sampling — 2026-10-05

The idle driver measures an initially focused-then-blurred native editor in an
active window and then an inactive window, with two-second settling and 60-second
intervals. It checks native activation transitions, independently observed AX
frontmost state, and all native histogram deltas. No nonzero draw/submission or
input sample is allowed in either settled interval. See the
[idle driver](../../examples/performance_idle/README.md).

The opt-in `WindowObservation::capture` reads native activation and AppKit's
occlusion state without invalidating a view. Other backends/TestPlatform report
unknown visibility; collectors must not convert it to true. Qualification probe
schema v3 adds Begin_idle with cancellable, bounded native sampling at absolute
one-second deadlines. Ordinary Begin and production backends do not start that
sampling. Finish returns observations after stopping the task. Sampled visibility
is separate from physical presentation and cannot rule out transitions between
samples. The exact rectangular opaque reference window and collected native/AX
state must be reported; an occluded window cannot be accepted as settled idle.

Three full optimized focused/unfocused runs now pass the idle contract; see
[raw reports and evidence](../evidence/idle-performance-och17.md).

## Native entity audit isolation — 2026-10-05

The dedicated [resource audit backend](../../examples/resource_audit/README.md)
enables GPUI's existing leak detector separately from ordinary performance
executables. It uses the same acknowledged lifecycle workload, captures live
entity IDs after the final closed warmup, and requires no new live entity handles
after each subsequent close. The collector waits for both native and application
retirement records before the next window. This does not force GC/cache purges,
retain windows in the audit, or extend retired component event delivery.

Its own tracking overhead is excluded from responsiveness evidence. The
intentional retained-entity fixture must fail and pass after release; malformed
or missing audit records cannot satisfy completion. This coverage complements
OS physical-memory accounting and cannot substitute for it. The smoke and [three full native audit runs](../evidence/native-entity-retention-och17.md)
now pass locally.


## Presentation follow-up — 2026-10-05

The [Metal presentation design](metal-presentation-qualification.md) now specifies
a separate default-off diagnostic, scoped window/frame attribution, bounded
callback data and clock-correspondence requirements. The [standalone API probe](../evidence/metal-presentation-calibration-och17.md)
passes120 actual drawable callbacks on the reference Mac. This establishes the
platform mechanism only: GPUI integration, collector validation and actual workload
presentation measurements are still required. Existing CPU/submission budgets
are unchanged; no new threshold was chosen from calibration timings.


## Active presentation collector retirement — 2026-10-08

The optional resource-audit `--presentation` path starts a dedicated session on
mount, stops on close and drops the measurement owner before closed-window
entity/Metal checks and memory sampling. This closes a coverage gap in the older
lifecycle audit, whose CPU probe did not reliably start presentation collection
on short cycles. The test uses the existing one-second native settlement and
three-warmup/thirty-cycle structure; no resource limit changes.

Per-cycle value records must show actual supported drawable activity, unique
session/window identities, closed/stopped state, zero pending frames and exact
outcome accounting. The external collector requires them before acknowledging
the next cycle. Settled zero/missing/invalid-clock outcomes remain resource-only
observations; they do not pass presentation timing requirements. Production
rendering and the separate workload timing gates are unchanged. See the
[implementation and repeated resource evidence](../evidence/presentation-retirement-och17.md):
all three full 33-cycle trials pass on the recorded physical Mac, retaining every
callback outcome and physical-memory checkpoint. This does not qualify collector
overhead or relax the separate timing requirements.

The [paired overhead attempts](../evidence/collector-overhead-och17.md) retain an
inactive-window smoke timeout and a later full instrumented idle failure with
recorded input. A startup foreground preflight is documented, but the planned
three paired repetitions remain incomplete; no overhead estimate is accepted.
