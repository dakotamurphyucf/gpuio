# Radar presentation contract — OCH-41

Implemented with [scoped local evidence](../evidence/radar-projection-och41.md),
2026-10-06. This extends the existing per-axis radar
API with the pinned catalog's shared scale, outer radius and label spacing.
Rich axis-label content remains separate unfinished catalog work; this contract
is not complete radar-family acceptance.

`Chart_options.Radar.Scale` is `Per_axis | Data_max | Maximum of float`.
Per_axis remains the default. Data_max scans all series in the immutable snapshot;
all-zero/empty series project to the center. Maximum accepts a finite positive
value up to 1e100. It is independent of the dataset's per-axis admission maxima:
values may extend beyond the outer ring, matching the pin's extrapolating linear
scale. Original values, IDs, table rows and selection provenance do not change.
No OCaml callbacks run during native layout, paint or input.

`Radius` is `Fit | Pixels of float`. Fit preserves the half-minimum-plot-dimension
radius after gutters. Pixels accepts (0,32768] logical pixels. The native plot
mask clips oversized geometry; a specified radius never silently becomes Fit.
`label_gap` accepts [0,64] logical pixels and defaults to zero for compatibility.
Visible labels gain that radial offset and additional gutters, bounded by the
existing proportional limits in tiny viewports. Hidden labels reserve no gap.

Projection retains f64 arithmetic until the existing mesh conversion. Coordinates
must be finite and within the canvas coordinate budget of one million logical
pixels. Extrapolation beyond that limit returns Render_limit before tessellation;
it does not clamp vertices and distort the polygon. Partial plans are discarded.
A valid public options record can therefore fail preparation for extreme data,
just as other charts can exceed the declared rendering resource budgets.

Options wire schema becomes 7, with explicit scale/radius tags and gap following
radar's existing levels/dots/labels fields. Both decoders validate every field;
old schemas fail explicitly. Style/data schemas stay -2/1. Match OCaml and Rust
package revisions; this experimental wire change is not a compatibility promise.

Verification must cover independent paired fixtures, malformed/legacy decoding,
per-axis versus shared projections, zero/empty values, extrapolation and resource
failure, radius/gap behavior, immutable provenance, mounted preparation and a public
example. Build/worker evidence alone does not establish native input or pixels.
