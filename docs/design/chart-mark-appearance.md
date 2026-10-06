# Chart mark appearance

OCH-41 implementation design, 2026-10-06. Native preparation, codecs and public
gallery behavior have [scoped local qualification](../evidence/chart-mark-appearance-och41.md).
Broader catalog and release acceptance remain separate. Existing charts keep
their current defaults.

## Ownership and identity

`Chart_appearance` describes immutable presentation. `Chart_style.create`
resolves its theme tokens before submission; no native layout, paint, hit testing
or tooltip callback invokes OCaml. Series overrides use stable series IDs. Datum
overrides use `(series_id, datum_id)`, including radar axis IDs. Neither uses array
indices, category captions nor display order. Unknown IDs remain dormant, and
sampling never creates a mark merely because an override exists.

Resolution order is existing chart style/options, series override, then datum
override for fields supported by that mark. Omission inherits a field; explicit
false hides it. Removing an override restores inheritance. A path is styled as a
whole series; datum overrides affect markers and bars, not arbitrary fragments
of a continuous line. Pie slice colors retain their existing ordinal interface.

Path stroke controls visibility, width and brush independently of fill. Explicit
area/radar fill replaces the default palette plus area opacity: the supplied
brush carries its own alpha. Marker radius, fill, border and visibility are
independent; existing geometry visibility (including the isolated-point fallback)
remains a master gate. Borders default to zero width. Hidden markers retain the
default selection radius without removing line/area/radar selection. Visible
marker hit bounds include their effective radius plus the normal hit tolerance.

Series may supply a separate legend color. Otherwise the existing ordinal/palette
legend color remains stable. A gradient or many differently colored bars has no
single canonical swatch; this choice is explicit rather than inferred from the
first datum. Presentation does not alter source values, selection provenance,
accessible names or the original-data table.

## Curves and stacking

Grouped line/area series may override their curve. Stacked area boundaries must
use the same effective curve throughout the stack. Preparation must reject
conflicting effective area curves with `Chart.Error.Invalid_config`; it must not
silently draw gaps/overlaps or choose the first series' curve. Stacked bars and
unstacked lines are independent of this restriction. Radar ignores curve and bar
fields; family changes retain otherwise dormant configuration.

## Bars and aggregation

Corners are physical top-left, top-right, bottom-right and bottom-left radii,
bounded to 0..32 logical pixels and clamped to the actual bar bounds. Orientation
changes do not rename physical corners.

Bar fills offer four explicit coordinate systems:

* `background`: existing solid/two-stop sRGB or Oklab brush, with physical angles
  relative to the individual rectangle.
* `base_to_tip`: sRGB endpoints follow the actual base and tip, including negative
  values, reversed orientations and nonzero stacked baselines.
* `domain`: sRGB ramp across the prepared value domain, including cumulative
  stack bounds and aggregated values.
* `values`: sRGB ramp anchored to two finite, strictly increasing source values;
  out-of-range values use the corresponding endpoint color.

Value ramps are clipped/resolved before painting; do not send unbounded stop
percentages to the GPU or divide by a zero-length domain. A constant domain uses
the first color. Preserve the existing global `gradient_end` behavior when no
override is supplied. New ramps must not quietly reinterpret that legacy option.

Aggregate bars retain their original source spans. `Inherit_series`, the default,
ignores datum overrides on bars containing multiple defined observations.
`Uniform` uses a datum-derived appearance only when every participating defined
observation has the same effective bar appearance after inheritance; otherwise
it uses the series fallback. Compare resolved brushes/corners, not pointer identity
or only override presence. Missing categorical observations do not participate.
Single-observation bars use their datum override under either policy. Never
represent a heterogeneous aggregate with its first observation's style.

Build indexed overrides once during preparation. Aggregate spans must not cause
repeated full-source scans or per-frame work. Cancellation and existing render
budgets apply to this preparation too.

## Bounds and delivery

At most 128 distinct series overrides and 1,024 distinct datum pairs are allowed.
These support sparse highlighting independently of the 100,000-point source
limit. They are not a dense styling channel for every possible source point.
Marker radius is 1..24, border width 0..8 (clamped to radius), and path stroke
width 0.5..8 logical pixels. Colors resolve even for currently hidden/dormant IDs;
missing theme definitions are recoverable errors.

The independent appearance envelope is 192 KiB; the paired maximum encoding is
173,447 bytes. Attachment advanced the chart style schema to -5 and its bounded
envelope to 384 KiB. The later [cursor extension](chart-cursor-inspection.md)
advances the schema to -6 without changing these limits. A maximally populated parent style, including axis/node/pie
captions and all appearance overrides, is tested against this envelope and the
outer 1 MiB message limit. Matching OCaml and Rust packages are required.

The existing `Background` API covers solids and two-stop gradients. GPUI's other
background patterns, arbitrary per-datum baselines and the low-level area shape's
scalar `y0` baseline are separate catalog work, not capabilities established by
these paint overrides. Rich inspection remains required separately. The pinned
area shape has a scalar optional `y0` and a `y1` accessor; it does not establish
arbitrary per-datum lower and upper accessors. See the
[pinned shape source](../catalog/sources/component-plot-shape-area.rs.txt) and
[bar gradient source](../catalog/sources/component-chart-bar_chart.rs.txt).
Dense per-datum styling beyond the sparse limit still needs a bounded resource
design if required for the catalog equivalent; the limit does not create a
post-v1 deferral or establish full arbitrary-background parity.

Validation must cover paired bytes and rejection, theme resolution, stable IDs,
aggregate/missing-value rules, stack curve admission, all four directions,
negative/stacked gradients, hit testing, actual GPU pixels and public gallery
updates/selection/teardown. Unit construction alone is not native acceptance.
