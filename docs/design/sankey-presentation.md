# Sankey presentation controls

OCH-41, 2026-10-06. This contract extends the existing native Sankey geometry;
[qualification](../evidence/sankey-presentation-och41.md) is recorded separately.

`Chart_options.Sankey.create` accepts four additional optional fields:

| Option | Bounds | Default | Meaning |
| --- | --- | --- | --- |
| `node_corner_radius` | 0–32 | 1 | Logical-pixel node radius, clamped to half the actual width/height. |
| `link_opacity` | 0–1 | 0.5 | Multiplies the resolved source color's existing alpha. |
| `min_link_width` | 0–64 | 0 | Minimum logical-pixel ribbon endpoint thickness before clipping. |
| `label_gap` | 0–64 | 6 | Logical-pixel distance from the node's facing edge to its label anchor. |

All dimensions must be finite. Invalid values are rejected by public constructors,
wire validation and native decoding. These defaults preserve the earlier fixed
radius/opacity and unexpanded ribbons. The pinned source has different defaults
for some fields; exposing its functionality does not silently change established
GPUIO defaults.

Minimum width affects positive rendered ribbons. It does not change node flow
weights, raw source values, throughput, IDs or the original-data table. Zero flows
remain absent from the plot. Widened endpoint spans clip to the plotting rectangle,
so a ribbon at an edge may display less than the configured minimum. Overlapping
ribbons use the existing source-order paint and selection policy. Native hit
indexing uses the same widened geometry as painting. Opacity zero leaves geometry
and selection available deliberately; it is appearance, not input disabling.

Labels now start beyond the facing node edge, correcting the previous left-node
anchor that could overlap its rectangle. Nodes in the left plot half put labels
to their right; nodes in the right half put labels to their left. The prepared
label carries its alignment, so a large gap cannot accidentally flip its side.
Existing bounds/truncation still constrain labels in small views. This is not the
pinned source's outer-column margin/above-middle-column layout.

The worker prepares options, geometry, colors and the hit index together. Style
changes preserve the existing generation/revision fencing; no synchronous OCaml
callback, extra resource or hover-driven publication is introduced.

The options envelope advances **3 → 4**, appending four float64 fields after the
Sankey `labels` flag. The default options fixture is 100 bytes, with the existing
256-byte decoder bound unchanged. Old options-3 view frames are rejected; the
current paired view fixture is `chart-v4-style-inspection-view.hex`. Style schema
-1 and chart data version 1 are unchanged. OCaml and Rust packages must match.

Rich per-node multiline labels, per-line font/color and source-to-target ribbon
color gradients remain separate catalog work. This contract does not claim full
upstream plotting-builder parity, VoiceOver or performance qualification.
