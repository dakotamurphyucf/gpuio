# Stable ordinal chart colors

OCH-41, 2026-10-06. [Local qualification](../evidence/chart-ordinal-colors-och41.md)
passes. This document specifies the public contract; broader catalog/release
acceptance remains open.

`Chart_style.Ordinal.create ~domain ~range ?unknown ()` defines an explicit,
ordered color domain. Domain keys are typed separately from color values and from
source positions. `Chart_style.create ~ordinal ~theme ()` resolves the colors,
including tokens, and supplies that immutable mapping to the native chart.

## Keys and resolution

`Chart_style.Key` has typed constructors for Cartesian/categorical/radar series,
pie slices and Sankey nodes, plus `rising` and `falling` candlestick keys. The
namespaces are distinct: series 9, slice 9 and node 9 do not collide. A source's
label, current position and numeric value do not determine its ordinal identity.
A Sankey ribbon defaults to its source node's color. The optional
[ribbon policy](sankey-link-colors.md) instead uses its target color or blends
between both resolved endpoint colors.
Candlesticks retain hollow/filled movement cues independently of their colors.
Rising means close > open; flat candles keep the existing falling-color convention.

The domain contains 0–1024 unique keys; the range contains 1–32 colors. A known
key at domain position `i` uses range position `i mod range_length`. The current
data order can change without changing this lookup. Domain order is deliberate:
reordering the domain itself changes color assignments. Updating a dataset does
not implicitly grow or reorder the domain.

For an unknown key, `Ordinal.find` returns the optional unknown color. If it is
absent, native rendering uses the existing position-based palette for that entry.
This fallback is explicit and may change when an unmapped entry moves; use an
unknown color when unmapped entries also need stable colors. Empty domains are
valid, including an unknown-only mapping. Transparent colors are ordinary explicit
colors, not a synonym for the absence of an override.

The pinned `ScaleOrdinal` also cycles range positions and has an optional unknown
result. GPUIO deliberately rejects duplicate domains and empty ranges instead of
using first-duplicate lookup or returning None for an empty range. These bounded
validated choices are not arbitrary accessor callbacks. The pure OCaml `find`
operation is available for application-side use; the native implementation does
not call it during rendering.

## Native ownership and accounting

A worker constructs a bounded ordered lookup, then resolves one color per native
legend entry in source order. Cartesian/radar have at most 32 entries, pie/Sankey
at most 256, and candlesticks two. Prepared paths, marks, candle movement colors
and legend swatches use the same retained color vector. Preparation happens again
for a new style or source publication; no domain lookup is repeated in paint.
Selection and original-data browsing keep their source identities and order.
Tooltips keep the corresponding source title and their existing configured text
appearance; this feature does not introduce a new tooltip-color API.

The mapping's vectors count toward chart configuration retained bytes in both
the native tree and the worker/ready-result admission. The prepared color vector
counts toward the existing render-plan quota. Duplicate/size/identity/color
validation applies to decoded inputs as well as OCaml construction. Cancellation
is checked during lookup construction and source resolution. No synchronous OCaml
layout, paint or input callback is added.

## Wire and compatibility

The ordinal-color addition introduced **style version 0**, with
the existing fields followed by the optional ordinal record. The zero prefix
unambiguously rejects legacy frames, whose first value was a valid nonempty
palette length of 1–32. That style reader capped input at 16 KiB and the containing
view at 18 KiB. Domain/range counts are bounded before allocation.

Chart-options version 3 and chart-data version 1 are unchanged. The new paired
view fixture is `chart-v3-style-v0-view.hex`; the earlier v3 view fixture remains a
legacy-style rejection case. OCaml and Rust bridge packages must match.

Style **version -1** appended the [inspection configuration](chart-inspection.md)
after the ordinal field. The historical v0 fixture is now an explicit rejection
case; that addition's fixture is `chart-v3-style-inspection-view.hex`. The bounded
ordinal mapping remains unchanged. [Sankey presentation](sankey-presentation.md)
subsequently advances options to version 4; data remains version 1.

Current styles use **version -2**, appending [rich node labels](chart-node-labels.md),
with bounded decoder caps of 64 KiB for style and 66 KiB for view. The current
paired fixture is `chart-v5-link-colors-view.hex`; options/data are 5/1 after
[ribbon color policies](sankey-link-colors.md).

## Scope and validation

This maps the chart's existing color-bearing series/slice/node/movement entries.
It does not add category-specific colors within a Cartesian series, independent
Sankey link colors, arbitrary gradients or per-datum radii. Rich node-label styling
has its own contract; other gaps remain named in the catalog review.

Qualification must cover paired bytes, domain/key/unknown validation, theme
resolution, stable colors and names after reordering in every family, actual
paint colors and GPU readback, cancellation and retained charges, plus public
OCaml gallery and installed-consumer behavior. No native accessibility, physical
presentation latency or Linux desktop acceptance follows from these tests alone.
