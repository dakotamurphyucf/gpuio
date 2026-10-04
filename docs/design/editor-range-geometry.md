# Editor range geometry

OCH-41 implementation work. The native helper has [local regression evidence](../evidence/editor-range-geometry-och41.md). The public OCaml query has
[local boundary validation](../evidence/editor-range-api-och41.md). Physical acceptance remains open.

## Native contract

Input uses UTF-8 source byte offsets, with a half-open non-reversed range.
Both endpoints must be valid boundaries within the current source. A collapsed
range describes a zero-width caret rectangle with the laid-out line height.
Nonempty ranges return the union of their rendered glyph spans and endpoint
caret rectangles, including intervening lines and soft wraps. Endpoints at a
soft-wrap boundary use leading affinity for the start and trailing affinity for
the end. Newlines have no glyph cell; their endpoints still contribute geometry.

Coordinates are logical pixels relative to the window content, taken from one
completed layout/paint. Bounds are not clipped to the viewport or ancestor masks
and are not a physical visibility or presentation receipt. Both endpoints must
be represented in the retained layout; off-layout ranges return `None`. Native
overscan can include rows beyond the visible viewport. No offscreen shaping,
scroll, focus, selection, text, undo or composition mutation is performed.

The recorded layout carries its source revision and masking mode. An edit or
masking change before the next paint makes source-index queries unavailable;
combining current source indices with an older layout is forbidden. Layout-only
changes may still return the preceding coherent layout until the next paint.
Masked inputs translate logical source offsets into display-mask offsets.
Range rectangles use the same glyph-span geometry as native range backgrounds,
including reordered shaped cells. Work is limited to the retained layout; the
union has constant output size, and any shared shaped-cell cache follows the
existing layout lifetime.

The code-editor/LSP family has additional folding, ghost text and adornment
semantics and remains post-v1. This work qualifies ordinary single-line inputs
and text areas; it must not imply general code-editor geometry acceptance.

## Public query contract

The asynchronous query targets an exact editor lease with an explicit source
snapshot revision and a validated UTF-8 source range. A newer native revision produces
`Stale_revision`; invalid source boundaries produce `Invalid_selection`; an
unavailable matching layout yields an explicit absent result. The result must
record its source revision and coordinate semantics. It must not overwrite the
controller's text snapshot or install a polling subscription. Existing bounded
editor requests and unmount/window-close rules apply.

The interface is `Gpuio_eio.Text_input.range_bounds editor ~snapshot ~range`,
returning `(Gpuio.Editor_geometry.t option, Text_input.Command_error.t) result
Effect.t`. The directed `Text_input.Selection.t` is normalized at the native
boundary. `Editor_geometry` exposes the source revision and x/y/width/height.
Coordinates are finite in ±1e9 logical pixels; width is 0..1e9 and height is
positive and at most 1e9. No source text is sent with the query.

This follows existing exact-lease metadata timing: a successful delayed result
can describe a preceding paint and source even if later work has since occurred.
It never installs that source as the controller's current snapshot. Callers
needing a current anchor must compare with their current application observation.
Native source mismatch at execution returns `Stale_revision`, while malformed
response geometry or a response revision differing from the requested revision
returns `Native_failure`. Closed windows cancel outstanding requests.

The unpublished bridge appends command tag 11 `(revision, selection)` and result
tag 7 with optional `(revision, x, y, width, height)`. All prior tags remain stable;
both languages ship together. Request offsets are in 0..262144 and revision is
nonnegative. Decoders reject malformed tags/offsets, truncation and trailing data.
