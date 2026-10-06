# Cartesian chart value directions

`Chart_options.Orientation` now has four variants:

| Variant | Category direction | Increasing numeric value |
| --- | --- | --- |
| `Vertical` | left to right | upward |
| `Horizontal` | top to bottom | rightward |
| `Vertical_reversed` | left to right | downward |
| `Horizontal_reversed` | top to bottom | leftward |

The two existing variants retain their behavior and wire tags 0/1. Reversed
variants append tags 2/3 in the existing options envelope. Both bridge packages
must support the selected variant; an older reader rejects the unknown tag.
Default options encode to the same bytes as before.

Direction is a native value-coordinate transform for all Cartesian layers,
including mixed line/area/bar plots. It does not mutate source values, IDs,
source order, aggregation spans, or the original-data table. Signed values keep
their meaning and the zero baseline follows the numeric domain. Area fills,
curves, point markers, value ticks and grid lines use the same transform.
Bar gradients mirror with the value direction. Axis gutters stay on their
existing sides; reversing values does not request top/right tick-label layout.

Both horizontal variants use the plot height as the category extent for source
reduction, bar grouping and nearest-sample hit queries. Reversal does not reduce
against the width or reverse semantic keyboard order. Selection remains tied
to the same source datum/publication and uses the actual mirrored hit geometry.

Non-Cartesian families retain their existing layouts even when these options
are present: pie, radar, candlestick and Sankey do not inherit the Cartesian
direction setting. Other source plotting options—including categorical scales,
stacking, independent bar corners, richer labels and tooltip styling—remain
separate catalog work. This addition does not certify whole-family parity.

The public [gallery walkthrough](../../examples/gallery/charts_page.md) shows
both direction controls. [Evidence](../evidence/chart-directions-och41.md)
records codec, geometry/provenance, hit-index and native-pixel qualification.
