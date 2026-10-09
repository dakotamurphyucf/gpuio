# Document accessibility

Release contract for OCH-17. Implementation and validation progress belongs in
the evidence ledger; this document does not claim completed screen-reader support.

Read-only code, diff and source pages expose the displayed page as a labelled,
read-only multiline text input. Its value is the editor's installed text, including
Unicode and diff markers. It offers focus and native selection/copy, but no
accessible value replacement. The existing bounded source pagination applies to
accessible values too; the toolbar announces the displayed byte range. Changing
pages changes the value rather than materializing the entire source in the
accessibility tree. Source offsets remain UTF-8 bytes; OS text APIs use their own
documented native units.

The installed native page supplies logical-line TextRun children and its current
selection. Each run uses selectable Unicode scalar boundaries, with CRLF treated
as one line break, matching the editor's explicit selection operations. Keyboard
movement/deletion retains the editor's separate grapheme policy. Empty text and
a trailing newline have an empty final run for the caret. The native bridge
preserves anchor/head direction; macOS reports an ordered UTF-16 range.

Accessible selection changes target those current run identities. They reject
foreign or stale runs, changed text/revisions, active IME composition, retired or
hidden source presentations and a closed interaction gate. Selecting a read-only
page does not grant permission to edit it. The same adapter supports ordinary
plain-text inputs and textareas; hidden and revealed passwords publish neither
text runs nor accessible selection actions. No synchronous OCaml callback or
whole-source materialization is involved.

The [source-editor geometry adapter](editor-accessibility-geometry.md) now supplies
same-prepaint shaped range/caret rectangles, wrapped visual runs and directional
line links. It keeps logical selection order and omits unavailable off-layout
geometry; word-navigation metadata remains separate. Geometry and selection
queries do not establish complete VoiceOver reading or tracking. Rendered Markdown
has a separate selection projection and is not covered by this source-editor
adapter. Its [rendered-selection contract](rendered-document-selection.md) now has
[native dispatch and actual macOS range/Copy evidence](../evidence/rendered-selection-dispatch-och17.md),
including the later cache-lifecycle and retry qualifications linked from
[the current status](../status.md). Those results do not establish rendered
complete visual-line geometry or VoiceOver reading/tracking acceptance. The later
[rendered line qualification](../evidence/rendered-visual-lines-och17.md) links
source-adjacent runs on the same painted line within a semantic flow, with
bidirectional current-frame relationships. It covers native style/link/Unicode
fragmentation, table-cell boundaries and reflow, plus actual macOS paragraph
line/range and character-point queries. Missing layout still supplies no invented
geometry; broader rendered geometry and screen-reader acceptance remain separate.

Rendered Markdown exposes text from the actual painted nodes in reading order,
with headings, lists, table structure, image alternatives and links. Do not add a
second hidden copy of the raw Markdown as a substitute for these semantics.
Accessible link activation must use the same queued document-navigation event as
pointer activation, without ambient URL opening. Links also need keyboard access
and visible focus. Selection and copying continue to refer to the rendered text.
Parsing and accessibility remain bounded by the document preparation limits;
oversized documents use the existing paginated source presentation.

Markdown headings retain their parsed level (1–6); the macOS heading's numeric
AXValue exposes that level while painted child text supplies its content. Markdown
tables expose one table with row/column counts, ordered rows, and header/data cells
with zero-based row and column indices. The first parsed row is the header. Each
row has a distinct identity within its table; cell text remains in the actual
painted children. Both wrapping and horizontal-scroll layouts use this structure.
Copy/action controls remain outside the table's data hierarchy. These are read-only
document tables, so the semantics do not advertise row selection or editing.
On macOS, AXColumnHeaderUIElements and AXRowHeaderUIElements return the current
painted header nodes in reading order. AXHeader identifies their nearest shared
row/group container when one exists. A nested table's headers belong to that
table, and hidden/retired nodes are excluded. These queries neither prepare source
text nor materialize offscreen rows or columns. The same mapping applies to the
managed table; clients correlate header and cell column indices.

The rendered document owns keyboard focus. Tab and Shift-Tab move through its
logical Markdown links in source order; moving past either end returns traversal
to the containing widget. Enter activates the selected link, Escape returns to
document focus, and pointer interaction clears keyboard link selection. Text
selection remains separate. Links have source identities shared across styled
fragments; an ordered catalog includes offscreen blocks. Navigation reveals the
block and requests autoscroll to the actual link position, including inside a
block taller than the viewport. This is a one-shot request, not a permanent scroll
lock. Unchanged links can retain selection through an append; reset or a changed
target clears it. Only the active link claims accessible descendant focus.

A direct accessibility Focus request selects that logical link and uses the same
document focus owner and one-shot reveal. It does not activate the destination.
Enter remains a separate navigation action. The target's prepared source identity
and URL must still match. Before changing focus, the native host checks the
installed presentation, live source generation, rendered mode and current
window/visibility/modal input scope. This predicate is native-only; it neither
calls OCaml nor introduces one native focus handle per link.

Collapse, unmount and inactive modal scopes remove or disable interaction with
document bodies. Accessible actions must validate the current native presentation
and focus scope, including delayed actions after replacement. Streaming preserves
the last installed reading tree while preparation is pending and replaces it
when the matching result is installed. No accessibility callback synchronously
invokes OCaml or parses the source.

Acceptance needs actual macOS accessibility queries for values, structure and
actions, native selection/copy and keyboard checks, and streaming/collapse/remount
regressions. AX-tree presence alone is not evidence of complete VoiceOver reading
or native selected-text range support. Linux desktop acceptance remains OCH-47.

Rich paragraphs group adjacent painted fragments by logical link source identity,
not by font, line, URL alone or image-loading state. A frame-local collector uses
the existing shaped text and atomic-object bounds; it does not reshape or retain
document history. One accessible action uses the prepared link name, unioned
bounds and the existing guarded navigation callback. Keyboard reveal targets its
first fragment, and focus outlines follow its individual painted fragments.
Distinct links to the same destination remain distinct actions.

Ordinary text and native custom children retain source reading order. Linked
plain-text object fallbacks use their alternative name through the link instead
of duplicating that text as a sibling. Custom native controls keep their own
semantics and interaction; linking an atomic object does not hide its interactive
descendants. This grouping changes neither selection projections nor asset I/O.

The native safe-image plugin gives a decoded, unlinked image the Image role and
its Markdown alternative text. Empty or whitespace-only alternatives are
decorative: the image may paint but has no separate accessible target. A linked
passive image contributes its alternative to the enclosing logical link exactly
once. If that link has no alternative text, its destination supplies the name.
Missing named images retain the safe visible `[Image: alternative]` placeholder;
missing decorative images remain silent. Reference-style images use the same
explicit registered-asset mapping as inline destinations. Neither form loads a
URL or file implicitly.

Inline renderers opt in to hiding a passive accessible subtree when linked.
Custom interactive children remain exposed by default; the image policy must
not hide their controls. An explicitly empty custom text representation stays
empty when the parser attaches source metadata; only an unspecified text value
defaults to source syntax. This preserves intentional decoration without
announcing raw Markdown as an alternative.
