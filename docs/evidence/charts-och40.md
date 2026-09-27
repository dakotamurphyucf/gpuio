# Native chart evidence (OCH-40)

This is an implementation ledger, not full ticket acceptance. Current local
machine: Apple M1 Max, macOS 14.5 (23F79), arm64. The repository's stock OCaml 5.3,
Jane Street/Bonsai v0.17 and pinned GPUI/Rust toolchains remain unchanged.

| Boundary | Evidence | Remaining |
| --- | --- | --- |
| Typed data/options/styles and paired bounded codecs | Core expect and Rust codec tests, independent byte fixtures, malformed/domain/size cases | Mounted view configuration/selection envelopes |
| Scoped data publication | Public windowless macOS `chart_upload`: 100k source points, coalescing, reset, rejection, cleanup, shutdown | Mounted resource readers and source-dependent redraw/retirement |
| Geometry/reduction | All seven families; empty, negative, constant, extreme, subnormal, gaps, aggregates, large input | End-to-end streaming frame measurements |
| Prepared GPU painting | Hidden native window, real off-thread preparation, actual GPU readback for every family | Resource-backed Core/Bonsai view and application example |
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

## Completion still required

Connect this painter through bounded, cancellable resource/view workers to
Core/Bonsai chart views. Validate data publication and release/close invalidation,
multiple-window readers, semantic selection and reset/revision fencing,
keyboard/AX/data alternatives, native labels/legends/tooltips and unmount/close
cleanup. Build the polished public all-family Chart Lab, measure actual streaming
CPU/frame/queue/retained-memory behavior on named hardware, and integrate a chart
into OCH-29 alongside its distinct canvas/independent-extension requirements.
