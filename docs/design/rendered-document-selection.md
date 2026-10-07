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
are described below. Whole-document TextRun publication, opaque block
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
without publishing a fabricated logical range. Empty inline alternatives now use
the explicit object edges described below; a byte interval alone cannot represent
their selected state. Opaque blocks use the wrapper described below; broader
custom-object qualification remains required before complete AX acceptance.


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
whole-Copy snapshot retains a differing declared representation. A snapshot is
also retained when append rebinding moves an old terminal structural separator outside the new logical range. Comparing only the old Copy
and old projection is insufficient: compare against the rebound selected text.
Thus exact Copy stays frozen while paint excludes newly appended characters.
Selecting a new range or clearing the selection discards the old Copy snapshot.
A whole-Copy scope can remain active even with zero displayed characters; this
adds no synthetic characters to the logical text.

Preparation counts logical bytes and declared Copy bytes independently, including
their different separator requirements, without allocating a second full Copy
string just to check its size. An empty visual presentation cannot admit an
oversized alternative or evade aggregate limits. Neither source admission nor the
SDK's generated-string allowance is reduced.

This addresses reader-owned custom block glyphs. Inline custom objects retain
atomic selection, including when their presentation declares Text. Empty-copy
inline objects use the ordered edges described below; a collapsed byte range alone
cannot represent that selection. Opaque/NonText blocks use the native wrapper
described below. Broader custom-object scope/virtualization and rich AX publication
remain separate qualification requirements.

## Empty inline-object edges

The native position model additionally orders zero-byte inline objects at a UTF-8
byte boundary. An endpoint contains the immutable preparation identity, byte and
object-boundary slot. This distinguishes selecting an object with an empty Copy
alternative from placing a caret beside it; no U+FFFC or other placeholder is
inserted into Copy. Slots are checked against the current bounded projection and
are not OS UTF-16 indices or public OCaml protocol offsets.

`position(byte)` addresses the canonical edge before empty objects at that byte.
`selection_for_part(index)` returns a specific owner's checked edges, and
`full_selection()` includes trailing empty objects, even in an all-empty document.
Use `is_collapsed()` to test semantic collapse. `bytes()` and the diagnostic
`selected_fragment_ranges()` retain real byte intervals and cannot independently
identify which of several empty objects was selected.

Pointer endpoints, multi-click, cross-participant coverage and explicit native
requests preserve those edges. The inline object owns its highlight and Source
Copy; a passive glyph child does not register a second text selection area.
Structural boundaries remain present even where paragraphs contribute no Copy
bytes, so paragraph selection cannot absorb a preceding empty-object paragraph.

Append/resource rebinding checks ordered empty occurrences using a weak projection
identity or matching source-span/name/Markdown metadata after the caller's existing
compatible source/AST-transfer checks. Matching empty strings is insufficient.
Projection/fragment and metadata storage is charged in preparation accounting;
no strong parent AST or native view is retained by an endpoint.

Both inline and opaque/NonText block objects now use these ordered edges; block
objects delegate their actual native layout through the wrapper below. Broader
reflow/virtualization qualification, selection performance and rich AX remain
required work.

## Opaque and NonText block ownership

A custom block with Opaque or NonText presentation is one selectable object.
`BlockObject` delegates layout and prepaint to the native child, registers its
whole bounds and checked owner edges, and draws the selection wash after child
content. No synthetic glyph or character-level mapping is invented for arbitrary
Rust rendering. Empty Copy alternatives remain selectable through ordered edges;
Source Copy uses the selected occurrence's declared Markdown.

`BlockSelection` distinguishes glyph-owned Text from Object(selected). Changing
renderer resources cannot leave an old whole-object flag shadowing partial glyph
Copy. Rebinding selected atomic blocks additionally requires compatible occurrence
identity/source metadata; equal Copy alternatives alone cannot preserve selection
when an append changes the selected block's meaning.

Native child semantic nodes remain under the block container. Button, checkbox,
radio, toggle, switch, slider and link pointer handlers suppress parent text
selection without stopping activation or focus propagation. Input keeps its own editor
selection and typing. Custom interactive renderer authors must similarly claim
pointer selection through `GlobalState::suppress_text_selection` in their native
mouse-down handler (or their established editor interaction adapter). This is a
Rust extension-author contract, not a synchronous OCaml callback. A semantic role
alone does not opt an arbitrary element into that interaction policy.

Targeted qualification includes repeated empty/nonempty blocks, both directions,
background double/triple clicks, resize, renderer-kind changes, unchanged appends,
replacement cancellation and unpainted multi-block requests with weak-owner
cleanup. Tests of a native child Input's focus/typing do not establish its complete
AX adapter. The block container is not rich TextRun/AX selection publication;
that work and broader physical/virtualized/reflow/performance acceptance remain
required.

## Accessibility text-root boundaries

The pinned AccessKit consumer traversed into nested text inputs when collecting
an ancestor Document's text runs. A standalone probe and a before/after consumer
regression demonstrate that `before` + embedded `editor` + `after` became one
parent range. That conflicts with the independent native selection owners.

The scoped [consumer adaptation](../../vendor/accesskit-consumer/GPUIO.md) stops
text-range traversal at nested text inputs, Documents and Terminals. It preserves
their semantic children, actions and their own text ranges. Labels, links, headings
and table/cell structure remain in their owning document's reading order; lack of
painted geometry does not remove a run. This does not by itself publish GPUIO's
rendered selection or authorize any OS action.

The same probe establishes another conversion constraint: two empty TextRuns
produce a degenerate AccessKit range with identical UTF-16 offsets. Distinct native
empty-object edges therefore cannot be exposed merely as two empty runs. The prepared
coordinate adapter below supplies a reversible accessible-object representation
without inserting placeholders into clipboard text, source or public protocol
offsets. Full rich AX publication and native qualification remain unimplemented.

## Prepared accessible coordinates

The native projection now prepares an accessibility coordinate index beside its
existing selection owners. Each logical part has an opaque identity tied to the
exact preparation. Styled or wrapped TextRuns may later split a part, but must
map back through that identity; a repeated string or an old ordinal is insufficient.
These are logical parts, not a second published semantic document.

An empty atomic alternative contributes one U+FFFC **only to accessible text**.
Its character edges map reversibly to the native object's ordered boundary slots.
Copy, source text and public protocol offsets remain unchanged. Nonempty atomic
alternatives remain readable, but requests inside them are rejected because the
native widget has no corresponding partial selection. An empty document has one
zero-character part for its caret.

The index distinguishes UTF-8 bytes, scalar/CRLF character indices and global
accessible UTF-16 units. Surrogate interiors, mid-CRLF positions, foreign part IDs
and atomic interior endpoints are rejected. Shared boundaries canonically prefer
the following part; adjacent empty objects retain distinct positions. Logical
coordinates do not depend on the visible viewport or shaped geometry.

ASCII parts allocate no character table. Other parts store one encoded byte per
character and byte/UTF-16 checkpoints every 64 characters. Conversion uses binary
search and at most 63 intervening widths, without rescanning a whole document or
reshaping text. The existing preparation reservation and retained accounting
include the index; string admission limits are unchanged.

This implements coordinate conversion only. Semantic TextRun publication must
preserve the actual hierarchy and native children, split runs where needed for
line/directional geometry, and attach only current shaped bounds. OS action
handlers must still check owner/window/visibility/modal/input/generation policy
before applying the existing native selection request. Actual platform/VoiceOver
acceptance remains outstanding.

## Prepared structural ownership

`RenderedText` also carries a preorder arena of semantic owners prepared during
the existing logical-text traversal. Headings, paragraphs, blockquotes, lists,
list items, code, tables/rows/cells, frontmatter terms/definitions and custom
blocks retain their ancestry before any block is laid out. Table cells record
row/column indices and header status. Separators belong to their structural
parent rather than being appended to a neighboring cell's text.

Each `RenderedSemanticId` belongs to one exact preparation. Foreign IDs are
rejected even when the text matches; keeping an ID or the arena retains no AST
or native view. Direct-child iteration skips whole subtrees. Structural nodes
address contiguous logical parts, while links additionally retain exact byte and
empty-object edges, allowing several links within one native inline owner.
Known source identity coalesces styled pieces of one link; equal destinations
alone do not merge separate links. Reference URLs and titles use the renderer's
current `NodeContext` resolution, including resource-refresh preparation.

The arena has a 32,768-node bound. Preparation reservation and retained accounting
include node capacity and unique shared URL/title bytes; repeated references do
not copy a long destination per occurrence. Existing source/generated-text
admission limits are unchanged. This is structural metadata for the future
publication adapter, **not** an additional hidden accessibility tree or evidence
of OS behavior. Native control attachment, TextRun splitting/geometry, final-frame
selection publication and guarded actions still require implementation and actual
platform qualification.

## Real block subtree attachments

The renderer now binds prepared top-level block owners to their real native
accessibility subtrees. `semantic_block` addresses the original parsed block
slot, including empty slots for definitions/rules; ignoring a non-text block
cannot shift the next owner's identity. A transparent element scope delegates
the original layout, prepaint and paint without replacing headings, links,
table cells, native controls or their actions. Its element key uses the stable
block slot, never the preparation identity that changes during updates.

`TextView` already contains a native `Document` element named `Document content`.
Its synthetic-child callback filters recorded scope IDs against that element's
actual final children. Virtual-list measurement and rolled-back prepaint can
record candidates, but only nodes present in the resulting document subtree
become attachments. Multiple surviving candidates for one logical owner are
rejected rather than selected arbitrarily.

The attachment frame holds a weak projection reference and plain IDs. It resets
at each TextView prepaint and records its window. The native
`rendered_semantic_attachments(window)` accessor returns the last completed
prepaint's bindings only for the exact installed preparation and window.
Replacing text rejects an old frame before redraw. Offscreen logical owners
remain in the prepared arena, with no invented native subtree or geometry.

This is a read-only mapping, not an action capability or a promise that an owner
remains visible after a later hide/unmount. Publication must use it within the
current document frame; OS actions must independently validate current tree,
owner, visibility, modality, input policy and interaction generation. Nested
TextRun placement, unpainted logical text, shaped character geometry and
post-paint selection publication remain required integration work. No complete
rich-document accessibility or VoiceOver acceptance follows from block bindings.

## Native text-run publication (integration in progress)

Laid-out Inline labels and link proxies now publish TextRun children beneath
their existing semantic nodes. The rich-flow collector carries those runs into
coalesced links without replacing native controls or their action owners. This is
partial leaf publication; complete Document text, selection and actions are not
yet exposed by this integration.

Inline prepares its frame-owned `TextSelectionRun` during prepaint and reuses it
for pointer registration during paint. Accessibility derives geometry from the
same cached shaped clusters, including current bounds and alignment. It does not
shape a second copy of each character. TextRuns split at line/direction changes,
discontinuous geometry and transitions to missing geometry. Scalar character
lengths treat CRLF as one character. Ambiguous overlapping clusters retain their
text without an invented rectangle.

Hard line breaks omitted from rich-flow glyph layout are explicit logical
fragments, addressed by the original text item and byte offset. They retain the
actual source link membership and publish a newline without glyph geometry or
focus-outline fragments. Their source-order slots exist with accessibility both
on and off, avoiding an activation-dependent change to later element keys.

The native regression covers combining characters, emoji, RTL text and a long
styled link with a hard break and soft wraps. Existing pointer fixtures address
the owning labels rather than also counting their new TextRun descendants.
Neither those tests nor leaf publication qualify OS selection. Remaining work
includes semantic separators and atomic alternatives, complete offscreen logical
content, visual-line adjacency, final-paint selection publication, guarded OS
actions and actual platform tests.

Published run IDs now incorporate the exact prepared fragment's endpoints.
Replacing equal text retires those IDs while retaining native control identities.
Ordinary text, rich-flow pieces and hard line breaks supply their canonical
native owner and source interval; repeated strings are never used as identity.
Same-part coordinate conversion resolves an interval's end against its owning
part rather than accidentally choosing the following separator.

The Document's final prepaint callback filters candidate run bindings through a
read-only traversal of its actual completed descendants. Measurement-only rows
and unrelated sibling subtrees cannot become addressable runs. Per-node character
lookup and a sorted source interval index provide bidirectional conversion through
the prepared scalar/CRLF coordinate index. Other windows, preparations and invalid
character indices are rejected. Unrealized logical text still has no native run
at this stage; complete offscreen publication remains required.

These conversions describe the last Document prepaint, not an action capability.
Ancestor clipping can subsequently hide nodes; later unmount, modality, input
policy or interaction changes can also invalidate an action. The eventual OS
handler must validate those conditions against current native ownership.

## Logical text for unrealized blocks (integration in progress)

The existing Document now merges prepared top-level semantic owners with its
actual native attachments. Realized blocks keep their original subtrees and
control actions. Unrealized blocks receive structured logical descendants,
including headings, lists, table rows/cells and links. Their text comes from
prepared part intervals, including structural separators and empty-object U+FFFC
alternatives; it is never recovered by matching repeated strings. An explicit
work stack handles semantic nesting without input-sized call-stack recursion.

Logical TextRuns split at hard line breaks, preserve CRLF as one AccessKit
character, and have no fabricated bounds, character rectangles or input handlers.
Native-widget alternatives can contain multiple readable characters, but only
an object's outer edges are valid native selection positions. The run index uses
global accessible UTF-16 offsets so that reading positions inside such an
alternative do not collapse onto the same native byte/slot pair. Empty documents
have a zero-character caret run. Clamped previews do not expand into the full
logical document.

This supersedes the earlier absence of offscreen runs. It does not complete the
Document contract: visual-line adjacency, final-paint selection, guarded
reveal/activation, OS selection actions and comprehensive clipping-policy
acceptance remain required. Offscreen link
roles and URLs do not imply an implemented activation handler. Actual macOS
accessibility and VoiceOver acceptance remain separate from TestPlatform coverage.

Publication collects each synthetic owner's direct children separately before
using the existing subtree builder. Completed roots are merged with native roots
once, avoiding repeated scans of earlier siblings. Selection endpoint validation
uses binary searches over ordered, nonoverlapping parts to reject atomic interiors;
this also avoids a full part scan for every semantic range. Native IDs, actions,
empty-object boundaries and foreign-position checks retain their contracts.
The [publication diagnostic](../evidence/rendered-publication-cost-och17.md)
records the measured debug improvement and its limits. Optimized workload and
physical presentation qualification remain separate requirements.

## Completing visible native scopes (integration in progress)

Prepared semantic nodes retain original nested block slots, including slots with
no semantic owner. `NodeRenderOptions` carries the current owner without cloning
the link-reference map or using a prepared revision as a native element key.
Nested blocks, native table cells/rows and description-list scopes complete
missing text during their synchronous prepaint callback. Actual descendant runs
and completed scopes identify each direct child's text extent; missing intervals
are inserted only after checking ordering and containment. Native controls retain
their own nodes and actions. Already finalized descendant nodes are not mutated,
so the builder's postorder and rollback contracts remain intact.

Blockquote and list continuation paragraphs use distinct original sibling indices.
Root publication treats completed native scopes as represented, preventing duplicate
boundary separators. Empty semantic blocks still provide a document caret, and
clamped previews do not acquire complete-document text. The extra prepared slot
storage is included in retention and admission accounting. See
[native scope evidence](../evidence/rendered-native-scopes-och17.md) for the
reproduced failures, coverage and remaining limitations. This does not authorize
actions from last-prepaint coordinates or establish OS accessibility acceptance.

## Reading inline atomic alternatives

Native text snapshots distinguish a shaped glyph fragment from an atomic reading
part. A reading binding carries the exact prepared part ID and character range;
its TextRuns may split a multiline alternative without granting native selection
inside the object. The existing checked conversion still permits only atomic
edges, including distinct edges for adjacent empty objects represented by U+FFFC.
No character bounds are inferred from an arbitrary widget's rectangle.

Linked objects publish through their existing logical link, while unlinked objects
publish beneath their actual native wrapper. Each snapshot has one owner; custom
controls retain their native identities and actions. Equal-text replacement retires
reading IDs independently of control IDs. The
[inline-object evidence](../evidence/rendered-inline-objects-och17.md) includes
action/coordinate tests and a horizontal-table reading check beyond the viewport.
Readable offscreen text is not permission to dispatch actions to an offscreen
control, and these bindings remain separate from final-paint selection publication.
