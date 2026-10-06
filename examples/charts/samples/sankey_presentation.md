# Sankey appearance and tiny flows

[sankey_presentation.ml](sankey_presentation.ml) is a pure OCaml example with a
small [public interface](sankey_presentation.mli). It demonstrates rounded nodes,
transparent ribbons, visible tiny flows, label spacing and styled multiline labels.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Charts & data → Flow styling**. Switch among Default flows, Rounded nodes,
Muted ribbons, Visible small flows, Spaced flow labels, Rich flow labels and Hide
target label. Home/arrows preview a
flow; Enter selects it. Update chart samples increments the main flow. View data
includes both nodes and all three edges, including the zero-valued edge. Local
native checks target macOS; Linux desktop qualification remains deferred.

Read `type t`, `all`, `label`, `options` and `node_labels`, then `data_exn`.
The variant `t` enumerates the choices; `all` determines button order and `label`
supplies their captions. `[@@deriving equal]` generates the typed equality function
the page uses to highlight the selected button. Each preset returns an
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

`node_labels` is the second pure preset function. `Chart_node_labels.Line.create`
builds one validated caption with optional font/color; `Node.create ~node` binds
those lines to a typed node ID, and the outer `create` checks uniqueness and the
collection's total text budget. Rich labels supplies two differently styled
lines for nodes 10 and 20. Hide target returns an empty line list for node 20;
the other presets return an empty override collection, restoring original labels.
An empty collection and an entry with no lines therefore mean different things.

The page passes this value to `Chart_style.create ~node_labels`. This is GPUIO
presentation configuration, while Bonsai's `B.state` owns which preset is chosen.
`let%arr` reads that changing preset and derives a new immutable chart configuration;
it does not call Rust paint or start an I/O task. A preset button delivers an effect
that updates the state, then native preparation produces the styled labels.
Native block clipping and font/color application happen after that boundary.

For a concrete trace, find `component` in `charts_page.ml` and its
`let flow_style, set_flow_style = B.state ... graph` binding. Bonsai constructs
a persistent state cell, initially `Default`, in this component's reactive graph.
The returned values are reactive inputs, not a mutable OCaml reference. The
page's `let%arr ... and flow_style = flow_style and set_flow_style = set_flow_style`
reads their current values and derives the view when its dependencies change.
`B.Let_syntax` makes this PPX syntax available; ordinary OCaml expressions inside
the body construct GPUIO values.

Click **Rich flow labels**. `Palette.button` has received
`set_flow_style candidate`, a Bonsai effect describing a future state change;
constructing the button does not execute the effect. GPUIO delivers the native
activation asynchronously, the runtime runs that effect, and Bonsai updates
`flow_style`. The derived view now calls `options Rich_labels` and
`node_labels Rich_labels`, feeds them to `Chart_options.create` and
`Chart_style.create`, and passes the resulting `Chart.Config` to `V.chart`.
The unchanged `gallery-chart` key preserves the native chart's identity.
Rust prepares the new presentation; the page's `on_event` handles the later
`Ready` observation by updating the notice. No `Source.update` call is involved
in this preset change. Clicking **Update chart samples** follows that separate
data-publication path instead.

The captions **Intake / Recorded activity** and **Outcome / Delivered locally**
are static presentation text. The source still names the nodes Input and Output;
selection and View data retain those names and raw values. To add a value caption,
compute its string from the application's model and update it intentionally.
Changing the dataset alone does not evaluate a formatting callback in the label.
See the [rich label contract](../../../docs/design/chart-node-labels.md) for bounds,
unknown IDs, native clipping and matching-package requirements.

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

To customize the headline, edit `line ... "Intake"` in `node_labels`; retain
node ID `10L` so it still targets Input. For user-provided text, handle the
constructors' `Or_error.t` results explicitly instead of copying this sample's
`ok = Or_error.ok_exn` shortcut for known constants. The public
[label interface](../../../lib/core/chart_node_labels.mli) and
[style interface](../../../lib/core/chart_style.mli) describe validation and theme
resolution. This example's hard-coded colors demonstrate overrides; a real app
can use its theme colors when deriving the style.
