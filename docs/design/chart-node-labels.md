# Rich Sankey node labels

OCH-41, 2026-10-06. [Scoped local qualification](../evidence/chart-node-labels-och41.md) is recorded;
this contract does not certify the complete chart catalog or release.

`Chart_node_labels.Line.create` validates preformatted text and optional color/font
size. `Node.create ~node` binds zero to four lines to a stable `Chart_data.Node_id`.
`Chart_node_labels.create` forms an immutable collection for
`Chart_style.create ~node_labels`. There are no native-to-OCaml label callbacks.

| Limit | Contract |
| --- | --- |
| Entries | At most 128 unique positive node IDs. |
| Lines | Zero to four per entry; zero explicitly hides that node's plotted label. |
| Text | At most 256 UTF-8 bytes per line, no ASCII control characters; empty text is allowed. |
| Total text | At most 32 KiB across the collection. |
| Font size | Optional finite 8–32 logical pixels; default native size is 11. |
| Color | Optional concrete/theme color; resolved with the chart style's theme. Missing tokens fail construction. |

No entry means the original plotted node label remains. IDs absent from the current
dataset are ignored; reordering data does not reassign captions by position.
`Chart_options.Sankey.labels=false` hides original and overridden labels alike.
The collection affects Sankey plots only. It never renames source nodes, changes
edge values, or changes the names in the original-data table and selection details.
Applications that show values in custom captions must update those captions
themselves alongside their model; arbitrary strings are not live native formulas.

Native preparation expands the original label into a retained block. Each line's
height is `max(font_size + 4, 18)` logical pixels. The existing node-edge anchor
and side alignment are preserved. The block is vertically centered, then shifted
inside the plot when it fits; a too-tall block clips its lower lines. Individual
lines are not independently clamped onto each other. Width remains bounded to
140 logical pixels and the view width, with text truncation. Zero-area lines are
not emitted as native label elements. Visible text uses the requested font/color
and a contrasting backing; the native label carries the full custom line text.
These semantics do not establish VoiceOver qualification.

The worker builds the expanded labels before plan admission. String/vector storage
is charged in the prepared plan and configuration accounting; the existing plan
bound is rechecked after expansion. Source identity, geometry, native hit testing,
data ownership and generation/revision fencing remain unchanged. Theme changes
require rebuilding the resolved chart style, as with the existing ordinal colors.

The containing style envelope advances **-1 → -2**, appending the label collection
after inspection controls. The standalone style decoder cap becomes 64 KiB and
the view cap becomes 66 KiB, accommodating the explicitly bounded text/metadata
alongside the existing ordinal mapping. A maximal valid metadata fixture checks
that admitted values fit those caps. Options remain version 4 and data version 1;
old style -1 frames are explicitly rejected. Both bridge packages must match.

The pinned `SankeyLabel` builder's multiline text, color and size are represented
as serializable values. Its outer-column margins/above-middle label placement and
source-to-target ribbon gradients remain separate catalog work. This adds no
arbitrary text-layout closure or unrestricted per-frame OCaml callback.
