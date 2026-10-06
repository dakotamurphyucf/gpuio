# Sankey outside-label placement

**Draft implementation plan, OCH-41, 2026-10-06. Not shipped or qualified.**
Current behavior remains the [inside-label presentation](sankey-presentation.md)
and [ID-keyed rich label values](chart-node-labels.md). This document makes the
next catalog gap concrete without claiming it is implemented.

## Public contract to add

Extend `Chart_options.Sankey` with `Label_placement.t = Inside | Outside` and
optional `?label_placement`, default Inside. Existing applications keep their
current layout. Outside places the first column's labels to the left, the last
column's labels to the right, and middle-column blocks above their nodes.
For a single column, first-column placement takes precedence, as in the pin.
The existing global labels flag suppresses all labels and their reserved space.
Missing rich overrides use source labels; empty overrides reserve no label space.

Source values, IDs, node/edge membership, alignments, zero-flow omission, scaling
and selection semantics remain unchanged. Outside changes geometry deliberately:
paint and hit testing must use the same new extent and prepared source revision.
Rich font/color and label gap compose with the new policy. Colors do not affect
measurement. No user layout closure or per-frame OCaml callback is introduced.

The implementation would advance options schema 5 to 6 with a bounded enum tag,
keep style/data -2/1, add an independent paired view fixture, and reject unsupported
versions/invalid tags. These are planned schema changes, not current wire values.

## Native measurement before geometry

The pinned [Sankey chart](../catalog/sources/component-chart-sankey_chart.rs.txt)
uses topology first, measures the widest first/last label, reserves margins,
then lays out that topology in the remaining extent. Its
[label helper](../catalog/sources/component-plot-label.rs.txt) shapes text with
the current native font. GPUIO's current `chart_geometry::prepare` fills the
original plot and `chart_node_labels::apply` expands rich text afterward; merely
moving those labels would overlap ribbons or clip them without reserving space.

Keep the actual measurement on the existing bounded chart worker:

1. Native chart request captures the inherited `gpui::Font` and the app's
   `Arc<TextSystem>`. Capture no Window/App borrow or OCaml value. Only requests
   requiring measured outside labels need this additional context.
2. Worker constructs its own `WindowTextSystem` around the shared text system.
   Do not use the window's frame-layout cache from a worker. Collect default or
   overridden lines by node identity, shape each at its effective font size and
   obtain actual widths, including Unicode fallback. Add the actual label backing
   padding used by GPUIO. Do not estimate width from byte/scalar counts.
3. Pass ordinary bounded width/block-height metadata to the geometry layer. That
   layer reuses one validated topology, reserves margins and computes node/ribbon
   layout against the remaining extent. Text-system types stay out of pure geometry.
4. Retain explicit placement metadata with each label: horizontal alignment,
   width budget and vertical center/above anchor. Rich expansion preserves that
   placement across all lines of a coherent block. Rendering uses native ellipsis
   and clipping rather than silently deleting source/accessibility text.
5. Paint labels with the same captured font used for measurement. A pending new
   font/layout request must not paint an old prepared extent with a different font.
   Include the relevant font identity in request equality/invalidation; width,
   source revision, options and rich label changes already invalidate preparation.

Use the pin's independent **20% plot-width cap per side** and **60% combined
vertical-margin cap**. Reserve the tallest middle block plus its vertical gap;
reserve the small bottom gap when labels exist. Retain current configurable node
width/padding semantics, fitting them to the *new* available extent. Tiny views
must remain finite and bounded; do not force a one-pixel extent beyond their
actual subpixel remaining area. Wide labels ellipsize within their margin and
middle labels use the smaller distance to the plot edges as their centered budget.
A dense graph may still require application-controlled hiding/shorter labels;
do not promise globally nonoverlapping labels without proving that behavior.

## Lifetime and bounds

Measurement stays inside `chart_jobs`' existing two-worker limit, cancellation
and output/workspace admission. Check cancellation between lines and topology/
layout stages. Charge retained placement/string storage through the existing plan
accounting. Document temporary metric/cache storage and discard the private cache
after preparation; do not accumulate a per-revision cache on the window.

A closed/replaced view must reject stale work as today. An app text-system handle
may remain owned until a bounded worker exits; it must not retain a window or make
that worker await the UI. Native measurement failure must return the normal typed
preparation failure, not substitute approximate metrics silently.

## Feasibility evidence and required acceptance

A scratch Rust probe on `efc1711` compiled a function moving `Arc<TextSystem>`,
Font and owned text into `std::thread::spawn`, constructing a private
`WindowTextSystem`, shaping a line and returning its width. This proves the pinned
API/type boundary compiles; it did **not execute measurement**, open a window or
qualify any platform behavior. PlatformTextSystem is explicitly Send + Sync in
the pin. [Probe source and command log](../evidence/sankey-label-measurement-probe.md)
retain the exact limited evidence. Real worker execution remains required.

Implementation acceptance must cover:

- OCaml constructors/independent wire bytes, enum/default/old-version rejection,
  and the paired view fixture in both languages.
- Pure geometry with supplied measured widths: first/last/middle and single-column
  cases, reordered IDs, empty overrides, global hiding, multiline sizes, long
  Unicode strings, all alignments, zero flows and very small viewports. Verify
  painted and hit-tested node/ribbon positions belong to the same layout.
- Actual worker measurement with the native text system, captured font changes,
  cancellation, stale-request fencing and bounded retained/temporary storage.
- Root and independently installed OCaml gallery examples with at least three
  columns, short/long/rich labels, inside/outside switching, narrow resize, theme/
  font changes, unchanged raw selection values, updates and cleanup. Read actual
  text geometry/pixels; a two-node-only screenshot cannot prove middle placement.
- Required Rust/OCaml/lint/format and hosted Linux build/unit/consumer checks.
  Real macOS input/render evidence remains distinct from mocked geometry and
  from deferred Linux desktop or VoiceOver acceptance.
