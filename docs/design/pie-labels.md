# Pie labels and leader lines — OCH-41

Implemented contract, 2026-10-06, following fixed/per-slice radii (`fd2bc6f`).
[Scoped local qualification](../evidence/pie-labels-och41.md) records actual evidence
and remaining platform/release limits. The pinned source
`component-chart-pie_chart.rs.txt` exposes outside labels, label spacing, global
text color and per-slice leader-line colors. Its paint-time closures become
bounded serialized values; no native measurement or painting enters OCaml.

## Public contract

- `Chart_options.Pie.Label_placement.Inside | Outside`, default Inside, preserves
  existing apps. `Pie.create ?label_placement ?label_gap` adds a finite gap in
  [0,64] logical pixels (default 15). Gap affects outside labels only.
- `Chart_pie_labels.Entry.create ~slice ?text ?line_color ()` and a bounded
  collection give stable-ID presentation overrides. Omitted text inherits the
  source label; explicit empty text suppresses its caption/leader. Optional
  per-slice line color overrides the chart's leader color. Unknown IDs are
  retained but ignored until present. Source names, weights, legend and original
  data rows are unchanged.
- `Chart_style.create ?pie_labels ?pie_label_line_color` resolves tokens against
  its theme. The default leader color inherits the resolved axis color. Existing
  `label_color` supplies caption color. Text is single-line, at most 256 UTF-8
  bytes without ASCII controls; at most 256 unique IDs / 32 KiB total override
  text. Style/config accounting includes retained text and entries.

## Native ownership and layout

A mounted outside-label pie captures its native font context. The admitted chart
worker measures bounded captions, reserves symmetric measured horizontal margins
for Fit, prepares wedges and lays out captions. All ready geometry/text belongs
to the same immutable source/config/font snapshot; changing font/config cancels
older work through the existing request serial. Inside defaults keep their
previous geometry and styling.

Leader anchors use each visible wedge's actual outer radius and angular midpoint,
including ID-keyed radius overrides. Stable source indices accompany labels;
renaming or repeated captions never merges identities. Text and line-color
presentation overrides join through slice IDs. Zero source weights/equal radii
produce no wedge or caption. Outside labels for sweeps below half a degree are
omitted, following the pinned component; original values remain available.

Outside candidates are split by side and sorted by target vertical position with
source order as a deterministic tie-break. Two passes spread them within the plot.
If a side cannot fit all 18-pixel rows, retain a deterministic evenly distributed
subset instead of stacking labels on top of each other. A plot shorter than one
row has no outside captions. Caption widths are bounded to a quarter of the plot
and ellipsize. Fit reserves measured margins, bounded proportionally to leave a
positive plotting radius in cramped views. Explicit/per-slice radii remain literal;
large wedges and leader segments clip to the plot and may enter label space.
Do not promise nonoverlap between arbitrarily oversized wedges and captions.

Leaders are retained native stroke geometry, painted before text, with edge,
bend and horizontal endpoint. They are decorative, not separate hit targets.
Outside text is unbacked, single-line and aligned away from the center; inside
text retains its contrast backing. Label hiding does not remove the original-data
row; absent wedges cannot be hit/keyboard-previewed as documented for radii.

Options schema advances to 9; chart style to -3. View/data stay -1/1. Explicitly
reject old nested revisions; update paired current fixtures and preserve legacy
rejection evidence. Style frame bounds must cover both independently bounded
Sankey and pie overrides plus ordinal/inspection payloads.

## Required evidence

Paired OCaml/Rust bytes and invalid/truncated/count/color/text/old-version cases;
Core token resolution/ID/text budgets; geometry placement/leader provenance for
variable radii, reorder/rename, hidden/zero/tiny/dense/small/long labels; native
font measurement and cancellation; bounded meshes and actual GPU line/text pixels;
public OCaml controls, themes and raw selection/data preservation from root and
fresh installed consumer; resource teardown, strict checks and exact source/platform
records. Full Linux nongraphical checks remain required; Linux desktop remains
OCH-47. This increment does not complete unrelated chart/catalog/release gates.
