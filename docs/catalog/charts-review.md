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
| Line | Named ID-stable series, explicit gaps, Linear/Natural/Step_after curves, dots, axes/grid, legend, tooltip, native selection | Source uses evenly spaced `ScalePoint` x categories; GPUIO uses strictly increasing numeric x with linear spacing. Source point labels are not an implicit categorical axis API. |
| Area | Same typed series and curves; zero-baseline fill, negative values, alpha and mixed layers | Arbitrary lower/upper area accessors from the low-level shape API are not exposed; the public area baseline remains zero. |
| Bar | Grouped series, four Cartesian value directions, numeric x, negative values, width fraction, corner radius and two-color value-axis gradient | [Reversed directions](../design/chart-directions.md) now support positive bars growing from top/right as well as bottom/left; category order and existing axis gutters remain unchanged. Source categorical bands, independent corners, axis-side customization and arbitrary per-datum/chart-aware backgrounds remain broader than these options. |
| Pie | Named ID-stable nonnegative slices, donut-hole fraction, padding, palette, labels and original-value selection | Per-slice inner/outer radius functions, leader-line colors and custom label gaps are not exposed. Outer radius is fitted natively; data cannot encode a rose chart merely by changing values. |
| Radar | Named axes with individual positive maxima, multiple series, grid levels, dots, labels and alpha fill | Source global maximum/outer-radius/label-gap options and rich `RadarLabel` presentation are not identical to the typed per-axis domain and native labels. |
| Candlestick | Increasing numeric x, validated OHLC, body width, axes/grid and exact/OHLC reduction | Native hollow rising/filled falling/equal-price marks preserve meaning without color alone. Source point-spacing/tick-margin behavior is not claimed identical. |
| Sankey | Bounded acyclic graph, stable node/edge IDs, parallel edges and zero/isolated values, four alignments, relaxation, node width/padding, linear/sqrt weights | Source per-node colors, custom multi-line labels with per-line font/color, node corners, link opacity/minimum thickness and label gap are not all public options. Sqrt affects geometry while raw values remain available. |

Applications may precompute serializable data/labels in OCaml before publication;
they do not supply Rust accessors that call OCaml during layout/paint. Data IDs
are distinct from labels and source positions. Missing y values are explicit
gaps, not NaNs. Invalid domains are rejected rather than sorted, normalized or
silently repaired. Series/slice/node palette cycling is order-based, not an
implicit color map keyed by datum ID.

Categorical data can be assigned numeric positions by an application, but doing
so changes the data model and does not install category axis labels. This is an
important remaining surface difference, not complete categorical-scale parity.
The [categorical implementation draft](../design/categorical-charts.md) specifies
typed identity, point/band placement, sampling and original-data requirements;
it is planning evidence, not an implemented capability.

## Plot helpers and styling

| Nested source surface | Owner / status |
| --- | --- |
| `Plot`, `IntoPlot`, `paint`, `tooltip_state` and `tooltip` | Native retained preparation, painter, hit index and tooltip own these operations. `View.chart` receives an application-scoped resource and validated options; it does not expose synchronous OCaml paint/tooltip closures. Custom native plots belong to the static extension SDK. |
| `ScaleLinear`, `ScalePoint`, `ScaleBand`, `ScaleOrdinal` and sealed scale trait | Numeric linear domain handling is native in the existing chart preparation. Point/band/ordinal scales are not standalone public chart APIs. The source's band deduplication/padding and ordinal unknown-value fallback are not inferred from numeric plots. |
| `PlotAxis`, `AxisText`, `Grid`, label measurement/truncation | Public Axes selects x/y/grid, bounded tick count and typed number formats; style supplies axis/grid/label colors and the view font. Arbitrary explicit tick positions, label sides/alignment, grid dash arrays and per-label font styles are not exposed by these options. Text measurement helpers map to native rendering ownership, not OCaml layout callbacks. |
| Arc, pie, line, area, bar and radial-line shapes | Prepared family geometry supplies the existing widgets. Lower-level arbitrary angle ranges, per-datum baselines, independent marker fill/stroke and path-specific style builders are not a public general-purpose plot API. Canvas offers separate retained drawing; it does not grant chart selection/data-table semantics automatically. |
| `Stack` | Source computes cumulative lower/upper values, substituting zero for missing values. GPUIO bars are explicitly grouped and areas use zero baseline; neither is a stacked-series renderer. Do not relabel mixed layers or ordinary preprocessing as complete stacked plotting support. |
| Sankey topology/layout and ribbon path helpers | Bounded native preparation owns graph validation/layout and retained geometry. [Dependency provenance](../../third_party/sources.json) and reconstruction records govern compiled source; raw catalog snapshots are evidence only. |
| Tooltip state, title/rows, cross lines, dots and appearance | Existing native tooltip/mark preview uses prepared hits and raw data. Hover/drag previews stay native; OCaml observes committed selection. Arbitrary tooltip content, cross-line orientation and marker customization are not exposed solely by `Chart.Config.style`. |

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
