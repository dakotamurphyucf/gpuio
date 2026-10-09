# Pure evaluation fixtures, geometry and validated documents

[workspace.ml](workspace.ml) and [workspace.mli](workspace.mli) define Signal
Studio's immutable evaluation model. It owns four fixed sample identities,
positions, selected sample and run number, and derives canvas/chart values.
It owns no native handles, Bonsai graph, Eio tasks, filesystem access or actual
model measurements. The adjacent [test source](test/workspace_test.ml) shares
this guide because it checks these model contracts.

Build/run from the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
```

Select/drag a sample or Shift+arrows in the canvas, adjust the run counter, and
inspect the changing latency chart. Everything is deterministic illustrative
data, never LLM/provider output. macOS is the v1 native target; required Linux
nongraphical checks and [informational GUI coverage](../../../docs/platform-release-policy.md)
remain distinct. No external fixture file is needed for ordinary launch.

## Read the model and its units

Read Sample, `create`, accessors/`find`, `set_run`/`select`/`move`, `scene`, `chart`,
Document/`encode`/`decode`, then `scheme`/`route`. The abstract `t` stores samples,
run (initially 0) and optional selected item ID (initially None). Sample.t is also
abstract: ID, name, color and transform are exposed through meaningful accessors.
Initial samples are Swift(180,320), Sage(370,210), Atlas(510,100), Orbit(250,160)
with stable canvas item IDs 1–4 and fixed colors. Coordinates are world-space
logical pixels, not device pixels or encoded physical measurements.

Sample.position applies its transform to (0,0). Latency is `(x-64)*1000/580` in
milliseconds; quality is `(404-y)*100/340` percent. The 580×340 plot starts at
(64,64). Move clamps centers to x74–634/y74–394, reserving ten pixels for circles,
so actual center values stay slightly inside axis endpoints. Swift starts at
200.0 ms and about 24.7%. Selection IDs are not chart datum IDs even when both
use small integer fixtures.

`set_run` accepts only integers 0–100. `select` accepts None or a known sample
ID. `move` rejects unknown IDs and accepts a validated Point, clamping coordinates
and rebuilding only that sample's translation. It does not persist native scale/
rotation/pan; the application's Moved handler extracts the transform's origin
before calling it. Recoverable input errors return Or_error without modifying
the old immutable value. Literal geometry constructors use Or_error.ok_exn for
checked-in invariants.

## Derive a scene and a chart

`scene` creates seven horizontal/seven vertical grid lines, six axis labels,
160 deterministic decorative background points, four interactive colored circles
and their name labels. Item IDs occupy separate ranges (sample1–4, grid10–26,
axis30–35, cloud100–259, names30000+). Text resource IDs are a separately typed
namespace. Each sample has ellipse hit region, draggable/activatable metadata,
plot clipping and stable transform/ID. Background points have no interaction.
The scene description explains the simulation/accessibility meaning. Public
[geometry](../../../lib/core/canvas_geometry.mli),
[scene](../../../lib/core/canvas_scene.mli) and
[resource](../../../lib/core/canvas_resource.mli) contracts validate descriptors;
constructing one does not register or mount a native canvas.

`chart` creates four line series with stable series IDs/names. Each has 24 points
with stable within-series datum IDs, x = 0–23 and y = base sample latency plus a
15 ms sine perturbation using `(point_index + run)/4` and series index.
The run shifts phase; sample movement changes its base latency. There is no
randomness, timing sample or missing y fixture. Quality is canvas/inspector data,
not another chart series. [Chart_data](../../../lib/core/chart_data.mli) describes
validated chart values, separately from the scoped native chart controller.

The interaction trace in [application.ml](../application.ml) is native Moved
observation → deferred effect obtains latest workspace and extracted Point →
move returns new workspace → publish sets scoped scene/chart and observable
Ui.Snapshot → [Component](../component.ml)'s let%arr derives Ui.view.
A reactive value updates dependent views; a Bonsai effect describes a later
model/native operation. This module is the pure calculation between those layers,
and pan/zoom remain native. Publication acceptance is not physical paint evidence.

## Decode untrusted storage and narrow routes

`encode` writes version-1 S-expression with run, selected raw ID and all four
positions. Labels/colors, native handles, chart/scene registrations, viewport,
UI visibility and notification state are omitted and reconstructed from fixtures.
The internal Document representation derives sexp conversion, but `decode`
validates it before admitting a Workspace.t: ≤16 KiB input, valid syntax/version,
run bounds, each ID1–4 exactly once, finite in-bound Points and valid selection.
It sorts IDs to catch duplicates/missing/unknown entries, then reconstructs from
create. Decode **rejects** stored out-of-plot coordinates rather than clamping
them as interactive move does. Expected parse failures become Or_error.

`scheme` is gpuio-signal. `route` permits only route sample, exact paths /1–/4
and no query/fragment; /02 is rejected even though it looks numerically related.
It selects a known sample, never opens a resource or interprets a URL as a file
path. [Application identity](../application_identity.md) declares the scheme;
OS registration/readiness is a separate runtime/package concern.

## Supporting expect tests and adaptation

[model/dune](dune) links only Core/gpuio; [test/dune](test/dune) adds inline expect
tests. Run the focused nongraphical tests in the development profile:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/signal_studio/model -j 2
```

`let%expect_test` registers tests through ppx_jane/ppx_expect. The tests cover:

- Round-trip run42/Swift selection and interactive clamping to (634,74).
- Invalid empty/oversized/version/run/duplicate-ID/NaN/out-of-bounds/selection
  documents, printing eight true rejection results.
- Exact accepted sample/2 routing and rejection of /02/query/fragment/other route.
- All runs0–100 producing validated scenes/charts, chart encoding round trips,
  workspace round trips and invalid run/selection rejection.
- Axis-unit agreement for Swift's 200.0 ms/24.7%.

`[%expect]` records expected text/S-expressions, while assertions compare typed
values. These tests do not exercise native dragging, files, OS URL delivery or
physical charts. Commands are documented from the actual Dune stanzas; this
review does not newly execute them.

A small adaptation is another latency perturbation: change chart's pure formula,
retain stable IDs and update meaningful expectations. Adding a fifth sample also
requires updating decode's exact identity set/document version policy, scene ID
allocation and route contract; changing only create would make saved documents
inconsistent. For real data, define its units/validated ingestion separately and
keep native handles out of serialized domain values. The [README](../README.md)
and [design](../../../docs/design/signal-studio.md) link actual platform/evidence
boundaries.
