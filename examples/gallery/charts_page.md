# Charts: application data, Bonsai controls and native plotting

The Charts & data page demonstrates seven chart families, mixed area/bar/line
layers, typed categorical data, source publication, selection, and a complete
original-data table.
The Cartesian examples support vertical or horizontal categories and either
direction for the numeric value axis. This page is ordinary OCaml application
code; it does not require a Rust plotting callback.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Charts & data**, then a family or **Mixed layers**. Try **Horizontal
axes**, **Reverse value axis**, and **Update chart samples**. Use arrows/Home/End
to preview plotted values and Enter to select. **View data** opens the original
values, including any omitted from the rendered picture by explicit sampling.
Existing [categorical](../../docs/evidence/categorical-charts-och41.md),
[stacked](../../docs/evidence/stacked-charts-och41.md) and
[inspection](../../docs/evidence/chart-inspection-och41.md) evidence records specific
local macOS checks. This documentation review adds no native acceptance; full
Linux desktop qualification is deferred. No external data, assets or credentials
are needed. This page has no dedicated launch diagnostic flag; optional native
test scripts are separate from ordinary launch. See the
[development guide](../../docs/development.md) for toolchain prerequisites.

## Reading order

1. [charts_page.mli](charts_page.mli) exposes the page's component boundary.
2. [charts_page.ml](charts_page.ml) defines the model, source owner and reactive
   view. Start with `Mode`, then `Source`, then `component`.
3. [gpuio_chart_samples.ml](../charts/samples/gpuio_chart_samples.ml) constructs
   validated example datasets and translates typed selections into descriptions.
   Its [interface](../charts/samples/gpuio_chart_samples.mli) lists the families
   and sample presets; the [catalog walkthrough](../charts/samples/gpuio_chart_samples.md)
   explains their constructors and synchronous Option syntax. These are
   demonstration values, not an external feed.
4. [preview_scope.ml](preview_scope.ml) owns branch activation, asynchronous
   acquisition and cancellation. [application.ml](application.ml) owns the app
   and window, while [component.ml](component.ml) selects this page branch.

## Data and lifetime

`Mode.t` selects a named family, mixed layers, categorical data, stacked bars or
areas, the ordinal-color example, or Flow styling. Its `data` function builds immutable
`Chart_data` values with validated numeric domains, source IDs and resource
bounds. IDs identify values; labels describe them. Missing line/area values are
explicit gaps. The sample's mode and phase select new data without changing the
meaning of the library's selection IDs.

`Source.t` owns a page-visit-scoped `Gpuio_eio.Chart.t`, the selected mode, and
an update phase. Its small mutable fields belong to the OCaml UI domain.
`create` registers an initial dataset. `choose` uses `Chart.reset` when changing
family, which creates a new logical data generation. `update` uses `Chart.set`
for a new publication in the same generation. `Source.choose` resets phase to
zero; `Source.update` advances it modulo 13. Mutable fields change only after
the local operation succeeds. Native acceptance remains asynchronous, and the
[registration interface](../../lib/eio/chart.mli) documents coalescing and native
rejection rather than treating local success as publication acknowledgement.

`Preview_scope.acquire` creates the source for the mounted page and cancels its
scope when the page is left. Returning reacquires native data and clears the
selected target. Retained mode and control choices are separate from the lifetime
of the native source. The `Chart.handle` passed to the view is borrowed: keeping
a handle does not keep a cancelled registration alive.

## Bonsai and GPUIO boundaries

Bonsai owns the page's reactive choices and status. `B.toggle` supplies each
Boolean and its update effect. `B.state` supplies the notice text and setter.
The mode and selection use `B.Expert.Var` because resource operations update
them within explicit effects; they are constructed once per page graph, not on
each render. `let%arr` derives the view from current values. Initially the mode is Line,
inspection and flow presets are Default, category layout is Auto, unknown color
and stacking are enabled, horizontal/reversed/disabled are false, and selection
is absent. A `Bonsai.Effect` is work scheduled by an event, not an operation to run
while deriving a view. For example:

```ocaml
let update =
  E.bind
    (E.of_thunk (fun () -> Source.update source))
    ~f:(function
      | Ok () ->
        if Registered.is_published source.chart
        then E.Ignore
        else set_notice "Publishing updated samples…"
      | Error e -> set_notice (Error.to_string_hum e))
```

`E.of_thunk` defers the UI-domain source operation until the button runs it;
`E.bind` chooses the resulting notice effect. The outer `let%arr` also reads
setter effects so native buttons/switches can receive them. The lifecycle hook
resets the notice when the branch deactivates; `Preview_scope` handles the actual
resource cancellation.

GPUIO supplies the switches, buttons, theme styles and `V.chart`. The chart's
stable key preserves its native identity across ordinary control changes. Its
config contains the source handle, accessible label, plotting options, resolved
colors and disabled state. Bonsai does not tessellate plots or calculate native
pointer hit tests. `Disable chart input` changes `Chart.Config.create ~disabled`
without unregistering data. Public constructors are documented in
[chart.mli](../../lib/core/chart.mli),
[chart_options.mli](../../lib/core/chart_options.mli) and
[chart_style.mli](../../lib/core/chart_style.mli).

The two direction switches select `Chart_options.Orientation.Vertical`,
`Horizontal`, `Vertical_reversed` or `Horizontal_reversed`. Reversal changes the
numeric value projection: positive bars grow upward, rightward, downward or
leftward respectively. It keeps category order, raw values, IDs and axis gutters
unchanged. The setting affects Cartesian layers; pie, radar, candlestick and
Sankey keep their own family-specific layouts.

## Follow an update and a selection

Clicking Update invokes an OCaml effect, advances the sample phase, and asks the
registered source to publish the new immutable data. `is_published` distinguishes
pending publication from accepted data. Rust prepares bounded plot geometry off
the UI thread. The chart reports Ready when that preparation becomes observable
through its native view. The page displays the family, source count and direction.
Ready is not a physical-presentation or FPS measurement.

Native pointer/keyboard previews use the displayed geometry. Committing a
selection delivers a typed event with publication identity. `on_event` stores
its target in the selected variable; `Samples.describe_selection` resolves it
against the accepted dataset. The page withholds a description while a new source
publication is pending. Selecting a mark does not copy the whole dataset back
through the bridge. Opening View data is a separate native browsing operation;
it exposes original values and does not implicitly change the selected mark.

For a concrete ID trace, choose Categorical and select the first Research bar at
phase zero. The helper resolves Completed, category ID 42, value 30; the second
Research bar has a distinct category ID 99. Update publishes phase one: those
values become 31 and 46 while identity and the missing Code bar remain. While
publication is pending the page hides derived selection text; it does not
clear the selected variable on ordinary updates. Choosing another mode does
clear it and reset the source generation. Native selection reconciliation and
the matching-published-data guard determine the subsequent readout.

## Eio, performance and adaptation

This page has no application file/network I/O and starts no polling loop. The
Eio application runtime and scoped Chart API handle asynchronous registration,
publication and cleanup. Native workers own reduction, geometry and hit indexes;
native hover stays local. Application updates should publish immutable datasets
at useful boundaries, with stable IDs and deliberate sampling policies.

To replace the samples, construct `Chart_data` through its validated constructors
and publish through `Source.update` or an equivalent scoped owner. Keep `set`
for a new revision of the same logical data and `reset` for replacement identity.
Add UI choices through typed `Chart_options` and `Chart_style` values. Do not
encode missing data as NaN or infer category identity from labels.

See the [chart contract](../../docs/design/charts.md) for ownership, sampling and
selection details, and the [catalog review](../../docs/catalog/charts-review.md)
for remaining public options. `scripts/test_gallery.py --section charts` drives
native keyboard selection, source updates, original-data paging, all four
Cartesian directions, theme/size changes and page-scope cleanup. GPU direction
and gradient pixels are checked separately by `native_chart_paint`; neither test
is a complete VoiceOver or performance qualification.

## Categorical data

The Categorical mode uses the separate [sample module](../charts/samples/categorical.ml)
and its [walkthrough](../charts/samples/categorical.md). Two Research labels have
different IDs; Code has a missing bar and a present line value. `Category_layout`
in this page translates Bonsai control state into Auto/Point/Band options. Native
preparation owns their geometry. Changing layout preserves data and selection;
Update publishes new values with stable IDs. Original-data browsing distinguishes
the repeated labels by category ID and retains the missing observation.

## Stacking

The Stacked bars and Stacked areas modes share a [pure sample](../charts/samples/stacked.md)
with stable category/datum IDs, a missing Wednesday value, signed adjustments and
an independent Target line. A separate Bonsai toggle defaults to stacking; it
switches `Chart_options.Stacking.Stacked` / `Grouped` only for these modes.
The existing family and mixed examples retain their default unstacked behavior.
Raw selection, scoped publication and original-data browsing use the same paths
as the categorical example. Native tooltips distinguish the value from cumulative
bounds. The [contract](../../docs/design/stacked-charts.md) explains shared area
curves and the deliberate difference between signed accumulation and divergent
positive/negative stacking.

## Stable ordinal colors

The [Ordinal colors sample](../charts/samples/ordinal_colors.md) rotates pie-slice
source order on update, while a fixed typed domain assigns Build and Research
stable colors. The page's `unknown_color` Bonsai toggle chooses an explicit
unknown color or the ordinary position palette for Review. `Chart_style.create`
resolves the mapping; the native prepared plan shares its colors with the legend.
The existing publication guard and ID-based selection remain in force.

## Inspection presentation

The inspection state selects a pure [public preset](../charts/samples/inspection.md).
It changes `Chart_style.create ~inspection`, independently of the data mode,
source publication, selected source ID and original-data table. Native hover and
keyboard previews use the prepared style; a matching Ready notice names the
preset. Crosshair/marker layout and anchored-card placement run natively. No
Bonsai update is needed for ordinary pointer movement within a prepared chart.

## Sankey presentation

Flow styling uses the [Sankey presentation sample](../charts/samples/sankey_presentation.md):
main, tiny and zero parallel flows with separate node/edge IDs. A separate Bonsai
state selects node radius, ribbon opacity, minimum width, label gap or rich-label presets.
Flow styling and Flow labels apply these sample options; other modes retain Sankey defaults.
The usual source generation, selection and original-data paths stay in use.
Minimum width changes native paint/hit geometry without changing the value 0.01;
zero flows remain available only through original data.

`flow_style` and `set_flow_style` come from `B.state`; the preset buttons receive
the setter's effects, while `let%arr` reads the current choice. The pure sample
function `node_labels` supplies `Chart_style.create ~node_labels`, separately from
`options`, which supplies `Chart_options.create ~sankey`. Rich flow labels creates
two styled lines per node. Hide target label supplies an empty line list for the
target ID; other presets supply an empty override collection and restore source
labels. `V.chart` keeps its stable key and borrowed source handle. These changes
affect presentation only: source publication, original names and selection IDs
retain their existing ownership. The sample's walkthrough traces the actual
button effect through Bonsai and the native Ready observation.

Rich labels are presentation overrides keyed by `Chart_data.Node_id`; their
validated constructors and limits are in
[chart_node_labels.mli](../../lib/core/chart_node_labels.mli). The main view's
`Chart_style.create` receives the complete override collection, restoring empty
overrides outside the two flow presentation modes. For example, click **Rich flow labels** in that
mode: its native button runs `set_flow_style`, `let%arr` rebuilds the options/style,
and native preparation produces styled label lines while preserving original
node names in the data table. **Hide target label** submits an explicit empty
line list for the target; it does not remove the node, its flow or semantic
selection. A Ready notice names the current preset after preparation; a label
change alone does not create a new source revision.

Gradient flows and Target-colored flows use the same preset state/effect path.
`Samples.Sankey_presentation.options` supplies `Sankey.Link_color.Gradient` or
`Target`; source data and rich-label overrides remain separate. Color changes
need no dataset publication, and Ready names the selected preset. The sample
walkthrough explains endpoint colors and opacity.

## Measured outside labels

`Mode.Flow_labels` uses `Samples.Sankey_presentation.placement_data_exn`, a three-stage
source with nodes intentionally stored out of topology order. The
[sample guide](../charts/samples/sankey_presentation.md#three-column-outside-labels)
explains the source IDs, four edges and seven original-data rows.

`outside`, `narrow_flow`, `show_labels`, `long_labels` and `bold_labels` are
independent `B.toggle` state cells. `let%arr` reads them and builds GPUIO values:
placement/visibility become `Sankey` options, long/middle captions become ID-keyed
label metadata, and narrow/bold become view width and inherited font weight.
The plot's stable key and source owner stay the same. Outside and Show flow labels
start true; long, bold and narrow start false.

A toggle runs a Bonsai effect, derives new immutable options/style and triggers
native preparation. Rust captures font context for measured outside labels and
keeps matching geometry/text together; no OCaml font measurement or per-frame
callback is needed. Ready notices identify Inside/Outside but do not certify
physical presentation. Native selection/update/data browsing use the same guarded
publication path as other modes. Only choosing a different mode resets source
identity. Removing the page retires the scoped source.

`python3 scripts/test_gallery.py --section chart-labels` drives rich/long/hidden
captions, theme and inside/outside changes, 420-pixel width, bold text, keyboard
selection, publication, original-data browsing and resource cleanup. The native
chart-view harness separately reads actual text pixels and checks a multiword
caption stays on its prepared line. These checks do not establish Linux desktop,
VoiceOver or 120 FPS acceptance.


## Pie radius controls

Choose the Pie family to expose **Fixed pie radius 80** and **Per-slice pie radii**.
In `component`, `fixed_pie` and `variable_pie` each come from
`B.toggle ~default_model:false graph`. Here `B = Bonsai.Cont`: `graph` owns the
reactive state cells, and each toggle supplies a current Boolean plus an effect
that changes it. The outer `let%arr` reads both Booleans and their toggle effects,
deriving the current `V.switch` descriptions and plotting options. Passing an
effect to `~on_toggle` schedules nothing until a native switch event runs it.

`Chart_options.Pie.create ~inner_radius:0.5` starts with a donut hole equal to half
the global outer radius. Its `~radius` argument chooses
`Chart_options.Pie.Radius.Fit` or `Pixels 80.` from `fixed_pie`. Fit uses half the
smaller available plot dimension after native presentation gutters; Pixels fixes
that outer radius in logical pixels. The fractional hole is therefore 40 pixels
when the fixed radius is enabled and no slice override matches.

When `variable_pie` is true, `List.map` turns four `(id, inner, outer)` tuples into
validated `Chart_options.Pie.Slice_radii.create ~slice ~inner ~outer ()` values.
`Chart_data.Datum_id.of_int64` supplies each stable slice identity; `ok` unwraps
these known-valid fixture results. The [sample source](../charts/samples/gpuio_chart_samples.ml)
assigns ID 1 to Reasoning, 2 to Code, 3 to Research and 4 to Other. Their inner/outer
pairs are respectively `(20, 90)`, `(35, 65)`, `(15, 75)` and `(0, 50)` logical
pixels. An override replaces **both** radii for its ID, rather than multiplying
the global radius or reusing the 0.5 hole fraction. All four fixture IDs match,
so enabling the fixed-radius switch while per-slice radii are enabled does not
change these four wedges. Disabling per-slice radii submits `[]` and restores the
global Fit/80-pixel radius and fractional hole.

Trace: toggle Per-slice pie radii → native switch executes `toggle_variable_pie`
→ Bonsai updates its Boolean → `let%arr` derives the ID-keyed options and chart
config → native preparation updates wedge, caption and hit geometry → Ready
updates the notice to name the radius settings. The chart keeps its stable key
and borrowed source handle. This path does not call `Source.update` or change
slice weights, angular shares, selected IDs or original-data values. The ordinary
selection handler and matching-publication guard continue to use source data.
Radius associations follow IDs across reordering or renaming, not caption text
or a slice's current position. The options are constructed with the other chart
options; these two switches are displayed only in the Pie family.

The [public interface](../../lib/core/chart_options.mli) and
[pie-radii contract](../../docs/design/pie-radii.md) require finite global Pixels
in `(0, 32768]`, fractional `inner_radius` in `[0, 0.95]`, and finite override
pixels with `0 <= inner <= outer <= 32768`. At most 256 distinct override IDs are
allowed. Unknown IDs are retained but ignored; unmatched slices use the global
settings. Equal radii, including `(0, 0)`, hide a wedge and its caption while a
positive source weight still reserves its angular interval and original-data
selection value. A zero source weight stays invisible even with positive radii.
Oversized radii may clip to the plot rather than moving its center.

To adapt the example, change Research's ID-3 tuple to `(25, 85)` while preserving
its source ID; the next derived options change that ring only. To demonstrate a
hidden positive-weight sector, use equal radii for ID 3 and keep its source value
18: the original-data browser still exposes that value, but the absent wedge is
not a hit or keyboard-preview target. Native selection highlighting requires a
prepared mark, just as it does for zero-weight slices. Handle constructor errors
when radii come from user input instead of using the fixture's `ok` shortcut.
This documentation update records the implementation; it adds no native, GPU,
accessibility or platform acceptance evidence.

## Pie captions and leader lines

The Pie family also exposes **Outside pie labels**, **Pie label gap 32**,
**Custom pie captions** and **Show pie labels**. Their `B.toggle` cells are
`outside_pie`, `spaced_pie`, `custom_pie_labels` and `show_pie_labels`; the first
three start false and the last starts true. These are independent reactive
Booleans in the page graph. Their toggle effects run on native switch events;
the outer `let%arr` then derives plotting options and resolved style from the
current values and palette. It does not measure text in OCaml or modify the data.

`Chart_options.Pie.create` receives `~labels:show_pie_labels`,
`~label_placement:(if outside_pie then Outside else Inside)` and
`~label_gap:(if spaced_pie then 32. else 15.)`. `Label_placement` belongs to
`Chart_options.Pie`. Inside is the default; gap is a finite logical-pixel distance
in `[0, 64]` and affects Outside placement only. Thus the gap switch changes no
inside-caption spacing. Hiding labels suppresses captions and leaders while
leaving the wedges and original data intact.

`Chart_style.create ~pie_labels` receives `Chart_pie_labels.empty` until custom
captions are enabled. Then `List.map` builds entries with
`Chart_pie_labels.Entry.create ~slice ~text ~line_color ()`, using validated
`Chart_data.Datum_id.of_int64` IDs, and `Chart_pie_labels.create` validates their
collection. ID 1 becomes Agent reasoning; ID 2 becomes Code generation; ID 3
gets a deliberately long Research caption. ID 4 has explicit empty text, hiding
Other's caption **and** leader without removing its wedge. ID 99 is absent from
this fixture: its entry is retained but ignored until such a source ID exists.
Omitting `~text` would inherit the source caption; it differs from passing `""`.

The global `~pie_label_line_color:(Palette.muted p)` supplies the ordinary leader
color. Custom entries override it: ID 1 uses orange `Color.rgb_exn 0xfb923c`,
while the others use `Palette.accent p`. These colors affect Outside leaders;
`~label_color:(Palette.foreground p)` supplies caption color independently.
Style construction resolves colors for the current theme. The switch handlers
carry effects, not paint callbacks; theme changes rebuild the resolved style.

Trace: enable Outside pie labels → native switch runs `toggle_outside_pie` →
Bonsai changes the Boolean → `let%arr` builds Outside options and the current
caption/color collection → native preparation measures captions and prepares
leaders with the same source/config/font snapshot → the chart delivers Ready.
The current Ready notice names radius settings and the inspection preset; it
does not spell out these label switches. Enable Custom pie captions through the
same path to replace caption text and colors. No `Source.update` is needed:
source names, weights, angular shares, original-data rows, legend names and
selection values remain independent. IDs bind presentation across reorder or
rename; repeated text cannot merge slice identity. Leaders are decorative,
not additional selection targets, and use each wedge's actual outer radius.

The [typed caption interface](../../lib/core/chart_pie_labels.mli) permits at
most 256 UTF-8 bytes per single-line caption, without ASCII controls, at most
256 unique IDs and 32 KiB of total override text. The
[style interface](../../lib/core/chart_style.mli) describes color precedence;
the [pie-label contract](../../docs/design/pie-labels.md) describes native layout.
Outside captions for sweeps below half a degree are omitted. Dense sides retain
a deterministic subset when 18-pixel rows cannot all fit; plots shorter than a
row omit outside captions. Widths are bounded to a quarter of the plot and
ellipsize. Fit reserves measured margins; literal oversized radii can clip or
enter label space, so arbitrary radii do not guarantee separation between wedges
and captions.
Zero-weight and equal-radius slices still have no wedge or caption.

To adapt the sample, replace ID 3's text with a short application caption while
keeping the source ID. To change only its leader color and inherit Research,
omit `~text` for that entry and supply `~line_color`; keep the original source
label untouched. Validate user-supplied text and collection errors instead of
unwrapping them with the fixture's `ok`. The separate
[qualification record](../../docs/evidence/pie-labels-och41.md) states tested
behavior and platform limits; this walkthrough itself is not acceptance evidence.

## Radar projection controls

The Radar family adds a small `Radar_scale` model: its values are the public
`Chart_options.Radar.Scale` variants. `B.state` retains the selected mode; two
`B.toggle` values retain fixed radius and extra label spacing. Their effects only
change options in the `let%arr` view; they do not republish or mutate the dataset.

Choose **Shared data maximum** to normalize both series by their largest source
value (94 in this fixture). **Shared maximum 50** deliberately puts larger values
outside the grid. **Fixed radar radius 80** makes the ring 80 logical pixels;
**Radar label gap 24** moves labels outward and reserves gutters. The native worker
prepares geometry for the new options and returns Ready before the status reports
the selected settings. Home/Enter still selects Atlas / Quality / 88 out of its
source maximum 100; scaling does not rewrite its accessible original data.

To adapt this example, change the explicit maximum in `Radar_scale.all` and its
label together, or pass a different `Radius.Pixels` value in `component`. Keep
values inside the public option bounds; extreme data/scale combinations may
report Render_limit instead of drawing a distorted clamped polygon. See the
[radar contract](../../docs/design/radar-presentation.md). Default Per_axis/Fit/gap 0
preserves previous behavior.

### Ordinary Views as radar labels

**Custom radar labels** replaces Quality with a real button, Cost with a
two-line column, and Context with an editable note. The remaining axes keep
their dataset captions. The pure `radar_labels` helper builds this collection
from the current palette, counter, effect and editor controller. It returns an
empty collection when custom labels are disabled. The helper uses
`Chart_radar_labels.Entry.create ~axis` to associate each ordinary `V.t` with a
positive `Chart_data.Datum_id`, validates the entries with
`Chart_radar_labels.create`, and passes the collection as `V.chart ~radar_labels`.
The IDs 1, 3 and 4 come from the sample dataset; neither list order nor caption text
determines which native axis receives the content.

The `rich_radar` and `show_radar_labels` toggles are Bonsai state. `label_activations`
is a separate `B.state 0` model. Pressing the Quality button runs the effect from
`set_label_activations`; the next `let%arr` result contains updated button text and
the activation readout. That is an ordinary Bonsai state/view update, without a
chart-data publication or an OCaml measurement callback. `accessible_name` keeps
the button's name stable while its visible counter changes.

`Gpuio_eio.Text_input.create window ... graph` constructs the editor's Bonsai
controller once in the page graph. `B.return` lifts its constant validated
`Text_input.Config` into a Bonsai value; `Single_line` selects the editing mode,
`label` supplies the accessible name, and `placeholder` supplies the empty-field
hint. The outer `let%arr` reads the controller and passes it to the pure helper.
`Gpuio_eio.Text_input.view editor` then places its one native editor in axis 4.
The controller and the placement are separate: do not call `create` in the pure
view helper or place the same controller twice.

Typing goes to Rust's native editing session. Its text, selection and undo
history are not reconstructed from the button counter or the chart dataset.
The controller receives asynchronous observations; this example does not mirror
or replace the draft on every observation. The input has explicit dimensions
while the enclosing axis wrapper is measured naturally. See the
[editor controller interface](../../lib/eio/text_input.mli) for snapshot and
revision-checked command APIs when adapting this field to application data.

Rust lays out these submitted Views at natural size during native prepaint, then
positions them at their axis anchors. The button uses ordinary padding, border,
radius and theme colors. The Cost column composes two `Palette.text` Views. They
retain their own styles instead of receiving `Chart_style.label_color`. Large
content can overlap or clip: use normal View sizing and the chart's radius/gap
controls when adapting the layout.

Changing the application theme updates the palette values used by these ordinary
Views. The axis keys, page-owned Bonsai counter and editor controller stay the
same, so restyling does not replace the native editor or discard its draft.
The public radar walkthrough switches themes in both directions with this field
focused and checks its draft, focus and the counter afterward.

**Show radar labels** changes `Chart_options.Radar.labels`, keeping the View
entries mounted but hiding their native content. The original-data browser also
hides them. Closing the browser or showing labels restores the content; neither
operation resets the Bonsai counter or destroys the retained editor draft. **Custom radar labels** off removes the
entries and unmounts their native children, while the page-owned counter still
exists. The native editor draft, selection and undo history are destroyed on
unmount; enabling custom labels again creates a fresh empty editor. To retain
text across unmount, explicitly maintain an application-owned draft and supply
a validated creation seed through the editor API. To reset that model on removal too, change its owning Bonsai branch/key
deliberately. **Disable chart input** also disables the label button through its
chart ancestor, even though the button itself does not request a disabled state.

To add another composed label, create an entry for the corresponding stable axis
ID. The collection permits at most 64 distinct IDs; an ID absent from the current
dataset is retained but hidden. Original-data names, values and selection remain
unchanged. See the [content contract](../../docs/design/radar-label-content.md)
for the native ownership and qualification requirements.
