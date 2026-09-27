# Native charts (OCH-40)

Status: in progress. The validated Core data model and paired bounded codecs are implemented and tested.
The extracted Sankey layout source compiles against the existing GPUI revision.
The native resource store, scoped Eio scheduler and application/host transport now
pass local ownership and windowless macOS integration tests. Widgets, interactions,
accessibility and public graphical examples remain. This document separates implemented contracts from
the remaining implementation work.

## Data contract

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
Source-dependent redraw/interaction integration remains part of the chart widget
work; current publication refreshes native windows.

## Explicit reduction policy

`Gpuio.Chart_sampling` now defines validated policies, shared with native code:

| Data family | Default | Opt-in alternatives |
| --- | --- | --- |
| Line and area | Extrema envelope, at most 1024 x buckets | Exact source points; a different envelope bucket limit |
| Bar | Exact bars | Per-bucket sum or mean |
| Candlestick | Exact candles | Per-bucket OHLC aggregation |
| Pie, radar, Sankey | Exact bounded source data | No implicit aggregation |

`max_buckets` is in [1,8192]. Effective bucket count is the smaller of this limit
and the plot's rounded-up logical-pixel width. Buckets divide the numeric x domain,
shared across Cartesian layers; they do not divide array indices or derive
identity from labels. The pure reducer accepts finite widths in (0,32768].

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

The native implementation must cover all seven families; the pure model and
Sankey extraction do not replace that scope:

- Connect borrowed resource handles to native chart views and validate multiple-
  window readers, unmounting, source-dependent redraw and interaction retirement.
  Do not put large datasets in each reconciled view node.
- Define axes, native formatting, legends, labels, palette/stroke/fill tokens and
  tooltips, including non-color distinctions. Shared Cartesian layers provide
  useful custom combinations. Preserve the pinned families' applicable styling
  and plotting options while keeping the public API typed.
- Connect the implemented exact/envelope/sum/mean/OHLC policies to prepared geometry
  and semantic selection. Bound native geometry/cache memory separately from
  source data. Expose complete source provenance for aggregates; never substitute
  line sampling for bar or candle aggregation.
- Rust owns layout, retained paint geometry, hit testing, hover and drag. Reuse
  prepared plans across idle frames and bound background jobs/caches. Data and
  label access cannot synchronously call OCaml from layout or paint.
- Provide stable, revision-checked semantic selections, keyboard navigation and
  meaningful accessibility/data-table alternatives. Native observations are
  asynchronous; changing data or closing a window cancels stale gestures/events.
- Build a polished public Chart Lab with every family, empty/degenerate cases,
  streaming updates and interaction. Validate actual macOS paint/input/AX behavior;
  Linux build/unit gates are required, GUI evidence remains separately recorded.
- Measure named-hardware dataset size, CPU/frame work, transport bytes and retained
  memory. Integrate a chart into OCH-29's broader application without replacing its
  custom-canvas and independent-extension requirements.

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
