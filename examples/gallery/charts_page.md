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
macOS has local native acceptance; Linux desktop qualification is deferred.

## Reading order

1. [charts_page.mli](charts_page.mli) exposes the page's component boundary.
2. [charts_page.ml](charts_page.ml) defines the model, source owner and reactive
   view. Start with `Mode`, then `Source`, then `component`.
3. [gpuio_chart_samples.ml](../charts/samples/gpuio_chart_samples.ml) constructs
   validated example datasets and translates typed selections into descriptions.
   Its [interface](../charts/samples/gpuio_chart_samples.mli) lists the families
   and sample presets. These are demonstration values, not an external feed.
4. [preview_scope.ml](preview_scope.ml) owns branch activation, asynchronous
   acquisition and cancellation. [application.ml](application.ml) owns the app
   and window, while [component.ml](component.ml) selects this page branch.

## Data and lifetime

`Mode.t` is either a named family or Mixed. Its `data` function builds immutable
`Chart_data` values with validated numeric domains, source IDs and resource
bounds. IDs identify values; labels describe them. Missing line/area values are
explicit gaps. The sample's mode and phase select new data without changing the
meaning of the library's selection IDs.

`Source.t` owns an application-scoped `Gpuio_eio.Chart.t`, the selected mode, and
an update phase. Its small mutable fields belong to the OCaml UI domain.
`create` registers an initial dataset. `choose` uses `Chart.reset` when changing
family, which creates a new logical data generation. `update` uses `Chart.set`
for a new publication in the same generation. Both operations return errors
instead of silently replacing accepted data on failure.

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
each render. `let%arr` derives the view from current values. The lifecycle hook
resets the notice when the branch deactivates; `Preview_scope` handles the actual
resource cancellation.

GPUIO supplies the switches, buttons, theme styles and `V.chart`. The chart's
stable key preserves its native identity across ordinary control changes. Its
config contains the source handle, accessible label, plotting options, resolved
colors and disabled state. Bonsai does not tessellate plots or calculate native
pointer hit tests.

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
