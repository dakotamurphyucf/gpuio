# Rendered-document accessible selection

Implementation plan for the remaining OCH-17/OCH-41 accessibility work. This is
**not an implemented API or acceptance claim**. The [macOS baseline](../evidence/rendered-selection-baseline-och17.md)
proves that native rendered-text copying works while document-level AX selection
attributes are absent. The source-editor adapter is a separate implementation.

## Required behavior

A rendered Markdown or HTML document must expose its current native selection to
accessibility clients and accept a valid accessible selection request. The
selected accessible text is rendered plain text, even when the application chooses
Markdown as its clipboard format. Changing that copy preference must not reinterpret
the selected range or publish source syntax as the accessible value.

Keep the existing document, heading, list, table, cell, link, image and custom
control structure. TextRun children belong to the corresponding rendered text
owners; do not replace that hierarchy with a second hidden source document or
remove native custom controls to make range queries easier. Link names/actions
must remain unique across styled or wrapped fragments.

Selection addresses the currently installed, bounded prepared document. A pending
parse must not publish a new source with old selection indices. Scrolling or
virtualization must not silently narrow the accessible selection to the last
painted blocks: a native Select All can include text outside that viewport.
Actual character geometry may be absent off-layout; this does not make readable
text or its logical selection disappear. Collapsed or retired bodies must not
remain actionable.

## Native data and ownership

Draft the final Rust types against the implementation before editing the renderer:

- An immutable **selection projection** belongs to one installed prepared
  document. It maps logical rendered text, structural separators and atomic
  selectable objects to the existing native selection owners. It shares bounded
  text allocations and retains no history, window or strong parent View handle.
- A **position** identifies a projection/run and a Unicode scalar boundary.
  Scalar-addressable OS ranges remain distinct from grapheme-aware keyboard
  selection. Byte offsets, scalar indices and macOS UTF-16 indices must have
  explicit checked conversions; CRLF and empty endpoints need defined behavior.
- A **selection snapshot** contains directed anchor/head and the projection
  identity. Native pointer/keyboard selection publishes this snapshot from the
  same state used for Copy. Accessibility must not infer a range by searching
  for the selected string, which is ambiguous for repeated text.
- A **request stamp** binds an AX action to the actual owner, prepared document
  and relevant interaction generation. Deferred requests validate all three
  before applying any mutation. Text-run identity must prevent an old node ID
  from selecting different text after streaming, reflow, reset or remount.

`DisplayedText` is a search/decoration projection, explicitly not the copy/AX
representation. Reusing its fragment list verbatim would lose separators,
image alternatives and custom-object semantics. Likewise, `selected_text()` alone
cannot reconstruct directed endpoints. Reuse underlying owner identities and
source spans where correct, with explicit adapters for the different projections.

An accepted AX request updates the existing native selection owners, invalidates
only the required paint/accessibility state and uses normal native focus/reveal
policy. Copy must read that same selection. No synchronous OCaml callback, parse,
I/O, per-frame OCaml update or second selection authority is permitted.

Prepared semantic structure and frame geometry need separate lifetimes. Keep
logical reading order stable when visual fragments rewrap or scroll. Reconcile
existing link/custom-control semantic children with text runs; do not publish
both the old text value and an independently announced duplicate subtree.
A source reset invalidates the projection; an append can retain selection only
when the mapped owners and selected text genuinely remain compatible.

## Admission and failure behavior

Reject foreign/stale run IDs, invalid boundaries, malformed/out-of-range
positions and closed/hidden/modal-blocked owners without changing selection.
Directed backward selection itself is valid. `User_select=false` must reject
external selection requests as well as native selection/Copy. Read-only rendered
content remains selectable but never advertises value replacement. Embedded
editable controls retain their own privacy, focus and selection policy.

Bound the prepared projection and accessible text/run counts using documented
preparation/admission budgets. Large-source fallback remains the existing paginated
source presentation. Do not silently truncate a valid small document's accessible
selection to satisfy a new arbitrary limit. Geometry uses already-shaped native
text; avoid one reshape or whole-document scan per requested character.

## Implementation and qualification sequence

1. Specify and test projection/position conversion against actual parsed blocks:
   paragraphs, formatted/link spans, lists, tables, fenced code, images and declared
   Text/NonText/Opaque custom objects. Cover duplicate text, empty text, CRLF,
   combining marks, joined emoji and bidirectional logical order.
2. Integrate directed native selection snapshots and guarded selection application.
   Verify Copy, source-format Copy, select-all, partial selection, streaming,
   viewport changes and source replacement share the same owners.
3. Add TextRun metadata to the existing semantic hierarchy and publish document
   selection. Preserve unique links, heading levels, table/cell structure and
   interactive children; test before/after virtual realization and stale actions.
4. Extend the real macOS probe into a required integration check: exact selected
   text/ranges, AX-set partial selection followed by native Copy, backward keyboard
   selection, offscreen select-all, source-format distinction, independent windows,
   collapse/reset/remount and teardown. Add root and independently installed
   consumer evidence. Keep actual VoiceOver navigation/announcement separate.

Only completed stages receive implementation/acceptance claims. The baseline
below demonstrates the missing behavior; it does not validate this design or
close either milestone ticket.
