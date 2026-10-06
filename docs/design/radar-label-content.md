# Radar label content — OCH-41

Implementation design, 2026-10-06. **The public child-content API and native slot
adapter are not implemented yet.** Stable axis provenance is the first prerequisite;
the existing text-label renderer uses it. This document defines the remaining
work and must not be read as feature or native acceptance evidence.

## Source behavior and scope

The pinned [RadarChart source](../catalog/sources/component-chart-radar_chart.rs.txt)
at `84f57fdfcb4910623fb0bb7f795b077e249f9271` accepts `RadarLabel::Text` or
`RadarLabel::Element(AnyElement)`. Element labels are arbitrary native content,
measured during prepaint, styled independently of `label_color`, and omit the
axis title from the hover tooltip. Merely adding multiline text descriptors would
not cover this capability. GPUIO will accept ordinary OCaml Views, including icons,
buttons and inputs, with no OCaml callback from native layout, paint or measurement.

Existing [projection behavior](radar-presentation.md) stays intact: per-axis and
shared scales, Fit or fixed radius, label gap, immutable original data and bounded
geometry. Unlike the pin, GPUIO already permits axes/grid without any series;
custom labels must work in that state too. Source captions remain the default.

## Proposed OCaml interface

Use an abstract collection so duplicate targets and excessive slot counts fail
before view reconciliation. Content is generic to avoid a Chart/View module cycle:

```ocaml
module Chart_radar_labels : sig
  module Entry : sig
    type 'view t
    val create : axis:Chart_data.Datum_id.t -> 'view -> 'view t
    val axis : _ t -> Chart_data.Datum_id.t
    val content : 'view t -> 'view
  end

  type 'view t
  val create : 'view Entry.t list -> 'view t Or_error.t
  val empty : 'view t
end

(* Additional argument in the View module's interface. *)
val chart
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?on_event:(Chart.Event.t -> 'action)
  -> ?radar_labels:'action View.t Chart_radar_labels.t
  -> Chart.Config.t
  -> 'action View.t
```

The collection admits at most 64 distinct axis IDs, matching the dataset limit.
Axis identity, not caption text or list position, keys an internal retained
wrapper. Normal child keys continue to control identity within that wrapper.
Changing a caption or reordering the collection must not recreate an input.
Removing an entry unmounts its content. Returning the same axis later does not
resurrect native state from an unmounted subtree.

No synchronous dataset lookup is required by collection construction. An entry
whose axis is absent from the current radar snapshot is retained but hidden and
noninteractive. It becomes eligible when that ID appears. All entries are hidden
for non-radar datasets or `Radar.labels = false`. A source reset does not reset
the application's Bonsai model; applications can key their component by source
generation if that is their desired policy.

## Native measurement and placement

Each eligible ordinary View subtree is laid out natively at its natural size
using GPUI's root-layout facility during prepaint. The normal View style API can
set width, height, wrapping and overflow. The bridge does not impose a mandatory
fixed label rectangle or substitute a text-only rendering path.

For axis index `i` among `n` axes, the spoke direction is
`u = (cos(i * 2π/n - π/2), sin(i * 2π/n - π/2))`. The prepared label position
already contains `center + u * (radius + gap)`. Given measured size `s`, the
native origin is `plot_origin + label_position + (u - 1) * s / 2`, componentwise,
matching the pin. At the top this centers the box horizontally and puts its bottom
at the anchor. At diagonal angles this formula does **not** mathematically ensure
every corner clears the circular ring; do not describe it as such a guarantee.

Content clips to the chart viewport. Large labels may overlap or clip, as arbitrary
native label content can in the pin. The adapter does not silently move axes,
shrink a fixed radius or reflow the plot to avoid collisions. Authors use normal
View sizing, chart dimensions, radius and gap to choose their presentation.
The existing default plain-caption placement remains compatible. Label-only
content updates must not require re-tessellating otherwise unchanged polygons.

This avoids a measurement/worker feedback loop: only the main-domain retained
Views need measuring; immutable chart geometry remains on the existing worker.
Do not measure in paint. Non-finite or excessive native bounds must be rejected
or clipped using the same geometry limits as the enclosing chart, without
passing invalid coordinates into GPUI.

## Wire and retained tree

Carry ordered axis IDs in chart-view metadata and serialize the content through
ordinary retained-tree operations. Do not serialize an OCaml callback, encode IDs
in display text, or invent an extension merely to carry built-in label children.
The chart's child count must equal the metadata count; each internal slot owns one
content subtree. Validate the complete resulting tree transactionally, including
unique positive axis IDs and the 64-slot limit.

Introduce an explicit negative chart-view schema prefix (`-1`) rather than
silently appending fields to the existing unversioned Config. Legacy first bytes
are Option tags 0/1, so a negative prefix distinguishes the new envelope. Matching
OCaml/Rust package revisions are required. Options/style/data versions need not
change solely for this chart-view metadata. Add independent paired fixtures and
truncation/invalid-tree tests; account for metadata capacity in retained bytes.

Prepared radar captions carry stable axis IDs separately from pie captions.
Use those IDs to join content to positions and suppress only the corresponding
plain caption. Never match strings, assume caption-array offsets equal axes, or
join against a newer snapshot than the displayed plan.

## Interaction, accessibility and lifetime

The ordinary View renderer owns the content's semantics, handlers, editors,
commands and resources. Native chart keyboard navigation requires exact chart
focus, so a focused label editor retains its own keys. A disabled chart disables
its descendants; ancestor checks must be extended now that charts can have
children. Custom content supplies its own accessible name and ordinary View
styling. Chart `label_color` continues to apply to default text only.

Slot visibility depends on current source generation, axis membership, chart
visibility, and whether the plot rather than the original-data browser is visible.
Disabled descendants remain visible and accessible with disabled semantics, but
cannot interact. Gates must change on the relevant state transition, even if an occluded
window does not paint. Opening the data browser, resetting/releasing/replacing a
source, hiding/unmounting the chart, removing an axis or closing a window must
retire stale child input, focus and accessibility actions. The old picture may
remain during ordinary worker preparation, but an old axis must not remain an
actionable control after removal from the live source.

Use owned slot sets per chart; one chart must not clear another's hidden set.
Clipping must gate descendant semantics and input, including a fully clipped
control inside a partly visible composite. Reuse the existing retained-slot
visibility mechanisms where they have these semantics. Retaining an invisible
subtree is not permission to retain its active pointer capture or popup.

Preserve focus for an unchanged, still-visible axis on ordinary value updates.
When its target becomes ineligible, retire native editor focus and move to a
valid enclosing target using the repository's established focus rules. Do not
claim cleanup merely because an element was omitted from the next render tree.

For custom element labels, omit the display-axis title from the hover tooltip,
matching the pin. Original-data tables, source IDs, raw values and selection
observations stay unchanged; those are data contracts, not presentation labels.

## Required evidence before acceptance

- Core collection validation, axis-keyed reconciliation, reorder/rename preservation,
  child removal/remount and ordinary child event routing.
- Paired config codecs, legacy/truncated/malformed rejection, native tree validation
  and retained-byte accounting. Existing no-label chart tests continue to pass.
- Native prepaint with text, icon and an interactive child; cardinal/diagonal anchors,
  explicit and natural sizes, narrow/oversized clips, theme/font/scale changes and
  absence of unnecessary geometry work on label-only updates.
- Axis reorder/removal/return, unknown-to-known IDs, empty series, labels hidden,
  family changes, source replacement/reset/release and in-flight worker completion.
- Actual focus, keyboard/editor input, pointer and accessible actions; chart disabled,
  browser open/close, clipped/hidden slots and stale commands/callbacks. Include two
  charts and two windows to test isolation, plus unmount/window-close cleanup.
- Public OCaml gallery controls and adjacent code walkthrough, root and freshly
  installed consumer qualification, required Linux compilation/unit checks.

Native screenshots alone do not qualify focus/accessibility, and these planned
checks do not establish VoiceOver or Linux desktop acceptance.
