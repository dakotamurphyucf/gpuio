# Rendered-document accessible selection

Implementation plan for the remaining OCH-17/OCH-41 accessibility work. The
native text-projection foundation and request primitive below are implemented; **AX selection publication
and mutation are not implemented or accepted yet**. The [macOS baseline](../evidence/rendered-selection-baseline-och17.md)
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

## Native projection foundation

The Base adapter's `PreparedText::rendered_text()` now prepares an immutable
`Arc<RenderedText>` with the bounded AST, before layout. `TextViewState::rendered_text()`
returns that exact installed identity; ordinary unbounded parser updates clear it.
It does not expose a new OCaml API or publish AX attributes by itself.

- `text()` contains ordinary text, declared custom block glyphs, atomic
  alternatives and structural separators. `PreparedText::plain_text()` retains
  declared whole-document Copy representations, which can differ from custom
  glyphs. The window's Copy adapter separately trims outer plain-text separators.
  Markdown copy preferences do not change selection coordinates.
- `parts()` gives contiguous byte intervals classified as native text owners,
  atomic copy alternatives, or structural separators. Owner references are weak;
  keeping the immutable projection cannot keep a native document alive. Empty
  owner intervals remain represented so a request can clear their old native
  selection without inventing copy text.
- `position(byte)` validates Unicode scalar boundaries, treating CRLF as one
  break. `offset(position)` accepts only a token from that same preparation,
  including when a different document has equal text. These are UTF-8 positions,
  not UTF-16 indices or AccessKit node IDs. Atomic-object mutation policy belongs
  to the next integration stage.
- `selected_fragment_ranges()` reads only current matching local native owners.
  It does **not** recover direction, virtual cross-view coverage, Select All,
  preserved selection overrides or a document-level selection snapshot. The
  complete selection controller must integrate those states explicitly.

Preparation independently bounds logical selection text and declared whole Copy
at 1 MiB + 128 KiB each, with at most 16,384 selection parts.
The generated portion accommodates the extension SDK's existing 1 MiB aggregate
string budget; ordinary source text and structural separators have a further
128 KiB allowance. The previous 128 KiB combined limit incorrectly rejected
already-admitted plugin alternatives. Source/decoration limits remain 64 KiB,
and the SDK independently retains its per-string and aggregate generation limits.
Oversized preparation uses the existing source fallback.
Rich-document workers reserve the projection's bounded maximum before parsing,
then retain only its text/capacity allowance with the prepared result. Source/code
and diff workers do not reserve this unused rich-text allowance. These are
admission units, not measured process RSS or performance acceptance.

The remaining stages above must preserve this distinction: matching whole-copy
text is not enough to infer a native directed selection or to construct a correct
rich accessibility hierarchy.

## Native selection request primitive

`RenderedSelection` preserves distinct anchor/head positions from the same
preparation, including backward and collapsed ranges. `RenderedText::selection`
rejects foreign positions and endpoints inside atomic alternatives. Declared
text blocks remain text, while opaque/non-text block alternatives are atomic.

`TextViewState::prepare_rendered_selection` captures the installed positions and
the view's interaction epoch. `apply_rendered_selection` rejects a foreign/stale
request or disabled selection before mutation, upgrades and locks all participating
native owners, then commits their ranges together. Lock failure and an unmapped
declared-text/copy-alternative pair leave the existing selection unchanged.
Unpainted ordinary runs acquire their prepared text without shaping or parsing.
The window Copy provider reads the same native selection; source-format Copy
continues to use existing Markdown reconstruction.

The state retains the accepted directed range for exact plain-text Copy and
repainting. Copy-format changes do not invalidate it. Native selection gestures,
clear, Select All, disabled selection and prepared replacement expire queued
requests. Compatible native owner transfer can retain the range across a streamed
update only when the logical prefix through both endpoints still matches; the
retained positions are rebound to the new preparation. Ordinary pointer events
do not allocate epoch tokens when no request is outstanding.

A renderer-resource refresh can replace declared glyphs without reparsing the
AST. That path rebuilds both text projections and expires pending requests.
An accepted range is rebound only when logical glyph/alternative text is unchanged
and applying it to the current native owners still succeeds; otherwise its selection
clears.
Thus an empty declared presentation cannot be mistaken for an unpainted run.

This is a low-level Rust adapter primitive, not a new OCaml command or an OS
authorization boundary. The future AX action handler must also validate its
current window, visibility, modality and semantic action identity. The
`requested_rendered_selection()` accessor describes only accepted requests;
the common `rendered_selection()` accessor also represents adopted pointer ranges,
including the local portion of a cross-participant drag and mapped multi-click
gestures. Streamed Select All integration and declared block glyph coordinates
are described below. Whole-document TextRun publication, zero-byte atomic
selection and richer custom-object qualification remain required before general
rendered-document AX acceptance; they are not intentional v1 exclusions.

## Native pointer endpoint capture

The window selection content key now optionally carries a process-unique text
revision and UTF-8 byte offset alongside its existing virtual block key. The
checked revision allocator never reuses an identity; equal-text replacement and
renderer-resource refresh therefore cannot turn an old endpoint into a current
one. Captured endpoints retain neither old document text nor a native layout.

Preparation assigns ordinary inline owners their logical spans. Rich-flow
fragments use their existing canonical owner/local range to resolve those spans
in constant time. A view retains one frame of already-shaped `TextLayout` handles
through `TextSelectionRun` for hit testing, then clears the endpoint map on its
next paint. Immutable directional/grapheme geometry is cached in the corresponding
GPUI element state while text and shaped-line identities remain unchanged;
bounds and alignment update with the frame. Worker-owned parsed nodes retain no
thread-bound layout state. This does not reshape text
or allocate an unbounded endpoint registry on pointer motion. Preparation and
retained admission units include the added owner provenance.

The adapter converts participant content coordinates using the current content
bounds and scroll offset, captures a position only inside a mapped painted run,
and retains it in the window snapshot after that run leaves the viewport. It
does not extrapolate across unknown custom objects or paragraph gaps. A separate
nonvirtual sentinel preserves unrestricted Copy traversal instead of accidentally
restricting such a document to block zero.

`captured_rendered_pointer_selection` exposes mapped pointer ranges. When their
native owners are mapped, a selection-change event now
applies those endpoints to the owners and retains one directed logical range
for native painting and exact plain Copy. This uses the snapshot delivered by
that event, rather than reading a possibly later queued window snapshot. The
window controller continues to own the gesture, participant ordering and
autoscroll; adoption neither ends a drag nor converts it into local Select All.

The retained range distinguishes a pointer gesture from an adapter request.
`rendered_selection()` validates it against the installed preparation and also
represents genuine Select All. The request-only compatibility getter still
excludes pointer ranges. Resize does not clear a valid logical range merely
because its old geometric selection band changed. Compatible append/resource
updates retain its origin and direction while rebinding preparation identity;
replacement, clear and selection-disable still retire it. Exact plain Copy
does not append an unselected block separator, while source-format Copy keeps
native Markdown reconstruction.

This is partial native integration, not a complete accessibility snapshot. Raw
endpoint capture can still succeed when an unmapped owner prevents adoption.
Selection painting now uses the existing shaped glyph-cell background painter;
pointer caret capture and text hit regions use the same directional/grapheme
geometry as ordinary selectable runs. Actual single-row native checks cover
Hebrew, mixed-direction spans, Arabic, combining marks and joined emoji. Wrapped
and aligned rich-selection qualification still needs completion.

Zero-byte atomic selection and broader custom-object qualification remain
required; declared block glyph selection, streamed Select All and held-gesture
rebinding are described below.
An absent common range must not be interpreted as no native selection.
The staged native-owner update currently scans the bounded projection; pointer
hot-path cost and possible coalescing/delta updates need measured qualification.
Rich TextRun publication and guarded OS actions follow that work.


## Cross-participant logical ranges

Window snapshots carry the anchor participant's ordering relative to the cursor
participant, taken from the registered document order. This is independent of
screen Y, reflow and scrolling. Equal means the same document position in that
ordering, not a forward text range; same-participant character direction still
comes from its captured positions. Missing or ambiguous ordering cannot supply
a cross-participant logical range.

The text adapter combines that ordering with `Bounded`, `FromStart`, `ToEnd`
and `Full` coverage. It validates endpoint ownership and current projection stamps
for the endpoint documents; intermediate documents use their complete installed
projection. Forward and backward gestures retain their direction independently
in every participating document. The original window controller still owns scope,
retirement, gesture lifetime and Copy ordering. No selected-string search or
physical coordinate comparison recovers direction.

These ranges use the same staged native-owner update and pointer provenance as
same-document drags. Unmapped custom owners can still prevent adoption; adding
cross-participant coverage does not settle that mapping or the remaining AX
publication/action work.


## Mapped multi-click selection

Double-click maps the current cached shaped-run hit to its actual prepared owner.
The existing native word policy runs on that owner's text, spanning styled visual
fragments; its result expands to whole graphemes. This preserves combining and
joined-emoji sequences. It does not claim full language-aware Unicode word
segmentation: the shared native word policy still uses bounded character-class
scans. Repeated text is addressed by owner and position, never string search.

An ordinary paragraph gesture spans adjacent projection parts through the next
structural separator, including intervening inline objects. Rich-flow triple-click
selects the clicked visual line, combining its mapped text fragments and inline
objects; a wrapped word can therefore be split between visual-line selections.
Object lookup uses a bounded, sorted index of native selection-owner identities.
It costs logarithmic lookup per visible object rather than scanning the entire
projection for each object on every paint. The index retains no extra strong
owners, and both actual and maximum preparation accounting include it.

The result has distinct multi-click provenance, applies the staged native-owner
update, stops the gesture/autoscroll and registers participant-local selection.
It survives compatible reflow/append through the same retained range path.
Clear handlers synchronously retire old owners; a queued empty window snapshot
must not then erase a newer local selection installed by that mouse press.
Explicit clearing, replacement and input policy still retire selections.

Unbounded legacy views and unmapped owners retain their existing native behavior
without publishing a fabricated logical range. Zero-byte atomic alternatives need
an explicit object-selection representation: a collapsed text range would erase
the visible object's selected state. That integration and broader custom-object
qualification remain required before complete AX acceptance.


## Streaming and native frame identity

Genuine Select All becomes a frozen directed range when a compatible append
arrives. Its provenance stays distinct from pointer, multi-click and explicit
adapter requests. Source-format Copy retains the original source snapshot;
plain Copy uses the current mapped native range. Newly appended content is not
implicitly selected.

Structural separators require affinity to their original content edge. If a
terminal synthetic separator moves or becomes newly appended owned text, its
endpoint clamps to the old content edge. If it remains the same structural
separator, the offset stays. Owned code linebreaks are never trimmed by this
rule. Endpoint mapping validates preparation identity and the unchanged prefix;
it does not search for the selected string. Separator-only or empty selections
can become collapsed ranges with no active local Copy.

The window controller also rebinds captured anchor, cursor and pending extension
positions when a participant installs compatible prepared text or refreshes
compatible renderer resources. All endpoints are staged before mutation. The
controller retains gesture, virtual block keys and autoscroll ownership. Failed
mapping or incompatible replacement cancels the old window gesture, including
other participants. Cleanup runs outside the window-state lease; the initiating
view resets its own native owners instead of invoking its own clear callback
while its lease is held. There is no deferred unguarded cleanup that can erase a
subsequent local selection.

Painted inline text and object callbacks carry the installed bounded projection
revision. A callback from an obsolete frame rejects the press before mutating
owners or entering the legacy copied-string fallback. The guard also checks
selection policy. It is separate from the selection-request epoch, since an
ordinary mouse-down intentionally clears previous selection before handling the
current press. This does not replace the owner/window/visibility/action guards
required by future accessibility actions.


## Declared custom block glyphs versus whole-document Copy

The existing document SDK contract gives declared block `Text` to the reader's
ordinary partial glyph selection. Its `MarkdownNode.text` can independently
specify whole-document Copy. Selection coordinates therefore follow the declared
glyphs, including an explicitly empty presentation; Copy alternatives are not
fabricated as invisible selectable characters. Partial plain Copy uses the selected
glyphs. Whole-block Source Copy uses declared Markdown, while partial Source Copy
falls back to the selected glyphs when no character mapping exists.

Genuine Select All, and an explicit request covering the entire document, retain
the existing whole-document plain/source Copy policy. A compatible append freezes
that old scope: its logical range follows the old glyphs; a separate immutable
whole-Copy snapshot retains a differing declared representation. Ordinary matching
Copy continues to use the logical range and its structural-separator affinity.
Selecting a new range or clearing the selection discards the old Copy snapshot.
A whole-Copy scope can remain active even with zero displayed characters; this
adds no synthetic characters to the logical text.

Preparation counts logical bytes and declared Copy bytes independently, including
their different separator requirements, without allocating a second full Copy
string just to check its size. An empty visual presentation cannot admit an
oversized alternative or evade aggregate limits. Neither source admission nor the
SDK's generated-string allowance is reduced.

This addresses reader-owned custom block glyphs. Inline custom objects retain
atomic selection, including when their presentation declares Text. Selecting an
empty-copy atomic object still needs explicit ordered object edges; a collapsed
byte range cannot represent that selection. Opaque/NonText block interaction,
custom-object scope/virtualization and rich AX publication remain separate
requirements.
