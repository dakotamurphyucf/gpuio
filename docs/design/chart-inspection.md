# Native chart inspection presentation

OCH-41, 2026-10-06. [Local qualification](../evidence/chart-inspection-och41.md)
passes. This contract does not establish whole-catalog, VoiceOver or release acceptance.

`Chart_inspection` separates `Card`, `Crosshair` and `Marker` configuration.
Pass the resulting immutable configuration to `Chart_style.create ~inspection`.
All lengths are logical pixels, all constructors validate finite bounds, and
optional colors resolve through the style's theme. An omitted color inherits the
native chart foreground, backing or selection color. A transparent color remains
an explicit override. No hover, layout, formatting or paint callback enters OCaml.

## Card and marker

The default card preserves the existing top-right source title and value summary.
`Placement.Anchor` places it beside the inspected mark, flips left/up when needed,
and limits width/height to the available chart content. It follows the prepared
mark, not every cursor movement. Bottom anchoring uses actual element height,
including wrapped values. Excess content is clipped; original-data browsing
remains the complete alternative. The card stays below the data-control header
and above the legend when space permits; tiny viewports clip the whole overlay.

Card options control visibility, title/value presentation, width, gap, padding,
corner radius, text size/line height, border and text/background/border colors.
Line height must be at least text size. Title/value visibility controls visual
children; the visible card's accessible summary retains the full source details.
Hiding the entire card removes that summary, but preserves chart selection and
the original-data control.

The marker has independent visibility, size, fill, stroke width/color and status
cue visibility. Its default check/circle distinguishes committed selection from
preview. Stroke width cannot exceed half the marker size. Applications that hide
the status cue deliberately remove that visual distinction; event and source
identity semantics remain unchanged.

## Crosshair

Axis is Off, Vertical, Horizontal or Both. Pattern is Dashed or Solid; thickness
is 0.5–64 pixels, so a solid crosshair can be a highlight band. The crosshair spans
the prepared plot and uses the inspected mark's anchor. At plot endpoints it
stays inside the plotting rectangle; the containing element clips narrow plots.
Dashed borders have nonzero layout area so native painting includes them.

The pinned source also exposes custom partial spans, arbitrary tooltip children
and per-row content. These controls do not claim those APIs. Native summaries
continue to describe the exact prepared source, including category IDs, aggregate
sample counts, raw values and stack bounds. Arbitrary rich rows, caller-formatted
per-datum annotations and cursor-following cards remain separate presentation
work. The static extension SDK remains available for custom native plots.

## Lifetime and wire contract

These values belong to the immutable chart style, so style changes participate in
ordinary native re-preparation and stale-input fencing. Prepared results retain
matching geometry/style until replacement is ready. The fixed-size inspection
record is boxed to avoid enlarging every protocol operation; its allocation is
included in style/configuration retained accounting, alongside the ordinal mapping. No extra resource registration,
timer, permanent polling or source publication is introduced by inspection.

The plot owns pointer selection and capture, but does not own a wheel gesture.
Wheel events continue to the enclosing scroll view, including while inspection
overlays are visible. Scrolling geometry changes retain committed selection and
use the existing native preview/capture cancellation rules.

The style schema tag advances from **0 to -1**. Nonpositive schema tags keep the
version namespace disjoint from valid legacy unversioned palette lengths 1–32.
Previous style/version-0 view frames are explicitly rejected; matching OCaml and
Rust bridge packages are required. [Sankey presentation](sankey-presentation.md) subsequently advances options to
version 4; data version 1 remains unchanged. The inspection record follows the optional ordinal field; the existing
16 KiB style and 18 KiB view caps covered that bounded maximum. The subsequent
[rich node-label addition](chart-node-labels.md) advances style to -2 and raises
the caps to 64/66 KiB; options/data remain 4/1.
