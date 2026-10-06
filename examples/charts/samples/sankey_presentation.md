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
target label, Gradient flows and Target-colored flows. Home/arrows preview a
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

`Gradient` and `Target` are two additional `t` constructors in the chooser. Their
`options` branches call `Chart_options.Sankey.create ~link_color:Gradient` and
`~link_color:Target`; other presets retain the Source default. This is a typed
choice, not a custom painter. The existing button → `set_flow_style` → `let%arr`
path submits the changed options while the source handle and IDs remain stable.
Native preparation resolves each endpoint's palette/ordinal color and applies
`link_opacity` to both. A gradient spans each whole ribbon, including all its
tessellated triangles.

Try Gradient flows, select the main flow and update its value: the fill blends
from Input's purple to Output's teal, while the selected raw value moves from
100 to 101. Target-colored flows fills the ribbon using Output's color. The
source-node legend still explains each node's color. To combine the gradient
with quieter ribbons, add `~link_opacity:0.25` to that preset's constructor.
See [ribbon color policies](../../../docs/design/sankey-link-colors.md) for the
wire compatibility, opacity and ownership contract.

## Three-column outside labels

Choose **Flow labels** for `placement_data_exn`. It validates the same phase range
and creates Input → Processing → Output, with the main/tiny/zero incoming edges
and one outgoing edge equal to their sum. The node array deliberately stores
Processing, Output, Input; layout follows typed topology, not array position.
Four edges and three nodes remain available in the seven-row original-data view.

`options ?label_placement ?labels` lets the caller select
`Chart_options.Sankey.Label_placement.Outside` or `Inside`, and independently hide
all captions. Defaults remain Inside and visible. `node_labels ~middle:true`
adds the Processing override; `~long:true` adds a deliberately wide Unicode
headline. An explicitly hidden target remains hidden even with long captions.
These are pure configuration arguments, not callbacks into the native renderer.

The page's five Bonsai toggles control placement, long captions, font weight,
plot width and global visibility. It passes placement/visibility to `options`,
rich text to `node_labels`, and width/font weight to `Style.create_exn`. Outside
starts enabled in this mode; the other four switches start false except Show
flow labels, which starts true. Switching back to Flow styling uses its original
inside layout and full width.

Native preparation captures the current font and measures caption lines before
reserving bounded margins. First-column labels go left, last-column labels right,
and middle labels above their node. Each explicit rich-text line stays one line;
long content ellipsizes within its width while accessible text remains complete.
A font/width change requests new native preparation without republishing the
source. Application code does not measure fonts or run a per-frame layout effect.
See the [placement contract](../../../docs/design/sankey-label-placement.md).

Try **Rich flow labels**, then **Outside flow labels** and **Narrow flow plot**.
The first control's effect changes the preset; subsequent toggle effects change
reactive option/style values. Bonsai derives the chart configuration and native
preparation produces the new layout. Selection still reports Input → Processing
and the original value; Update changes 100 to 101 independently of captions.
To adapt this to four stages, add typed nodes/edges and corresponding overrides,
keeping distinct IDs and balanced values. Do not rely on source array order or
assume arbitrary dense graphs guarantee nonoverlapping captions.
