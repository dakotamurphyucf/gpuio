# macOS v1 performance qualification — OCH-17

Status: initial qualification targets, declared 2026-10-04 before the optimized
acceptance runs. No workload is certified by this plan. Historical development
build timings remain diagnostics. Changes to targets require an explicit dated
rationale and a new run; do not retroactively turn a failing result into a pass.

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

Each workload must verify content, event ordering and cleanup as well as timing.
Use deterministic text/data generation and publish exact sizes/seeds with its
runner. Warm up explicitly, then record three independent measured runs. Report
all runs and outliers. Aggregate CPU, native submission and physical observations
must remain separate.

| Workload | Required exercise | Initial target |
| --- | --- | --- |
| Loaded variable-height list | 10,000 loaded records; mixed 64/256/2048-byte UTF-8 bodies; repeated complete forward/backward traversal, row growth and anchor changes | Native draw p95 ≤16.7 ms, p99 ≤33.4 ms; process peak RSS ≤1 GiB |
| Logical paged table | 100,000 logical records, 64 columns, bounded loading; cover every row range, both ends, selection and reveal | Same draw targets; process peak RSS ≤1 GiB; loaded/mounted counts stay within the declared window/cache bounds |
| Growing document | Append deterministic Markdown/code through 20 MiB, while navigating/selecting/copying; exercise source reset and profile removal | Interactive draw targets as above; process peak RSS ≤1.5 GiB; report append→publication and preparation separately |
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
alone for lossless acceptance. No upstream fork change is needed for these APIs.

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
run the matrix and investigate failures. Retain platform/hardware/build metadata,
raw samples, phase markers, resource counters and cleanup status in each report.
Current chart staged diagnostics help locate delays but do not satisfy this
matrix or its predeclared targets. Linux desktop performance stays in OCH-47.
