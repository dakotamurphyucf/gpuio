# Chart and plot source review

Reviewed 2026-10-05 against the pinned GPUI Kit revision
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Inputs are the manifest-pinned
`component-chart-*` and `component-plot-*` snapshots, including nested scales,
shapes, axes, labels, grid and tooltip helpers. Documentation snapshots do not
change the compiled GPUI or extracted plot dependency pins.

All seven named chart families have public GPUIO implementations. This does not
mean every low-level plotting closure, scale or styling option is exposed.
[`Chart_data`](../../lib/core/chart_data.mli),
[`Chart_options`](../../lib/core/chart_options.mli),
[`Chart_style`](../../lib/core/chart_style.mli) and
[`Chart_sampling`](../../lib/core/chart_sampling.mli) define the actual boundary.
The [accepted chart design](../design/charts.md) supplies native ownership,
validation and resource bounds; OCH-41 must not infer parity solely from names.

## Families and data semantics

| Pinned family | Existing public behavior | Difference to retain in the ledger |
| --- | --- | --- |
| Line | Named ID-stable series, explicit gaps, Linear/Natural/Step_after curves, dots, axes/grid, legend, tooltip, native selection | Numeric series retain linear x spacing; explicit typed categorical series now support native point/band projection. Categorical series are aligned to their declared domain with explicit missing values. |
| Area | Same typed series and curves; zero-baseline or explicitly stacked fill, negative values, alpha and mixed layers | Arbitrary lower/upper area accessors from the low-level shape API are not exposed. [Stacking](../design/stacked-charts.md) uses native cumulative bounds and shared sampling/curves; [local qualification](../evidence/stacked-charts-och41.md) passes; wider catalog/release acceptance remains open. |
| Bar | Grouped or explicitly stacked series, four Cartesian value directions, numeric or categorical x, negative values, width fraction, corner radius and two-color value-axis gradient | [Reversed directions](../design/chart-directions.md) now support positive bars growing from top/right as well as bottom/left; category order and existing axis gutters remain unchanged. Categorical bands now have native preparation and source-provenance support. Independent corners, axis-side customization and arbitrary per-datum/chart-aware backgrounds remain broader than these options. |
| Pie | Named ID-stable nonnegative slices, donut-hole fraction, padding, palette, labels and original-value selection | Per-slice inner/outer radius functions, leader-line colors and custom label gaps are not exposed. Outer radius is fitted natively; data cannot encode a rose chart merely by changing values. |
| Radar | Named axes with individual positive maxima, multiple series, grid levels, dots, labels and alpha fill | [Radar projection](../design/radar-presentation.md) now exposes shared-data/explicit maxima, fixed radius and label gap, preserving per-axis defaults, with [paired/worker/root/installed evidence](../evidence/radar-projection-och41.md). Explicit maxima extrapolate; extreme geometry fails under the native coordinate budget. Rich `RadarLabel` presentation remains unfinished. |
| Candlestick | Increasing numeric x, validated OHLC, body width, axes/grid and exact/OHLC reduction | Native hollow rising/filled falling/equal-price marks preserve meaning without color alone. Source point-spacing/tick-margin behavior is not claimed identical. |
| Sankey | Bounded acyclic graph, stable node/edge IDs, parallel edges and zero/isolated values, four alignments, relaxation, node width/padding, linear/sqrt weights | Stable per-node colors use the ordinal mapping. [Sankey presentation](../design/sankey-presentation.md) now exposes node corners, link opacity/minimum thickness and label gap; [local evidence](../evidence/sankey-presentation-och41.md) records its scope. [ID-keyed multiline labels](../design/chart-node-labels.md) now expose per-line font/color and explicit hiding, with [scoped local evidence](../evidence/chart-node-labels-och41.md). [Ribbon color policies](../design/sankey-link-colors.md) now expose Source/Target/Gradient with [local unit/GPU/root/installed evidence](../evidence/sankey-link-colors-och41.md). [Outside placement](../design/sankey-label-placement.md) now reserves measured side/above-middle margins through the public API, with [local native pixels/root/installed evidence](../evidence/sankey-label-gallery.md). Captions are single lines within each rich block and ellipsize under bounded margins; arbitrary dense-graph nonoverlap is not promised. Sqrt affects geometry while raw values remain available. |

Applications may precompute serializable data/labels in OCaml before publication;
they do not supply Rust accessors that call OCaml during layout/paint. Data IDs
are distinct from labels and source positions. Missing y values are explicit
gaps, not NaNs. Invalid domains are rejected rather than sorted, normalized or
silently repaired. Default series/slice/node palette cycling remains order-based. The optional
[ordinal mapping](../design/chart-ordinal-colors.md) now assigns stable colors to
explicit namespaced IDs, with a cyclic range and explicit unknown-color policy.
It covers series, pie slices, Sankey nodes/source ribbons and candle movement;
per-datum colors within a Cartesian series remain separate presentation work.
[Local paired-codec, native/GPU and installed-gallery qualification](../evidence/chart-ordinal-colors-och41.md)
passes without establishing whole-catalog or release acceptance.

[Categorical data and layouts](../design/categorical-charts.md) now use explicit
typed IDs, domain order and labels. Point/band spacing is native, and sampled marks
retain original source ranges. Equal labels do not merge IDs. Missing observations
remain in the original-data table. This is an aligned dataset interface, not an
arbitrary sparse accessor API. [Evidence](../evidence/categorical-charts-och41.md)
records scoped qualification and remaining limits.

## Plot helpers and styling

| Nested source surface | Owner / status |
| --- | --- |
| `Plot`, `IntoPlot`, `paint`, `tooltip_state` and `tooltip` | Native retained preparation, painter, hit index and tooltip own these operations. `View.chart` receives an application-scoped resource and validated options; it does not expose synchronous OCaml paint/tooltip closures. Custom native plots belong to the static extension SDK. |
| `ScaleLinear`, `ScalePoint`, `ScaleBand`, `ScaleOrdinal` and sealed scale trait | Numeric linear domain handling is native in the existing chart preparation. Typed categorical datasets now have native Point/Band/Auto layout options. IDs are explicitly unique; the standard band formula differs from the source's 30-pixel cap/padding formula. Typed `Chart_style.Ordinal` now provides an explicit bounded domain/range, cyclic lookup and optional unknown color, resolved once in native preparation. Duplicate keys and empty ranges are rejected deliberately; generic plotting scales remain outside this API. |
| `PlotAxis`, `AxisText`, `Grid`, label measurement/truncation | Public Axes selects x/y/grid, bounded tick count and typed number formats; style supplies axis/grid/label colors and the view font. Arbitrary explicit tick positions, label sides/alignment, grid dash arrays and per-label font styles are not exposed by these options. Text measurement helpers map to native rendering ownership, not OCaml layout callbacks. |
| Arc, pie, line, area, bar and radial-line shapes | Prepared family geometry supplies the existing widgets. Lower-level arbitrary angle ranges, per-datum baselines, independent marker fill/stroke and path-specific style builders are not a public general-purpose plot API. Canvas offers separate retained drawing; it does not grant chart selection/data-table semantics automatically. |
| `Stack` | Source computes cumulative lower/upper values, substituting zero for missing values. Typed opt-in Stacked now implements natural signed accumulation, bars after explicit aggregation and areas with matching cumulative curves. Raw source IDs/values remain intact. Numeric layers must align within each kind; own missing observations retain gaps. [Local native, GPU and installed-gallery qualification](../evidence/stacked-charts-och41.md) passes; no whole-catalog acceptance is implied. |
| Sankey topology/layout and ribbon path helpers | Bounded native preparation owns graph validation/layout and retained geometry. [Dependency provenance](../../third_party/sources.json) and reconstruction records govern compiled source; raw catalog snapshots are evidence only. |
| Tooltip state, title/rows, cross lines, dots and appearance | [`Chart_inspection`](../../lib/core/chart_inspection.mli) now supplies card visibility/title/values, bounded anchor/corner placement and appearance; full-plot crosshair axes, dashed/solid bands and color; independent marker size/fill/stroke/status. Hover/drag previews stay native and OCaml observes committed selection. [Scoped qualification](../evidence/chart-inspection-och41.md) covers actual pixels and public gallery behavior. Arbitrary rich rows, cursor-following cards, custom partial guide spans and per-datum annotations remain separate public-surface work. |

These missing public options remain explicit catalog work/decisions; this review
does not create a new post-v1 deferral. Any extension must use bounded serialized
values/native preparation and preserve accessibility/provenance, rather than
introducing per-frame OCaml callbacks or weakening data validation.

## Interaction, accessibility and resource evidence

[`Chart.Config`](../../lib/core/chart.mli) supports native pointer/drag selection,
arrow/Home/End previews, Enter/Space commit, Escape cancellation/clear, disabled
policy and an original-data companion. The native View data control/D key opens
a paged keyboard-accessible table of every original value, including gaps,
zero-area marks and values omitted by explicit sampling. Browsing is separate
from chart selection. Numeric series identifiers supplement colors. None of this
is a complete VoiceOver audit; plotted pixel positions and AX roles alone do not
prove spoken navigation.

The dataset registration outlives mounted views and is scoped to one application.
Unmount releases view work/readers; releasing the registration retires its data.
Ordinary publication retains the last prepared picture until replacement is
ready; reset/source change clears it. Generation/revision guards protect updates,
selection, workers and cancellation. Line envelopes, bar sum/mean and candle OHLC
reduction are explicit; source IDs/ranges and raw data remain available. Geometry
admission may reject a render rather than secretly changing sampling or bridging
gaps to fit a quota. Preparation metrics are not RSS or frame latency.

Evidence entry points:

- [Chart evidence](../evidence/charts-och40.md): all seven native GPU families,
  style/interaction, original-data access, worker/view/resource lifecycle and
  bounded admission. Historical checkpoints remain scoped to their revisions.
- [Streaming measurements](../evidence/chart-streaming-och40.md): public upload,
  publication/preparation/render stages, exact versus sampled data and cleanup.
  The 2026-10-04 follow-up already investigates the historical 63,050.99 ms outlier:
  it did not recur in an 80-publication visible run; controlled minimization caused
  a roughly three-second preparation/readiness delay. That demonstrates a timing
  confound, **not** the cause of the old sample. Do not repeat this investigation
  merely because an earlier checkpoint calls it pending.
- `examples/charts`, `examples/chart_stream`, the Charts gallery and
  [Signal Studio](../evidence/signal-studio-och29.md) provide public consumers.
  New rendering features still need their own gallery and actual native evidence.

`Ready` means preparation became observable through the native view, not physical
presentation or render-independent worker completion. Current optimized
performance qualification, resource consolidation, VoiceOver and release gates
remain OCH-17. Linux build/unit/consumer results and deferred Linux desktop
qualification (OCH-47) must remain separately identified.
