# Rich chart inspection content

OCH-41, 2026-10-06. Implementation contract in progress. Validated target/content
values and paired metadata codecs use chart-view schema -2 with retained-tree
admission. `View.chart` attachment and an experimental native Card/Overlay adapter
are now implemented. [Initial interaction evidence](../evidence/chart-inspection-renderer-och41.md)
covers ordinary buttons and target/focus retention; broader widget/lifecycle,
rich-row helpers and public-gallery qualification remain required.

## Source capability and interface

The pinned [Tooltip source](../catalog/sources/component-plot-tooltip.rs.txt)
provides title plus colored swatch/label/value rows, or arbitrary `AnyElement`
children. Structured content takes precedence over freeform children. Its
`appearance(false)` bypasses the normal card and renders full-size content.
A text-only descriptor does not cover that arbitrary-child capability.

Use `Chart_inspection_content` to associate ordinary OCaml Views with semantic
chart targets. Its generic content parameter avoids a Chart/View module cycle.
At most 128 distinct targets are retained per chart. `Entry.create ~target content`
defaults to a native Card container; `~container:Overlay` uses the plot-sized
content region without the native card backing. These are layout choices, not
separate identities. Duplicate targets are rejected, regardless of container.
The collection never serializes its generic content: ordinary retained-tree
operations will carry the actual View children.

The `View.chart ~inspection_content` attachment coexists with
`~radar_labels`. Each target owns an internal wrapper keyed by target identity,
not entry order, caption or current hover index. Normal child keys remain local
to that wrapper. A convenience function for title/swatch/label/value rows will
compose ordinary Views, sharing the arbitrary-content renderer rather than
introducing a separate callback/serialization path. Rich annotations are supplied
as application-formatted text or arbitrary content before submission.

## Target identity and provenance

Singular targets are Cartesian `(series,datum)`, pie slice, radar `(series,axis)`,
individual candle, Sankey node or edge. IDs are validated positive domain IDs.
They survive source reordering; an exact selection's source index is deliberately
not its content identity. Sampling that chooses an original representative may
use that original target. Missing or unplotted targets have no visible card.

`Target.of_selection selection ~data ~data_revision ~data_generation` derives these
identities from a chart event. Publication numbers must be positive, including
for singular conversions. For Sum/Mean bars and OHLC candles it creates an
Aggregate target containing the borrowed source resource, exact publication and
complete selection span. The application identity is checked during metadata
binding, and the native target also carries the resource slot/generation.
Even a one-observation Sum/Mean/OHLC result remains an aggregate. A later revision
or generation cannot reuse it: equal endpoints/count do not prove unchanged
interior membership. Native lookup must compare the displayed immutable snapshot
and exact prepared selection. It must never assign first-datum content to an
aggregate or infer membership from numerically ordered IDs.

Unmatched targets use the existing native summary; original-data browsing always
remains available. Custom text is presentation, not a mutation of raw values.
The native accessible summary must continue to identify the actual original or
aggregated source; child controls provide their own ordinary View semantics.

## Rendering and interaction requirements

The retained adapter must use native measurement/prepaint without a callback into
OCaml. Card content respects the configured card width/padding/placement and
chart clipping. Overlay content uses plot bounds and ordinary View layout. Both
obey card visibility; marker/crosshair and original-data controls stay independent.
Theme and ordinary child View styling remain effective. Custom content must not
force source publication or retessellation solely because a child model changes.

Only the eligible current target's subtree may paint, accept input or expose
accessibility actions. Ordinary hover remains native, with no OCaml roundtrip.
The input contract must keep a pointer entering an interactive card from replacing
its target with the geometry underneath that card. Keyboard focus inside a custom
child must retain that target and route editing keys to the child, not the chart.
A source/target retirement overrides this retention. Pointer-leave, capture,
selection, Escape, blur and committed versus preview ownership require explicit
native state transitions and tests before integration is considered complete.

Retained hidden content may preserve its draft/model, but cannot retain active
focus, capture, popups or queued callbacks. Source generation changes, aggregate
revision changes, removed targets, original-data browsing, hidden/disabled charts,
unmount and window close must retire stale interactions before the next paint.
Use the existing per-chart slot gates and revocable input lifetimes from
[radar content](radar-label-content.md); one chart/window must not clear another's
hidden set. Bounds clipping must gate nested controls, not just the outer wrapper.

## Metadata and validation

Standalone metadata carries target and Card/Overlay tags only, bounded to 128
entries and 16 KiB encoded bytes. Target variants have fixed tags 0–6 for
Cartesian, Slice, Radar, Candlestick, Node, Edge and Aggregate. An Aggregate adds
resource slot/generation, positive publication revision/generation and the existing
validated aggregate selection. Each entry has an optional target: None denotes an
ineligible slot, never a wildcard or fallback.
Canonical wire-target comparison detects duplicate targets independently of content.
This also rejects equal wire identities supplied through different application
owners; a collection is intended for one chart in one application.
At binding, an aggregate from another resource or application becomes a targetless
(hidden) slot; even equal numeric handles from different application owners cannot
make it eligible. This preserves child-slot count without transferring authority.
Multiple targetless slots are valid; duplicate present targets remain invalid.
Changing the container does not bypass duplicate admission. Expert raw conversion
is not a replacement for the owner-aware metadata binding.

Chart-view schema **-2** now follows radar axis IDs with the inspection metadata
list, before legend/disabled flags. It rejects schema -1 rather than inferring
the shape of the remaining bytes. The standalone parent decoder budget includes
the extra 16 KiB metadata allowance; the normal message cap remains unchanged.
Retained accounting charges metadata vector capacity. Native tree admission
transactionally requires exactly the combined radar/content wrapper count, each
an empty-text Container with one child. Radar wrappers precede inspection wrappers.
The parent can hold at most 64 + 128 wrappers; normal node/depth budgets still apply.

`Chart.Expert.with_inspection_content` binds the generic collection to the config's
borrowed data identity. `View.chart` emits separately keyed radar and inspection
wrappers, preserving child identity across collection reorder and container changes.
Existing applications submit an empty inspection list. The native adapter combines
eligible radar slots and the one active inspection slot in the chart's visibility
gate; unmatched inspection wrappers stay retained but hidden. Ordinary child changes
use normal reconciliation without a chart-data publication or preparation request.

The initial adapter holds a custom target while the pointer is inside its clipped
container, a child has focus, or an existing child capture continues. Overlay uses
the entire plot as that container. Explicit navigation on the chart releases pointer
retention, so a resting pointer cannot prevent arrow/Home/End navigation or Escape.
Tab can enter an uncommitted preview without emitting a committed selection. Child
focus routes editing keys through ordinary controls. Source removal overrides
retention before paint; exact-ID children may remain eligible while new geometry
for a reordered publication is pending. Aggregate identity remains publication-bound.

These are implemented policies with scoped evidence, not complete acceptance of all
arbitrary child widgets. Native editor/IME, queued AX/command/popup, clipping,
aggregate and multi-window interaction combinations still need qualification.
Neither successful metadata decoding nor retained-tree admission qualifies rendering.

## Required evidence

- Public validation, stable-ID reorder identity, aggregate publication fences,
  namespace separation, duplicate/oversized admission, independent OCaml/Rust
  fixed bytes, truncated/unknown/malformed input and parent envelope bounds.
- Retained-tree key/order/content updates and malformed-child transactions.
- Actual rich rows/freeform View pixels, Card/Overlay layout, themes, resize,
  source/selection provenance and original-data browsing.
- Native pointer entry, child editor draft/focus/IME boundaries, command/popup/
  capture/queued AX retirement, clip/hide/return and multi-chart/window isolation.
- Root and independently installed public gallery walkthroughs and zero-resource
  cleanup, with beginner guides explaining actual Bonsai/GPUIO ownership.

OCH-41 stays open until that integration and broader catalog acceptance are met.
