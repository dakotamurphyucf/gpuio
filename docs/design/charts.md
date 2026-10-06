# Native charts (OCH-40)

The validated Core data model and paired bounded codecs are implemented and tested.
The extracted Sankey layout source compiles against the existing GPUI revision.
The native resource store, scoped Eio scheduler and application/host transport now
pass local ownership and windowless macOS integration tests. Typed plotting options
and retained logical-pixel geometry now cover all seven families. The prepared
painter passes local hidden-window GPU readback for every family. The resource-backed
Core/Bonsai view now mounts the painter through bounded native workers. A public
seven-family Chart Studio and hidden production-view lifecycle test pass locally.
Native labels, scrollable legends, typed selection payloads and prepared hit
queries, mounted interaction, tooltips and a bounded original-data companion are
implemented. Bounded non-color identification and two-window/list frame accounting
now pass locally. Public large-data streaming measurements now have a reproducible local baseline;
Signal Studio integration and its repeated resource workload also pass locally.
The paired bridge now advertises chart resources, rendering and interaction with
bit43 (`8796093022208`); its current aggregate is `17592186044415`.
[PR #14](https://github.com/dakotamurphyucf/gpuio/pull/14) records required hosted
gates, the checked revision and merge. These contracts and local evidence do not
substitute for that completion record.
The chronology below retains earlier unadvertised/unfinished checkpoints; this
delivery record supersedes those states.

## Data contract

Cartesian orientation now supports [reversed value directions](chart-directions.md)
in addition to the original vertical/horizontal projections. This opt-in change
preserves source and selection identity. [Categorical data and native point/band
layout](categorical-charts.md) add explicit ordered category domains, missing
observations and category-aware original-data access. Explicit native
[stacking](stacked-charts.md) adds cumulative bars/areas with preserved raw values.

`Gpuio.Chart_data` contains immutable values, with abstract positive IDs and
validated constructors. Labels are data, never identity or callbacks. Datum IDs
are scoped to a Cartesian series, or to the dataset for slices/candles/radar axes;
series, flow-node and flow-edge IDs have separate abstract types. Updates retain
IDs when the application means the same item. A label or array index is not a
substitute for identity.

| Family | Data and invariants |
| --- | --- |
| Line | One or more named series; strictly increasing numeric x; optional y marks an explicit gap |
| Area | The same series contract, with zero baseline; negative values remain valid |
| Bar | Numeric x, required y, zero baseline; negative values remain valid |
| Mixed Cartesian | Line/area/bar layers share numeric coordinates and unique series IDs |
| Categorical Cartesian | Ordered category IDs/labels, aligned line/area/bar layers, explicit missing observations and native point/band spacing |
| Pie | Unique labeled slices with nonnegative values; all-zero input is valid |
| Radar | 3–64 unique named axes with positive maxima; each named series provides exactly one value in each axis's domain |
| Candlestick | Increasing numeric x and unique IDs; finite OHLC with low <= open,close <= high; negative/zero-height candles are valid |
| Sankey | Unique nodes/edges, existing distinct endpoints and acyclic topology; nonnegative values; parallel edges, isolated nodes, zero flows and imbalance are valid |

Every family accepts wholly empty input. Finite numeric values have absolute
value <=1e100, leaving room for bounded sums and f64 normalization. NaN/infinity
are rejected, never used as gap sentinels. Chart adapters must normalize in f64
before converting to screen coordinates; valid raw magnitudes need not fit f32.

There are at most 32 Cartesian or radar series, 100,000 Cartesian points or
candles, 256 pie slices, 256 flow nodes and 2,048 flow edges. Radar has at most
64 axes. Text is valid UTF-8 without NUL/CR/LF, bounded to 128 bytes for series
names or 256 for item labels and 8 MiB total per dataset. Names and labels that
identify slices, axes or nodes are nonblank. Point/candle labels may be empty,
allowing a later native numeric formatter to supply their textual alternative.

Constructors perform no implicit sorting, aggregation or downsampling. They
validate source data and preserve its IDs, values and gaps. `value_count` includes
zero/missing values. `Expert.contents` exposes a read-only view for protocol and
data-table adapters without an unchecked constructor for validated datasets.

## Binary data boundary

`Chart_data.Expert.encode/decode` and Rust `decode_chart_data` share a version-1
standalone bin_prot envelope, capped at 16 MiB. The payload tags are Cartesian,
pie, radar, candlestick and Sankey; Cartesian layer tags distinguish line/area/bar.
Typed Core IDs convert only at this boundary. These are resource payloads, not
large inlined view properties. The resource store below implements publication
through a correlated application transport.

Both readers bound list counts before allocation, cap total Cartesian points
across all series, and charge all decoded text against the shared 8 MiB budget.
They reject truncated input, unknown versions/tags, invalid UTF-8 and trailing
bytes. Rust validates all domain invariants before returning a dataset. The Core
adapter passes decoded records through its validating constructors and rejects
oversized in-memory raw envelopes before constructing a second set of records.
Conversion stops on the first invalid element. Raw protocol types by themselves
are not validated domain values.

The OCaml reader uses explicit forward recursion for stateful reads. Independent
fixtures caught an initial reversed-point reader before acceptance; the fixed
reader preserves encoded series, point, axis/value and edge order. The public
model's numeric order invariants are enforced after decoding, not assumed from a
well-formed bin_prot record. ASCII whitespace classification is explicit and equal
across both language implementations.

`test/fixtures/chart-v1-data.hex` contains independent hand-encoded examples for
all payload tags and all Cartesian layer tags, including negative OHLC/values,
missing points and raw UTF-8. Both languages construct their own typed values and
compare their writers/readers against those same bytes. Every fixture truncation
and an appended trailing byte are rejected. Malformed domains, huge advertised
counts, per-series-valid but aggregate-oversized datasets, text-budget overflow
and a valid 100,000-point native decode are also covered.

## Resource ownership and publication

`Chart_resource` is a borrowed, application-owned identity. A registration owns
the dataset independently of any mounted chart; copying a handle cannot extend
the registration's lifetime. The wire operations are Create, Begin, Chunk,
Publish, Abort and Release. Begin names the expected published revision, its
successor revision, logical generation and encoded byte count. Initial revision
and generation are both 1. A reset may advance generation by exactly one on a
successful publication; ordinary updates retain it. Slots have a separate native
generation and never wrap. Chunks are contiguous, nonempty and <=256 KiB; the
complete encoded dataset remains <=16 MiB.

`rust/native/src/chart_store.rs` admits at most 256 registrations, four staged
uploads and two decode jobs. It reserves a conservative 64 MiB validation
workspace before issuing each sendable job, with 256 MiB total charged native
resources, input buffers, jobs and retained snapshots. Decoding runs without GPUI
or OCaml access. Cancellation is checked before and after the bounded decode;
it does not interrupt every decoded element. Cancelled work and completed-but-not-
consumed work keep worker/staging permits until actually reaped. Release cannot
create additional worker capacity while an old worker is still running.

Publication replaces the live reader slot only after successful decode/validation
and exact upload-token validation. The private token also fences an aborted upload
followed by a retry of the same revision, or a completion from another store with
the same numeric IDs. A failed decode consumes its upload and preserves the prior
publication. Abort of the next unpublished revision is idempotent, allowing uniform
cleanup after either admission or decode failure. Release/close invalidates live
leases immediately. Immutable snapshots already acquired by workers remain valid
and charged until their final reader drops them. Retired empty lease handles retain
their fixed bookkeeping charge too, until the last handle is dropped. A worker must return its completion
to the store; a host deliberately dropping an unrun job must abort its upload.

Retained native charges use actual vector/string capacities after decoding. The
workspace and fixed bookkeeping charges are conservative admission accounting,
not allocator/RSS measurements; later render-plan allocations need their own
bounded budget. No chart resource capability is advertised yet.

`Gpuio_eio.Chart_registry` is the UI-domain scheduler behind the public
`Gpuio_eio.Chart` API. It permits 256 registrations, four staged uploads, one
correlated request in flight and 128 MiB of conservative retained-data/encode
charges. It prioritizes cleanup and rotates progress between registrations.
Unstarted changes coalesce to the latest desired data; an in-flight publication
finishes before the latest update begins. Rejection preserves accepted data and
requires explicit retry. Scope cancellation suppresses late callbacks and releases
an allocation even when its Create response arrives after cancellation. Application
shutdown must also close the native store.

Reset intent has a separate OCaml epoch so coalesced resets never skip native
generations. Semantic revision admission rejects old reset epochs immediately,
including queued events from a currently publishing upload. Events from the exact
current Publish may precede its acknowledgement; later widget payload validation
must additionally validate the selected IDs. Distinct value-equal datasets are
charged separately; only the same physically shared immutable value is deduplicated
within a registration. A registration's quota includes conservative conversion and
encode space, but does not account for arbitrary application-owned data outside it.

`Chart.create app ~scope data` completes after the first successful native
publication. `Chart.set` and `Chart.reset` accept local desired updates;
`is_published` and `error` distinguish native acceptance from pending/rejected work.
`Chart.handle` returns the borrowed `Gpuio.Chart_resource.t` for future chart views.
`App.Diagnostics` reports scoped chart counts and conservative source-byte charges,
separately from serialized traffic. These are not native/GPU memory measurements.

The application protocol appends Message tag 21 and Event tag 61 without changing
existing tags. At most 63 raw Expert requests can be pending, with a further lane
reserved for scoped upload and cleanup. The native host dispatches admitted Work
through GPUI's background executor and returns Publish's reply only after atomic
commit on the native thread. Its completion channel is bounded to two results;
normal shutdown, emergency abort and OS quit close chart admission, cancel work,
retire pending replies and wait for worker exit. Repeated closers retain the same
completion fences, so an OS quit cannot bypass an asynchronous shutdown already
waiting. Late replies after transport closure are discarded under the mailbox lock.
Successful publication and release now invalidate dependent chart states and
retained managed-list presentations. Retirement clears displayed readers even
before a subsequent paint. Native chart interaction retirement remains part of
the pending selection/input work.

## Explicit reduction policy

`Gpuio.Chart_sampling` now defines validated policies, shared with native code:

| Data family | Default | Opt-in alternatives |
| --- | --- | --- |
| Line and area | Extrema envelope, at most 1024 x buckets | Exact source points; a different envelope bucket limit |
| Bar | Exact bars | Per-bucket sum or mean |
| Candlestick | Exact candles | Per-bucket OHLC aggregation |
| Pie, radar, Sankey | Exact bounded source data | No implicit aggregation |

`max_buckets` is in [1,8192]. Effective bucket count is the smaller of this limit
and the plot's rounded-up category-axis extent (height for horizontal charts).
Numeric buckets divide the shared numeric x domain; categorical buckets use
projected category positions, including padding. Neither derives identity from
labels. Categorical Sum/Mean spans preserve missing observations in provenance
and exclude them from arithmetic; Mean divides by present-value count. The pure reducer accepts finite widths in (0,32768].

Line/area envelopes retain each bucket's first, minimum-y, maximum-y and last
source point, deduplicated and ordered by source position. Every contiguous
defined run is reduced independently. A missing value breaks the path even when
both sides occupy the same bucket. A retained vertex identifies an original source
point, not an invented average. This is a visual reduction, not lossless retention
of every intermediate shape detail. Many short runs may retain more than four
points per bucket; geometry admission must report a limit rather than silently
joining gaps or discarding whole runs.

Bar sum/mean and candle OHLC are explicit aggregate operations. Each result stores
the complete half-open contiguous source index range relative to the immutable
dataset revision. That range maps back to original stable IDs, which need not be
sorted or consecutive. Aggregate x is the midpoint of the first/last source x.
Bar sums use compensated summation, preserving small contributions across large
opposing values. Their magnitude may exceed the 1e100 source bound but remains
finite under the 100,000-value input limit. Mean divides the compensated sum by
the range length. OHLC takes the first open, last close, maximum high and minimum
low. It never uses line-envelope selection.

`rust/native/src/chart_reduce.rs` implements this boundary without drawing or UI
callbacks. It validates source data/policy, checks cancellation and returns owned
reduction metadata plus source/rendered counts and retained-vector byte accounting.
It leaves the immutable source dataset untouched. Render plans must retain the
matching snapshot, cache reduction outside paint and expose aggregate provenance
through semantic selection. The view configuration, prepared geometry and actual
native rendering still need to consume this policy; defining and testing the
kernel alone does not provide a rendered chart.

## Pinned implementation assessment

Inspected the clean local GPUI Kit checkout at
`84f57fdfcb4910623fb0bb7f795b077e249f9271`, including all seven exports under
`crates/component/src/chart` and the scale/shape/axis/grid/label/tooltip modules.
The full styled component crate uses a different GPUI package pin (`gpui-pre`
0.3.1), theme access and plotting macros. GPUIO keeps its existing GPUI revision
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` and its own style/ownership boundary.

Line/area/bar/radar have optional hover behavior; the seven wrappers do not share
our required stable semantic selection, keyboard and accessible-data contract.
The line wrapper rebuilds scales during painting. Its point scale uses categorical
positions, whereas GPUIO's data model deliberately preserves numeric x spacing.
The low-level line shape collects defined points into one path, so GPUIO's explicit
gap semantics also require deliberate path segmentation. The linear scale's sealed
numeric types are f64 (or optional Decimal); integer arithmetic is not a concern
for the current source. Zero-extent domains return no tick and need an explicit
native empty/single-value rendering policy.

`rust/plot` now isolates the upstream Sankey topology/layout/ribbon implementation
from the styled component crate. Its Apache license, exact source/hash and the
namespace/iterator adaptations are recorded in `rust/plot/UPSTREAM.md`. All 12 retained
upstream tests pass against GPUIO's unchanged GPUI revision. This proves source
compatibility for that algorithm, not a rendered or interactive chart. The adapter
must still validate decoded data, bound iterations, normalize raw flow magnitudes,
retain original semantic values and cache layout outside per-frame painting.

## Remaining implementation contract

All seven families, mixed Cartesian layers, native interactions and original-data
alternatives now have local acceptance. Signal Studio integrates a chart alongside
its OCaml canvas and independently packaged extension; its fresh public consumer
and repeated lifetime workload pass. Required hosted gates and merge are recorded
on PR #14.
Linux GUI evidence stays separately recorded under OCH-17.

Continue to preserve explicit sampling/provenance, stable revision-checked
selections, asynchronous observations, native ownership and bounded resources.
No per-point synchronous OCaml callbacks belong in layout, paint or hit testing.

## Presentation options and prepared geometry

`Gpuio.Chart_options` is a validated, source-independent value with grouped
options for axes, Cartesian layers, pie/donut, radar, candlesticks and Sankey.
Irrelevant family options are retained but have no effect. Its version-3 binary
record (with categorical layout and stacking; versions 1 and 2 are rejected) has a bounded native reader (256 bytes), independent paired fixtures and
validation after decoding. Color/theme, legends, tooltips and the accessible
description belong to the mounted-view configuration.

- Axes use linear numeric domains, 2–12 ticks and native Compact/Fixed/Scientific/
  Percent formatting with 0–6 decimal places. Percent changes labels only.
  Categorical axes instead use the explicit domain labels and point/band positions.
  Empty numeric domains use [0,1]; constant domains center their value. Source x extents
  survive aggregation; bar values use the selected exact/sum/mean policy. Bar and
  area domains include zero. Tiny domains deduplicate ticks after f64 rounding.
- Cartesian options select Linear, Natural (uniform Catmull–Rom) or StepAfter,
  dots, four value-axis directions, category layout and grouped bar width. Natural curves
  can overshoot and require the mounted plot's clip. Every curve reaches its last
  data point. Missing values split runs, and singleton runs have visible dots
  even when ordinary dots are disabled. Mixed layers share category/value projections;
  bars are grouped by default. Explicit [stacking](stacked-charts.md) accumulates
  bars and areas independently, preserving raw values and matching boundaries.
- Pie/donut uses a 0–0.95 hole fraction and 0–0.2 radians of padding, clamped per
  slice. Zero slices have no area; all-zero input has no wedges. Labels sit within
  the ring when enabled. Radar uses data-defined axis maxima, 1–12 grid levels,
  optional labels/dots, and matches values by axis ID rather than input order.
- Candlestick body width is a fraction of nearest x spacing; OHLC source spans
  remain attached to marks. The future painter must draw rising bodies hollow,
  falling bodies filled and equal open/close as a horizontal mark, in addition
  to colors. Geometry retains all four values for that distinction.
- Sankey exposes all four alignments, Linear/Sqrt weights, node width/padding,
  0–32 relaxation iterations and labels. Raw flows normalize in f64 before the
  pinned layout engine's f32 coordinates. Fitting uses actual layer/column counts,
  preserving configured width when it fits and leaving positive node area in
  crowded columns. Original values remain in the immutable source snapshot.

`chart_geometry::prepare` consumes the implemented reduction policies and returns
paths, marks with revision-relative source provenance, labels and grid lines for
all families. Horizontal Cartesian plots use height as the sampling extent.
It performs no window operations, text shaping or OCaml callbacks. Plans report
actual retained vector/string capacities and reject more than 64 MiB; this is
not an application-wide allocator/RSS bound. The future worker must reserve peak
preparation space and charge retained geometry/tessellation separately from data.
The plan alone does not retain a resource or authorize stale data: its owner must
hold and validate the exact immutable snapshot.

These logical primitives now feed `chart_paint`, which retains tessellated
meshes and native quad specifications. `Gpuio.Chart_style` validates palette/theme
resolution, stroke/point/corner sizes, grid/axis/label/selection colors, bar
gradients and area opacity; its paired codec is bounded to 512 bytes. Source-layer
order is preserved across meshes and quads. Rising candles are hollow with wicks
outside the body; falling candles are filled, and unchanged candles have a
horizontal mark. The native painter passes actual local GPU pixel checks.

The shared mesh engine has a separate bounded allowance for native-generated
charts; canvas wire/mesh limits remain unchanged. This supports a 100,000-point
exact line and all default sampled curve modes without silently dropping source
points or inheriting the canvas wire's 4,096-command cap. Per-plan geometry,
mesh and quad retention is capped at 64 MiB. A shared frame budget reserves the
whole chart's draw vertices/quads before painting. Application-wide worker/cache
admission and initial mounted lifecycle are implemented below. Native
labels/legends/tooltips, interaction and end-to-end streaming measurements remain
outstanding. Multi-chart managed-list cache acceptance must also verify aggregate
frame accounting when GPUI reuses prior draw commands. See [chart evidence](../evidence/charts-och40.md)
for exact current coverage and limits.

## Current local evidence

Core expect tests exercise all families' empty/degenerate input, 100,000 source
points, numeric/text/identity limits, explicit gaps, negative values, OHLC ordering,
radar domains and graph topology. An independent reachability oracle enumerates
all 4,096 directed graphs on four distinct nodes and agrees on the 543 DAGs.
The plotting compatibility tests retain all 12 upstream Sankey cases, including
layout chains, cycles, degenerate inputs, alignment and value/width behavior.
Three additional Core codec expect tests and four Rust codec tests exercise the
paired boundary described above. These are model/codec/algorithm checks; native
chart acceptance is still outstanding.

Previous codec regression checkpoint: all **12 Core chart expect tests** and **191 Rust
protocol tests** pass locally, including four chart-codec tests. Strict all-target
protocol Clippy and both formatters pass. No graphical chart acceptance is claimed
from these checks.

The resource checkpoint adds independent request/response byte fixtures in both
languages, eight native store tests and scoped scheduler expect tests. Native
checks include actual worker-thread execution with 100,000 points, atomic reader
visibility, failed publication, cancelled-worker capacity, same-revision retries,
foreign/closed/stale completions, memory admission and 1,024 create/release cycles
with retained empty reader handles. Scheduler checks cover 1,000 coalesced updates,
reset/event epochs, explicit rejection/retry, cancellation at every upload boundary,
late Create cleanup, foreign scopes, 100,000-point chunking, quota rollback,
round-robin staging and cleanup priority. These use isolated store/scheduler APIs;
those isolated checks do not themselves exercise a connected application or graphical chart.

The subsequent application checkpoint connects the complete resource path.
`examples/chart_upload` passes on local macOS without opening windows: a real
100,000-point upload with gaps, 1,000 coalesced desired updates and reset, rejected
malformed data with unchanged native base revision, scope cleanup, 270 registration
cycles, 63 raw requests plus reserved scoped progress, and nine shutdown callbacks.
It asserts no rendering transactions occurred. A required macOS CI stage is added;
hosted execution remains pending. Native Session tests additionally prove publication
stays invisible until completion and shutdown releases pending-job accounting.
These checks establish resource integration, not rendered-chart acceptance.

Application checkpoint verification: the full local `dune build -j2 @all @runtest
@fmt` passes, including the independent backend example. The initial broad Rust
native/protocol regression run passes 602 tests; two additional host teardown
tests pass after the final shutdown refinement. The independent consumer lockfile
now includes the native macOS notification dependency without changing pinned
package versions, and older low-level examples explicitly handle the new milestone
6 response variants. Hosted macOS/Linux gates remain pending.

Reduction checkpoint: two Core policy expect tests and two paired protocol tests
fix policy tags, bounds, defaults and round trips, including every fixture
truncation. Nine native reducer tests cover extrema/source order, a separate
grouping oracle over 16,384 seven-point gap/value patterns at four widths, mixed
layers, shared numeric domains, signed compensated sums/means, negative OHLC,
empty/extreme/subnormal values, invalid inputs and cancellation. The 100,000-point
continuous fixture retains <=3,200 vertices at logical width 800; exact mode
retains 100,000. The alternating-gap fixture retains all 50,000 separate defined
runs at width 1. These counts and vector-capacity bounds are pure preparation
evidence, not native CPU/frame/RSS or graphical acceptance measurements.

Presentation/geometry checkpoint: 18 Core chart expect tests and two additional
Rust options-codec tests pass. The options fixture fixes 66 default bytes
independently in each language; native tests cover every truncation, malformed
tags/booleans/bounds, and 192 valid option combinations. Ten native geometry tests
pass all families, curve endpoints/gaps and singleton visibility, numeric grouped
bars, negative/aggregated OHLC provenance, donut proportions, reordered radar
values, empty/constant/subnormal/extreme domains and bounded native formatting.
They also cover all Sankey alignments/scales at five flow magnitudes and a
256-node two-column graph at sizes from 0.125 to 32,768 logical pixels. Exact
100,000-point area/candle plans fit the 64 MiB retained-plan admission bound;
sampled and 50,000-isolated-run fixtures stay below 16 MiB. These are capacity
checks in unit tests, not operating-system/GPU memory measurements. Strict native,
protocol and plot all-target Clippy passes on the unchanged toolchain/GPUI pin.
The full local Dune `@all @runtest @fmt` build passes, including the independent
extension backend; 639 Rust native/protocol/plot tests pass across 93 targets.
Both backend lockfiles add only the existing local `gpuio-plot` dependency.
Required hosted macOS/Linux validation remains part of the final milestone gate.

## Chart view bridge checkpoint (6b3a468)

`Chart.Config.create ~data:(Gpuio_eio.Chart.handle registration) ()` borrows the
application-owned registration. `View.chart ~on_event config` is also exposed by
`Gpuio_bonsai.View`; callbacks there return Bonsai effects. View style supplies
size; the chart-specific options, sampling and resolved style remain separate.
This earlier checkpoint established the typed description and transport. The
mounted rendering section below records the subsequent production integration;
the bridge-only tests are not themselves evidence of painted output.

The configuration carries the resource identity, a nonblank UTF-8 description
(up to 1024 bytes without NUL/CR/LF), options, sampling and style. No dataset is
embedded in a view transaction. A foreign application handle encodes no source,
allowing an eventual `Wrong_application` observation without dereferencing an
unrelated resource. The bounded Rust configuration reader admits at most 2048
bytes and validates every nested configuration. Existing wire tags are preserved:
chart kind 48, `Set_chart` operation 55 and `Chart_event` event 62.

Initial observations are `Ready metrics` and `Failed error`; selection events
remain to be implemented. Ready describes completion of preparation, not GPU
presentation. Metrics report original and retained values, mesh vertices, quads
and retained plan bytes; they are not frame latency or process-memory readings.
Every observation includes the data revision and reset generation independently
of the view-tree revision. Only a failure before acquisition can use 0/0.

Reconciliation rotates callback identity for configuration changes while keeping
node identity. Closure-only updates use the latest accepted callback without a
native mutation. Dispatch validates window/node/handler/tree revision and source;
the Eio registry additionally fences resource release, scope cancellation, logical
reset and accepted or exactly in-flight publication. Native tree transactions
require a configured chart leaf, bound chart nodes to 128 per window, charge
retained configuration memory and reject invalid updates atomically.

The mounted ownership contract is: unmount releases the view reader/work but
not the registration; release/close clears displayed data; ordinary publications
may retain the previous picture until replacement is ready; reset or source
change clears it. Background scheduling, aggregate worker/cache admission and
source-dependent managed-row invalidation now enforce the initial lifecycle
below; complete chart acceptance and capability advertisement remain pending.

## Mounted rendering and worker ownership

The bridge checkpoint above is now connected to the production host. Each native
chart state borrows a live data lease and keeps the exact immutable snapshot
beside its prepared geometry. Unmount/hide clears preparation and displayed
readers; it does not release the application's registration. Normal publications
may retain the previous picture while newer preparation runs. Reset, source
replacement and resource retirement clear it. Resize keeps the previous logical
size under the new element's clip until its replacement is ready.

The application render pool admits at most two workers and 128 live view jobs.
Requests coalesce to the latest snapshot/options/style/layout. Completion must
match the exact private cancellation token before releasing a worker slot, so
numeric identifiers alone cannot admit a foreign completion. Cancelled work and
queued completions remain accounted for through worker exit and delivery.
Shutdown closes admission/output, cancels work and joins workers; repeated
closers share the same completion fences and destruction provides a fallback
join. Workers never await a UI callback.

Retained plans have a separate 256-MiB quota. Before preparation each work item
reserves the painter's 64-MiB maximum plus 4096 bytes for fixed metadata, then
shrinks that reservation to retained plan capacity. Allocation admission failure
produces a typed render-limit observation; source data has its existing separate
store charge. A 192-MiB preparation allowance per worker is also admitted before
work and retained through delivery. It accounts conservatively for reduction,
geometry, native path conversion and tessellation scratch. It is **not a hard
allocator or RSS limit** on Lyon's private allocations. Per-source/path/output
counts remain enforced independently.

Successful publication/release updates only dependent production windows. Worker
completion invalidates chart-bearing retained rows in its observer window as well
as requesting redraw. Row invalidation refreshes retained measurement state;
the pinned list reconstructs and paints visible elements rather than replaying
cached view commands. Native acceptance now covers ordinary trees and two
managed-list windows, shared updates and per-frame accounting. Sustained
large-data measurements are recorded separately below.

[`examples/charts`](../../examples/charts/README.md) uses only public Core/Bonsai/Eio
APIs and visits every family in its self-test. Native tooltips, plotted-mark
keyboard navigation and the original-data companion are implemented and tested.
There is still no completed chart-rendering capability advertisement.


## Native labels and legends

`Chart.Config.create ?legend` defaults to `true`. Axis formatting comes from the
validated native options; family labels and legend names come from the exact
immutable snapshot that produced the displayed plan. GPUI text elements own font
shaping, clipping and accessibility. There are no synchronous OCaml text/layout
callbacks or additional application font caches.

Numeric plots reserve stable left/bottom gutters, swapping their roles for
horizontal Cartesian axes. Domain-value length does not change plot margins while
streaming. Radar labels have bounded surrounding gutters. Small views reduce
these margins proportionally and clip/ellipsize text rather than creating negative
plot dimensions. Very long labels retain full accessible names. Pie/flow labels have a neutral
backing chosen from the resolved foreground to remain readable over series fills;
terminal flow labels align inward from their nodes. Crowded labels
are not yet an automatic collision-avoidance or data-navigation mechanism.

A legend uses numbered names and palette swatches in columns of approximately
180 logical pixels. At most three rows, also capped at 30% of chart height, are
visible; the remaining entries use a native vertical scroll view. The source
schema limits legends to 256 entries (32 for Cartesian/radar). Candlesticks use
named hollow-rise/filled-fall entries. Ordinary updates preserve legend scroll;
reset, source replacement, hidden state and unmount clear it. No new permanent
timer is introduced. Hidden visual legends can be requested with `~legend:false`.

Preparation uses only the inner plot dimensions. The installed plan, snapshot,
style and text frame move together; normal pending updates retain all four, and
reset/retirement clears them together. Worker results are installed before the
native element tree is rebuilt, so labels and plot cannot represent different
publications in the same frame. Text/legend payload bounds are separate from the
prepared-mesh byte/vertex measurements; those metrics are not total GPUI memory.
A legend and axis labels are not a substitute for the upcoming complete keyboard
and data-table alternatives or non-color series identification in the plot.

## Semantic selection contract and hit-test preparation

The public `Chart_selection` module (also `Chart.Selection`) describes a target
in the publication named by `Chart.Event.data_revision` and `data_generation`.
Cartesian targets carry a stable series ID, source span and `Exact`, `Sum` or
`Mean` aggregation. Candles carry a source span and an OHLC-aggregation flag.
Slices, radar series/axes and Sankey nodes/edges use their distinct stable IDs.
The source span is a validated half-open index interval with stable endpoint IDs;
its endpoints **are not a numeric ID range**. IDs need not increase with source
order. Exact points/candles contain one datum. Envelope sampling still selects
an original single representative; it is not misreported as an aggregate.

`Selection_changed of Selection.t option` is appended after existing Ready/Failed
observations. `None` represents an explicit clear. The payload contains fixed-size
fields instead of transferring every member of a large aggregate. Selection
observations require a positive publication identity and pass the existing
application/resource/tree-handler/revision/reset/release fences. Core construction,
wire decoding and native provenance resolution each validate their boundaries.
The source resolver takes the exact immutable dataset and sampling policy that
produced a mark. It does not look up indices in a newer publication.

Each prepared painter now owns a worker-built hit index. A median-split hierarchy
bounds mark candidates; leaves contain at most eight entries. Dots, bars, candle
bodies/wicks, pie annuli, Sankey nodes and curved ribbons use analytic tests.
Nodes take precedence over ribbons as in painting; overlapping points choose
the nearer point, with draw order resolving ties. Cartesian line/area columns
also support a nearest plotted-sample fallback between points. That result is an
existing datum, not an interpolated value or a claim that a gap contains data.
Queries outside the plot or with non-finite coordinates return no target.

The index admits at most 100,000 marks and 16 MiB of retained arrays, counted
inside the existing 64-MiB prepared-plan limit and render-pool charge. Cancellation
is checked during preparation; queries do not allocate or rebuild geometry.
Spatial pruning reduces ordinary query work, but heavily overlapping geometry
can still require many candidates; no unconditional logarithmic-time guarantee
or measured frame-performance claim is made.

## Mounted input and selection ownership

`Chart.Config.create ?disabled` defaults to false. The chart owns a native focus
handle, local hover/keyboard preview and pointer capture. Only the plot receives
selection gestures; legend/gutter interaction does not implicitly select a value.
A left click or drag released inside the plot commits its current target; release
on blank plot space explicitly clears. Release outside the plot cancels the drag.
Hover and drag previews update native text and a marker without OCaml events.
The tooltip reads the displayed immutable source. Actual bar/OHLC reduction values
are retained during worker preparation, so hovering an aggregate does not rescan
a large source interval or infer values from pixels.

Arrow keys, Home and End preview plotted marks. Enter or Space commits a preview;
Escape cancels an active drag, or clears the committed selection otherwise. Tab
cancels previews and follows normal focus traversal. The focused chart participates
in the central modal/focus policy; disabled, hidden or pointer-inert charts reject
the corresponding input. **This is plotted-mark navigation, not the complete
original-data alternative:** missing, zero-area and downsampled-away originals
are available through the companion table described below.

Native callbacks carry an identity token. A data/config change, reset, source
replacement, hide, release or close invalidates old callbacks and cancels capture.
Blur/window deactivation also cancels previews/capture. Input requires that the
prepared snapshot be the registration's current publication and that the displayed
frame/config match the requested ones. A retained older picture while preparation
is pending is not permission to emit a selection from stale geometry.

Ordinary same-generation publications preserve a selected singular stable ID when
that ID remains in the newly prepared marks. Its source position is resolved again;
old offsets are never reused. Aggregates clear on publication because matching
endpoint IDs cannot prove unchanged membership. A worker-built sorted identity
index admits at most 100,000 entries and 8 MiB, charged inside the prepared plan's
64-MiB limit. Reset/source replacement/retirement clear selection. Automatic
reconciliation is silent; `Selection_changed` is emitted only for a user commit or
explicit clear and names the exact publication used. Applications that display
selection-derived values must interpret those values against that publication.

Redraw invalidates the affected managed row and owning view, with deferred view
updates to avoid reentrant native state borrows. There is no permanent input timer
and no synchronous OCaml callback from native input/layout/paint. Chart Studio now
shows public semantic callback results; its AppKit acceptance covers all seven
families and real pie hover/click/drag/cancellation.

Broader-example integration and consolidated gates remain required before the
chart ticket is complete.

## Original-data companion

Every chart has a native **View data** control; D opens it while the chart is
focused. It displays a read-only table of original values, independently of
sampling or successful mesh preparation. Line/area gaps appear as `Missing`,
zero slices and flows remain present, and isolated flow nodes remain browsable.
Cartesian rows follow layer/source order, radar follows series/axis order
(resolving values by axis ID), and Sankey exposes nodes followed by edges.
Numbers use their round-trippable source representation, not rounded axis ticks.
Stable source IDs and supplied labels accompany the values.

The table has at most ten mounted rows, with fewer rows at smaller chart heights.
Native accessibility exposes the full row count, absolute row indices, cells,
values and an active browsing row. No dataset-sized AX tree, row-copy cache or
additional source registration is constructed. Source lookup is bounded by the
schema's series/axis/flow limits; it never scans 100,000 points to access one row.
Allocate sufficient chart height for the toolbar, rows and paging controls
(144 logical pixels accommodates one row); undersized views clip normally.

Arrow keys move the browsing row, Home/End reach the first/last original, and
Page Up/Down move by a page. Previous/next page controls and AX row focus provide
the corresponding accessible actions. D, Escape or **Back to chart** returns to
the plot. Browsing does not change the committed plot selection or emit
`Selection_changed`: the table is a data-reading alternative, not a second
application selection model.

The table takes one immutable snapshot from the current live registration when
building its bounded page. It can therefore read data while plotting is pending
or rejected by rendering admission. Old plotted pictures and current original
data are separate presentations. Routes retain only a weak snapshot reference
and verify publication identity, callback identity, disabled state and focus/
pointer policy before changing the browsing position. Stale actions cannot apply
an old row offset to a replacement publication.

Ordinary publications preserve the numeric browsing position, clamped permanently
to the new count; growing again does not restore an out-of-range old position.
This is page-position retention, not selection of a stable datum. Reset/source
replacement, hidden state, unmount or release close the companion. Blur cancels
plot previews but retains the chosen presentation. Switching to the table skips
plot paint/text work while retaining its bounded prepared plan for a return to
the plot. It introduces no permanent polling, extra upload or synchronous OCaml
callback.

Chart Studio's **Edge cases** mode supplies a 100,000-point line with gaps,
empty area data, signed/zero bars, a zero pie slice, reordered radar values,
a negative flat candle, and a graph with an isolated node and zero-valued edge.

## Non-color identification and presentation

Multi-series Cartesian and radar plots repeat numeric identifiers at up to three
actual representative positions per series (96 labels maximum), matching the
numbered legend. The native accessible name includes the series name. Numbers
follow current publication order; stable IDs remain the semantic identity. Empty
series have no fabricated plot markers. Preparation runs on the bounded worker
and charges label storage to its retained plan.

Pie and flow labels identify slices/nodes, ribbons connect named flow endpoints,
and candles distinguish rising/falling/flat values through hollow/filled/line
geometry. Dense or coincident values can still overlap: the tooltip, keyboard
inspection and complete original-data companion remain necessary alternatives.
Turning family labels off deliberately removes that text cue; the library does
not guarantee collision-free charts for arbitrary datasets.

Layout reserves a 32-logical-pixel data-control header within the existing
bounded top gutter. Tooltips sit below it; donut selection markers sit toward
the inner edge to avoid the usual slice-label position. Companion controls use
resolved label/selection colors with derived neutral backing and borders.
Chart Studio demonstrates mixed area/bar/line layers, horizontal grouped bars,
light and monochrome styles, and a scrolling dense legend. Real macOS wheel
acceptance checks the last legend row, retained offset after a new publication,
and return to the first row after reset.

## Managed lists and shared sources

The pinned GPUI list retains measurements/scroll state but reconstructs visible
row elements and paints them each frame. GPUIO does not embed these rows through
`ViewElement::cached`; cached-view draw-command replay is a different GPUI path.
The root resets a per-window chart budget before child painting. Every visible
chart reserves its complete plan before drawing. Native integration now verifies
that the counter equals the sum of both visible plans on repeated frames and
after scrolling, with GPU pixels confirming both rows painted. No broad row
invalidation or GPUI patch was introduced for this accounting check.

Two windows can observe the same source while keeping independent list offsets
and prepared chart instances. Local hidden-window acceptance covers fourteen
shared publications, disposal of one observer followed by another update, and
idle release of the source in the survivor. This is bounded lifecycle coverage,
not a sustained large-dataset performance benchmark or foreground input test.

## Streaming measurements

The public workload covers 10k/100k sampled, 100k exact and coalesced eight-update
bursts, with native render callbacks and complete source checks. The
[baseline](../evidence/chart-streaming-och40.md) records named hardware, process
CPU/RSS, frame geometry, update latency, bridge bytes and queue/retention charges.
`App.diagnostics.native_command_queue` snapshots accepted native command count,
current serialized-size charge and lifetime peak under a short mailbox lock.
It adds no command/wake and excludes executing work/output events.

## Configurable inspection presentation

The [inspection contract](chart-inspection.md) adds typed card, crosshair and
marker controls through `Chart_style.create ~inspection`, preserving native
preview and committed-selection ownership. [Local qualification](../evidence/chart-inspection-och41.md)
includes actual pixels and root/installed OCaml gallery walkthroughs. The current
style envelope is -2 after [rich Sankey node labels](chart-node-labels.md);
options/data versions are 5/1 following the
[Sankey ribbon-color addition](sankey-link-colors.md). Broader chart presentation
options and whole-catalog/release acceptance remain separate.


## Radar scale, radius and spacing — options schema 7

See [radar presentation](radar-presentation.md) for the shared-data/shared-explicit
maximum, fixed logical-pixel radius and label gap additions. Per-axis scaling and
fitted radius remain defaults. Explicit maxima can extrapolate outside the grid;
source IDs/values and original-data selection remain unchanged. Extreme projected
coordinates return Render_limit before tessellation.

## Ordinary Views as radar labels — chart-view schema -1

`View.chart ~radar_labels` accepts a validated `Chart_radar_labels` collection
of ordinary Views keyed by stable axis IDs. Native prepaint measures buttons,
text and editors without an OCaml callback. Content retains normal styling and
input semantics. Hide/show retains native children; removing an entry unmounts
it. Original source values and selection are unchanged. See the
[content contract](radar-label-content.md), [example walkthrough](../../examples/gallery/charts_page.md#ordinary-views-as-radar-labels)
and [scoped local evidence](../evidence/radar-label-content-och41.md), including
remaining interaction/lifecycle qualification. The explicit chart-view envelope
is -1; options/style/data are 7/-2/1. Matching bridge revisions are required.


## Pie fixed and per-slice radii — options schema 8

The [pie radii contract](pie-radii.md) adds a global Fit/Pixels outer radius and
at most 256 stable-ID overrides containing both inner and outer logical-pixel
radii. The existing global donut-hole fraction stays unchanged. Overrides affect
presentation and hit geometry while preserving source values, angular weights
and selection identities. Unknown IDs are ignored until present; equal radii
omit the wedge/caption without filtering original data.

Options schema 8 explicitly rejects version 7 and earlier. The default frame is
114 bytes; style/data/view versions remain -2/1/-1. Matching packages are required.
This supersedes earlier sections' current-options version references; their
original fixtures and evidence remain historical. Outside pie captions, leader
styling and label spacing remain separate catalog work.
