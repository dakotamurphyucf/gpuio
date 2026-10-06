# Sankey ribbon color policies

OCH-41, 2026-10-06. Scoped local qualification passes; see the
[evidence](../evidence/sankey-link-colors-och41.md). Whole-release gates remain open.

`Chart_options.Sankey.Link_color` contains Source, Target and Gradient.
`Sankey.create ~link_color` defaults to Source, preserving existing presentation.
Source and Target use the resolved color of the respective node. Gradient blends
from source at the left endpoint to target at the right endpoint. Both endpoint
alphas are multiplied by `link_opacity`, including existing color transparency.
Stable ordinal mappings and their unknown-key policies apply before these choices.

The worker resolves endpoint IDs against the current immutable dataset once per
preparation. Colors do not depend on node list position when an ordinal mapping
is present. Missing endpoints are invalid source data; no fabricated fallback is
introduced. Positive ribbons retain their existing geometry, minimum width,
clipping and hit/selection identity; zero flows remain absent.

Each prepared ribbon owns one retained tessellated mesh and its optional second
color. The GPUI painter submits one path for the whole mesh with a 90-degree
linear background, so interpolation spans the complete ribbon rather than
restarting in each triangle. The existing mesh frame budget and clipping remain
in force. The added brush metadata is included by the prepared-plan size charge;
there is no per-frame OCaml callback, additional tessellation or color lookup.

Options schema **4 → 5** appends one enum tag after label gap: Source=0, Target=1,
Gradient=2. The default options fixture is 101 bytes and the 256-byte options
bound remains unchanged. Style/data versions remain -2/1. The original paired
view fixture was `chart-v5-link-colors-view.hex`. The subsequent
[outside-label option](sankey-label-placement.md) advanced options to 6
(102 bytes); [radar presentation](radar-presentation.md) now advances them to 7
(112 default bytes) with its matching fixture. Old versions and unknown policy
tags are rejected; both bridge packages must match.

The pinned source always paints endpoint gradients. GPUIO exposes that behavior
as an explicit choice alongside the existing flat-source default and a flat-target
choice. This does not add arbitrary link brushes, per-edge callbacks, outer-column
label margins or above-middle labels. Full catalog, accessibility, performance
and platform qualification require their separate evidence.
