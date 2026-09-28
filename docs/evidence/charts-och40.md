# Native chart evidence (OCH-40)

This is an implementation ledger, not full ticket acceptance. Current local
machine: Apple M1 Max, macOS 14.5 (23F79), arm64. The repository's stock OCaml 5.3,
Jane Street/Bonsai v0.17 and pinned GPUI/Rust toolchains remain unchanged.

| Boundary | Evidence | Remaining |
| --- | --- | --- |
| Typed data/options/styles and paired bounded codecs | Core expect and Rust codec tests, independent byte fixtures, malformed/domain/size cases | Semantic selection envelopes |
| Scoped data publication | Public windowless macOS `chart_upload`: 100k source points, coalescing, reset, rejection, cleanup, shutdown | Multi-window/cached-list streaming acceptance |
| Geometry/reduction | All seven families; empty, negative, constant, extreme, subnormal, gaps, aggregates, large input | End-to-end streaming frame measurements |
| Prepared GPU painting | Hidden native window, real off-thread preparation, actual GPU readback for every family | Full chart text/input/AX and streaming acceptance |
| Styling | Validated palette/theme resolution; real alpha, bar corners/gradient, hollow/filled candles and mixed layer order | Native labels/legend/tooltip presentation and selection visuals |
| Interaction and accessibility | Not implemented for the chart widget yet | Pointer/drag, keyboard, meaningful AX/data alternatives and stale-event checks |
| Budgets | Per-plan geometry/mesh caps, conservative shared-frame admission, cancellation and bounded tessellation | Runtime worker/cache admission across mounted views/windows |
| Platforms | Local macOS GPU test; required macOS CI stage defined | Hosted macOS/Linux gates; actual Linux GUI tracked separately under OCH-17 |

## Prepared painter

`native_chart_paint` prepares each fixture on a real worker thread, then mounts
the retained painter in a hidden GPUI window (`show=false`, `focus=false`). It
checks GPU pixels, repaints the same prepared objects, closes its window on both
success and caught failure, and exits the native application. It does not test
the future OCaml `View.chart`, resource release notifications, keyboard or AX.

The cases cover line location, area transparency, grouped bar fill/corners,
two-color pie, donut hole, radar fill, rising hollow/falling filled candles and
their outside-body wicks, Sankey ribbons/nodes, a line above an earlier bar, a
vertical bar gradient, and plot clipping. Preparation fixes the paint order
across mesh and quad primitives; Sankey ribbons precede every node.

`chart_paint` retains tessellated meshes and native quad specifications. Paint
projects/submits those primitives; it does not validate source data, reduce
points, build curves or tessellate polygons again. Grid/axis strokes are prepared
too. Text shaping, legends, tooltips and semantic hit testing are separate pending
mounted-view work.

Native-generated chart paths use the shared canvas tessellator with a distinct
bounded allowance: 200,004 commands, 262,144 flattened segments, 1,000,000 vertices
and 1,000,000 indices per mesh. Serialized canvas inputs keep their existing
4,096-command and original mesh limits. A regression explicitly checks both
entry points on the same larger path. Per prepared chart, retained logical
geometry/mesh/quad capacity is capped at 64 MiB, expanded mesh draw vertices at
1,000,000, and quads at 300,000. A shared window-frame budget admits a whole chart
before submitting any of its primitives. These are explicit conservative
accounting/geometry limits, not RSS or a claim of application-wide admission.

Unit tests prepare a 100,000-point exact line; all three default sampled curve
modes; a sampled 100,000-point area; and 50,000 isolated point markers. An extreme
exact curved workload exceeds tessellation admission and returns `RenderLimit`
without exposing a partial plan. Source sampling never silently bridges gaps to
fit that limit. Shared-frame rejection leaves its counters unchanged, and source
cancellation cannot return a usable prepared result.

## Local checks

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib canvas_mesh --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-canvas-tests --test native_canvas --test native_chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-canvas-tests --locked --all-targets -j2 -- -D warnings
```

Core chart coverage is now 20 expect tests. The chart painter adds six native
unit tests and the style codec adds two protocol tests. The GPU success marker
is `GPUIO_NATIVE_CHART_PAINT_OK`; the existing canvas GPU regression remains part
of validation for changes to the shared mesh engine. The required macOS workflow
prebuilds the new test and gives it a separate bounded execution/log stage.
Hosted execution has not yet been claimed.
The final local run passes all commands above: 199 protocol tests, six shared
mesh tests, six painter tests, both actual native GPU scenarios, 20 Core chart
expect tests and strict feature-enabled Clippy. Every owned test process/window
exits after the run.

The earlier full Dune and 639-test native/protocol/plot regression checkpoint is
recorded in [the chart design](../design/charts.md). That count precedes the
painter/style additions; it must not be reported as the new suite's count.

## Core/Bonsai chart view bridge

The typed chart view is connected through reconciliation, the runtime driver,
scoped Eio event admission, paired protocol and native tree transactions. This
checkpoint does not mount the painter in a public chart view yet and adds no
rendering capability advertisement.

Local macOS checks cover:

- Independent configuration/transaction/event fixtures in OCaml and Rust,
  preserved wire tags, every truncated fixture, invalid labels and nested options.
- Foreign application handles, callback refresh without tree mutation,
  handler replacement on configuration change, stale/future tree events and unmount.
- Bounded preparation metrics; invalid epochs; current publication versus logical
  reset, release and application shutdown; allowed pre-acquisition failures.
- Atomic native tree rejection for missing/invalid/duplicate chart configuration,
  text/children on chart leaves, and the 128-view mount quota with reuse on removal.

Full Dune `@all @runtest @fmt` also passes after updating the two low-level
examples to explicitly handle the new event constructor. The full native/protocol
Cargo regression passes locally (639 tests, two existing ignored tests across
94 reported suites; this run excludes the separate plot package). Strict
all-target Clippy with `native-canvas-tests`, rustfmt and whitespace checks pass.

The chart suite now has 23 expect tests. The new Rust chart-view codec and native
chart-tree suites contain two tests each; the existing two sampling codec tests
also pass after its decoder was factored for nested configurations. These are
bridge and ownership checks, separate from the earlier actual painter GPU run.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 test/chart test/runtime
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check -p gpuio-native -p gpuio-protocol --all-targets
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol \
  --test chart_view --test chart_sampling -p gpuio-native --test chart_tree --locked -j2
```

## Mounted production charts and public application

The production native tree now mounts the prepared painter through bounded chart
workers. Four worker tests cover 1000 coalesced replacements, actual background
execution, held workspace/completion charges, exact-token fencing against foreign
pool completions, owner disposal, shutdown and retained-capacity failure/recovery.
Two render-host tests cover repeated closer fences and destruction waiting for a
worker without requiring UI progress.

The hidden macOS `native_chart_view` test passes actual GPU pixels and production
state checks for initial paint, style replacement, correlated asynchronous data
publication, reset, idle resource release, resource generation reuse, source
replacement, hide/show and unmount. It explicitly checks that release removes
ready geometry/jobs/readers **before** forcing another frame. Cleanup joins both
resource and render workers and requires zero data/plan/workspace charges.

The public [`Chart Studio`](../../examples/charts/README.md) self-test passes every
family through Bonsai, Eio and FFI, checks reset generations and a native render
callback, then releases its registration and shuts down. A separate local
background-window inspection captured line/donut screenshots, invoked the Pie
button through macOS accessibility, and closed the owned window through its OS
close button; the child exited successfully. These are chart rendering and
application-control checks, not chart selection/keyboard/data accessibility.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib chart_ --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-canvas-tests --test native_chart_view --test native_canvas_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio build examples/charts/main.exe
./_build/default/examples/charts/main.exe --self-test
```

Final local chart unit run passes 41 tests, including the six new worker/host
lifecycle cases. The production chart and existing canvas hidden-window suites
both pass; the chart final counters are six completed preparations, one peak
worker and zero retained plan/workspace bytes. Strict feature-enabled all-target
Clippy and full Dune `@all @runtest @fmt` pass. The final public self-test and
existing 100k-point windowless chart-upload/cleanup regression both pass against
the rebuilt application. These counters do not substitute for a streaming benchmark.

The required macOS workflow prebuilds and runs the mounted test and public chart
self-test with bounded execution. Hosted execution and Linux GUI acceptance are
not claimed. Native metrics and source snapshots are separately accounted;
workspace allowance is not measured RSS or a hard bound on Lyon allocations.

## Native labels and legend acceptance

Local macOS 14.5 arm64 / Apple M1 Max, isolated repository toolchain, two jobs.
The mounted chart now has native axis/family text and numbered palette legends.
The text frame, geometry, config and immutable source publication move together.
Numeric gutters remain stable as values change; horizontal axes swap margins.
Long text clips/ellipsizes with a full accessible name. Pie/flow text has a neutral
contrasting backing, and terminal flow labels align inward beside their nodes.

`Chart.Config.create ~legend:false` hides the visual legend. Its default `true`
is appended to the unreleased paired chart configuration fixture; Rust rejects
invalid/truncated Boolean encodings and OCaml verifies default/false behavior.
The legend has at most three visible rows and a 30%-of-height cap. Native GPUI
layout for 256 long names produces a real scroll extent, preserves its offset
through an ordinary publication, and clears it after a reset. Existing pixel,
restyle, release, source-reuse, hide/show and unmount checks remain passing.
Final native counters: 10 preparations, one peak worker, zero retained plan and
workspace charges. These are charge counters, not a process RSS measurement.

Checks passed:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib chart_ --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native \
  --features native-canvas-tests --locked --all-targets -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/charts/main.exe @test/chart/runtest @fmt
./_build/default/examples/charts/main.exe --self-test --foreground
python3 scripts/test_charts.py --foreground
```

The final chart unit run passes 44 tests; both protocol chart-view tests pass.
The full Dune `@all @runtest @fmt` also passed before the final native label
contrast/scroll refinements; the final targeted rebuild/tests/format and strict
Clippy pass after those changes. Public self-test visits all seven families,
checks preparation generations, requests a frame callback and releases its
registration. The macOS AX script checks numeric endpoint ticks, family labels
and legend names, invokes the update button, captures every family and closes
its child through the OS close button. Screenshots were inspected, including the
contrast/position fixes. The update button smoke is not a revision acknowledgment;
the separate native test verifies actual publication and scroll retention.

An initial background self-test timed out before reporting preparation. Its
sample showed an idle AppKit loop, not an active preparation worker. Explicit
foreground runs passed. The example and AX script now accept `--foreground`;
the required CI self-test uses it so occluded background windows do not make
frame-dependent acceptance unreliable. This does not change normal application
focus policy. Every owned GUI child was closed/reaped.

These are native text and application-control checks. They do not establish
chart keyboard navigation, semantic selection, dense-legend wheel gestures,
a complete data alternative, collision-free dense labeling, hosted CI or Linux
GUI acceptance. No new chart capability is advertised yet.

## Selection schema, provenance and hit-index acceptance

Local Apple M1 Max / macOS 14.5 arm64, the same isolated toolchain and two-job
limit. Typed Core selections and the appended `Selection_changed` observation
now have independent paired fixtures for exact/sum/mean Cartesian targets,
slices, radar series/axes, exact/OHLC candles and Sankey nodes/edges. Tests reject
invalid spans, IDs, aggregation tags, Boolean tags, truncation and trailing data.
Exact targets require one datum; aggregate endpoints preserve source order even
when their IDs decrease. Selection and explicit-clear observations reject
pre-data epochs and are each fenced by fresh reset, release and shutdown cases.

Native provenance tests resolve actual prepared geometry for all seven families.
They verify full original membership for summed/mean bars and OHLC candles with
IDs `42, 99, 7`, and exact original representatives for a 1,000-point envelope.
Malformed internal indices/family mismatches return no target without indexing
outside the source. The source resolver is paired with immutable data; this is
not a callback into OCaml or a lookup against a newer publication.

The prepared painter now owns a bounded spatial hit index, charged with its
geometry and meshes. An exhaustive mark-scan oracle agrees with indexed exact
queries across all families and both orientations. Separate analytic checks
cover donut holes, curved ribbon interiors and thin candle wicks. The wick test
caught and fixed a broad-phase margin smaller than its analytic hit tolerance.
Actual 100,000-point vertical/horizontal datasets pass exact-sample targeting,
retained-index limits, pruning checks, cancellation and non-finite/out-of-plot
rejection. Empty and nearest plotted-sample queries are also covered. These
checks demonstrate bounded storage and pruning on those workloads, not a frame
latency benchmark or a worst-case logarithmic query bound.

Final checks passed:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol \
  --test chart_selection --test chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib chart_ --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-canvas-tests --test native_chart_paint --test native_chart_view --locked -j2
./_build/default/examples/charts/main.exe --self-test --foreground
```

There are 51 passing native chart unit tests and four passing protocol selection/
view tests. Strict all-target native Clippy with the native-test feature passes;
full Dune checks pass, followed by another runtime expect-test/format check for
separate selection/clear retirement cases. The actual hidden-window GPU suites
still pass every chart family, mixed layers, clipping and production resource/
legend lifecycle. The mounted final counters remain ten preparations, one peak
worker and zero retained plan/workspace charges. The rebuilt public seven-family
self-test passes reset epochs, frame callback and scoped cleanup; its foreground
window exits cleanly. All owned test children were reaped.

**Not yet accepted:** mounted pointer/drag/keyboard handling, tooltips, real
semantic selection callbacks, complete data alternatives, multi-window/cached-row
streaming, or the required named-hardware performance report. This checkpoint
implements their typed contract and query machinery; it does not claim completed
interactive charts, hosted CI, Linux GUI acceptance or a new capability bit.

## Mounted input and public semantic callback acceptance

Local Apple M1 Max / macOS 14.5 arm64, repository toolchain, two build jobs.
Native chart input now owns local hover/drag/keyboard previews, focus and pointer
capture. Actual GPUI input dispatch through the production view passes:

- Hover and drag previews emit no selection events; click/release commits and
  changes the actual rendered marker pixels.
- Outside release and Escape during capture cancel without changing selection;
  Escape without capture emits an explicit clear.
- Home/End and Enter/Space preview/commit plotted values.
- Same-generation source reordering preserves singular selection by stable ID
  and resolves its new position silently. Source mutation immediately cancels
  capture. Disable/re-enable before repaint rejects retained old callbacks.
- Blur/reset/native release cancel capture and prevent a stale release from
  committing. Unmount drops chart state and all charged retained data/plans.

The separate production lifecycle test still covers actual paint, style change,
asynchronous publication, source replacement, hide/unmount and dense legend
scroll/update/reset. The input and lifecycle suites finish with respectively
`(3, 0, 1, 0, 0)` and `(10, 0, 1, 0, 0)` counters: completed, discarded, peak
worker count, retained plan bytes and outstanding workspace allowance. Both
assert an empty native data store on shutdown.

`python3 scripts/test_charts.py --input` passes against the rebuilt public
Bonsai/Eio application. Real AppKit keys focus each of the seven charts, preview
first/last plotted marks without changing the OCaml readout, commit with
Enter/Space, and clear with Escape. Real pointer hover/click/drag on the donut
shows the actual source values and reaches `Selection_changed` in OCaml;
outside release preserves the prior committed target. The script verifies native
labels, updates every family, captures screenshots and closes/reaps its child.
Global pointer events check that each target belongs to the child before posting.
A process-targeted pointer attempt could hover but did not deliver the click as
needed; the accepted test uses the existing native-test approach with actual
system pointer delivery. A first run also launched an older executable before
Dune finished linking; all final checks used the completed build. A Sankey test
expectation was corrected to its actual ribbon-before-node navigation order.

Additional worker tests verify actual Sum/Mean/OHLC tooltip values, descriptions
for every family's prepared marks, singular ID lookup after source-position
changes, removed IDs, cancellation, retained index storage and intentional refusal
to retain aggregate selection even when its span has one sample. There are now
55 passing native chart unit tests. Paired config fixture tests cover default and
explicit `disabled`, and reject malformed values in both appended Boolean fields.

Commands passed:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib chart_ --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native \
  --features native-canvas-tests --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @runtest examples/charts/main.exe -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @fmt -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-canvas-tests --test native_chart_input --test native_chart_view --locked -j2
./_build/default/examples/charts/main.exe --self-test --foreground
python3 scripts/test_charts.py --input
```

The final self-test also passes all seven families/reset epochs/frame callback/
scoped release. All owned GUI children are reaped. Native and public input stages
are added to the macOS workflow, but consolidated hosted execution remains
pending. No Linux GUI acceptance or completed chart capability is claimed.

## Original-data companion acceptance

Local Apple M1 Max / macOS 14.5 arm64, isolated repository toolchain, two jobs.
The native companion table reads original published data independently of sampling
and successful mesh preparation. It is read-only: browsing leaves plot selection
unchanged and emits no semantic selection observations. No extra public wire tag,
data upload, dataset-sized row cache or accessibility tree is introduced.

The production native input test now additionally verifies D/Escape switching,
source-row browsing without selection traffic, and rejection of a retained
previous-publication row callback. A test-only reservation exhausts the renderer's
actual retained-plan admission allowance; a reset publication then fails through
the real worker/host `Render_limit` path with no prepared picture. Its original
data remains keyboard-browsable. Releasing the allowance and changing the plot
configuration successfully retries preparation. This reserves accounting capacity,
not 256 MiB of test heap, and does not install a fabricated Ready/Failed result.
Source shrinking permanently clamps the browsing position; subsequent growth
cannot restore the formerly out-of-range position. Final native counters are
`(6, 0, 1, 0, 0)` for input and `(10, 0, 1, 0, 0)` for lifecycle, with empty data
stores and zero retained plan/workspace charges at shutdown.

The public `scripts/test_chart_data.py` passes real macOS AX and keyboard checks:

- A sampled 100,000-point line exposes AX row count **100,000** while mounting
  only **eight** native rows in the 330-pixel chart. The implementation caps pages
  at ten. End reaches original row 100,000; Home, Page Up, previous/next buttons
  and AX row focus move the browsing position without committing plot selection.
- Original missing values and source identities are accessible. Ordinary data
  update preserves the numeric browsing position; D/Escape switch presentation.
- Every family's edge-case dataset is read through the table: an empty area,
  signed/zero bars, zero pie slice, radar values supplied out of axis order,
  negative flat candle, isolated flow node and zero-valued edge. Sankey table
  count includes nodes and edges; plotted source-value metrics count its edges.
- Reset while the companion is open returns to plot mode. All children/windows
  close and are reaped after actual OS close.

The public check caught and fixed an empty derived flow sum rendering as `-0`;
isolated-node totals now display `0` in the table and tooltips. It also caught
incorrect test expectations for Core's `100_000` count formatting and Sankey
edge metrics, and a test pressing the prior publication's button during reset.
The test now waits for the chosen family's preparation before acting. Zero-row
metadata passed this test; an explicit 24-pixel minimum was added in the later
presentation checkpoint below (the earlier claim that it was already installed
was incorrect).

There are **58 passing native chart unit tests**, including original-data access
for every family, all 100,000 source positions, missing/zero values, bounded pages,
last/out-of-range positions and isolated-node totals. Strict all-target Clippy
with `native-canvas-tests`, Rust formatting, example compilation and Dune `@fmt`
pass. The complete public seven-family plot input suite still passes after adding
the companion, including real pointer/keyboard semantic callbacks and OS close.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib chart_ --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native \
  --features native-canvas-tests --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @fmt examples/charts/main.exe -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-canvas-tests --test native_chart_input --test native_chart_view --locked -j2
python3 scripts/test_chart_data.py
python3 scripts/test_charts.py --input
```

The data acceptance script is wired into the macOS workflow. Consolidated hosted
macOS/Linux execution and Linux GUI validation are not claimed by this checkpoint.

## Non-color presentation and public visual acceptance

Local Apple M1 Max / macOS 14.5 arm64, isolated repository toolchain, two jobs.
Multi-series Cartesian/radar identifiers are generated on the worker, at most
three per series / 96 total, and identify actual plotted representatives. Their
accessible names include series names. Legend numbers follow publication order;
semantic identity remains the stable ID. Coincident values may overlap, so this
is not a universal label-placement guarantee.

Chart Studio now includes mixed layers, horizontal bars, dense legend, monochrome
and light styling. Original-data controls derive their colors from chart style;
the table explicitly retains 24 pixels even when empty. Its title now has native
label semantics. A reserved header keeps View data clear of the upper axis labels;
the tooltip sits below it, and donut selection markers move inward.

Local checks pass:

- **61 native chart unit tests**, including both Cartesian orientations, missing/
  empty series, 32-series label bounds, reordered radar axes and light/dark control
  colors; strict feature-enabled all-target Clippy, Dune example build and format.
- Actual native input, prepared GPU pixels for all families and mounted lifecycle
  suites. Final counters remain `(6, 0, 1, 0, 0)` / `(10, 0, 1, 0, 0)` with zero
  retained charges. Native input verifies the relocated donut selection pixels.
- Public `test_chart_visuals.py`: mixed/horizontal/radar series identifiers;
  monochrome/light plots and companion; real OS wheel scrolling to Channel 128,
  its retained visible position after a distinct 128-to-129-value publication,
  and reset returning to Channel 001. Wheel input produces no plot selection.
- Full `test_charts.py --input` seven-family real AppKit keyboard/pointer callbacks
  and `test_chart_data.py` 100,000-original bounded-table regression. All children
  close and are reaped. Radar's ordinary example now has two series / ten values.

Screenshots of mixed/light, horizontal, radar, pie and original-data presentation
were inspected locally. Dense overlapping marks may obscure individual labels;
repeated identifiers and the inspection/data alternatives remain necessary.
The new visual acceptance is wired into macOS CI; hosted execution remains pending.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib chart_ --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native \
  --features native-canvas-tests --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @fmt examples/charts/main.exe -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native \
  --features native-canvas-tests --test native_chart_input --test native_chart_paint \
  --test native_chart_view --locked -j2
python3 scripts/test_chart_visuals.py
python3 scripts/test_charts.py --input
python3 scripts/test_chart_data.py
```

## Two-window managed-list frame accounting

The production `native_chart_view` test now mounts two hidden 240×240 windows,
each showing two of three 200×120 chart rows in a managed list. Repeated forced
frames assert that the window budget equals the sum of both visible prepared
plans, and actual GPU readback verifies green chart pixels in both rows.
Scrolling one window changes its visible rows without moving the other.
Fourteen shared-source publications reach the visible charts; explicitly disposing
and closing one window permits the remaining observer to receive the last update.
Release immediately drops its ready plans, jobs and data leases.

Local macOS acceptance passes with final renderer counters `(65, 0, 2, 0, 0)`: no
remaining retained-plan/workspace charge, at most two concurrent workers, and an
empty data store at shutdown. This extends the existing lifecycle test; it does
not add a new CI binary or any production renderer changes.

The earlier cached-draw concern was a review hypothesis, not a reproduced defect.
Inspection of the pinned GPUI `elements/list.rs` confirms measurement retention
with fresh visible row elements; `List::paint` invokes each row's paint. The native
host has no `.cached()` embedding. The regression confirms actual frame accounting
for this path. Initial fixture failures were retired/gapped node IDs, corrected
by using fresh windows with contiguous IDs. No list invalidation fix was needed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native \
  --features native-canvas-tests --test native_chart_view -j2
```

## Completion still required

Measure sustained large-dataset streaming CPU/frame/queue/retained-memory behavior
on named hardware, and integrate a chart into OCH-29 alongside its distinct
canvas/independent-extension requirements. Consolidated hosted macOS/Linux gates
and merge remain pending; no Linux graphical acceptance is claimed.
