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
