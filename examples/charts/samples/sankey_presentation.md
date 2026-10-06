# Sankey appearance and tiny flows

[sankey_presentation.ml](sankey_presentation.ml) is a pure OCaml example with a
small [public interface](sankey_presentation.mli). It demonstrates rounded nodes,
transparent ribbons, visible tiny flows and label spacing.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Charts & data → Flow styling**. Switch among Default flows, Rounded nodes,
Muted ribbons, Visible small flows and Spaced flow labels. Home/arrows preview a
flow; Enter selects it. Update chart samples increments the main flow. View data
includes both nodes and all three edges, including the zero-valued edge. Local
native checks target macOS; Linux desktop qualification remains deferred.

Read `type t`, `label` and `options`, then `data_exn`. Each preset returns an
immutable `Chart_options.Sankey.t`. The dataset has separate typed node/edge IDs,
two nodes, and three parallel flows: a main flow of 100 plus the update phase, a
tiny flow of 0.01, and a zero flow. Distinct IDs preserve the edges even though
they share endpoints. `data_exn` validates the demo phase before constructing data;
it is an explicitly raising convenience for known demonstration inputs.

The [gallery page](../../gallery/charts_page.md) owns Bonsai state for the preset.
A button's effect updates that state; `let%arr` rebuilds the chart configuration
with `Chart_options.create ~sankey`. The Flow styling mode uses these options,
while ordinary chart modes keep their defaults. Changing a preset does not publish
new source data. Native preparation produces matching paint/hit geometry and
reports Ready; that report is not a physical-presentation timestamp.

Selection takes the other direction: native hit testing or keyboard navigation
finds an original source ID, then the committed selection event updates the page's
OCaml state. Increasing minimum width changes visible geometry while the selected
value and original-data table remain 0.01. Zero flows are not invented for display.
Opacity multiplies existing alpha and does not disable hit testing.

This module creates no Bonsai graph, native handle or I/O task. The page's scoped
Eio Chart owner registers/publishes the data and retires resources on page exit.
The sample's pure configuration must not become a per-frame callback. The
[page walkthrough](../../gallery/charts_page.md) explains model/effect/view and
source-scope lifetimes in detail.

To emphasize small flows, change the Minimum branch's `min_link_width`, within
0–64 logical pixels. Check overlaps and the original-data view; wider ribbons
are no longer proportional everywhere. To use softer nodes, change Rounded's
radius within 0–32; native rendering clamps it to the actual node dimensions.
See the [contract](../../../docs/design/sankey-presentation.md) for bounds,
clipping and remaining label/gradient limitations.
