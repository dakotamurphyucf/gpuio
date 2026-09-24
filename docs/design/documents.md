# Revisioned display documents (OCH-14, implementation in progress)

OCaml owns canonical content; Rust owns display copies, parsing, layout and
selection. A document outlives its mounted views and belongs to an application
or conversation scope. Offscreen row eviction releases presentation resources,
not the stream. Views refer to a resource lease, never a Rust pointer.

`Gpuio.Text_source` is an immutable UTF-8 snapshot with shared bounded chunks.
Appending copies at most one 16 KiB tail chunk; it does not flatten old content.
Edits/replacements may rebuild the document. Snapshot identity and generation
are opaque allocation identities, not user-provided revision numbers. Runtime
registration assigns monotonically increasing wire revisions against the last
accepted snapshot. Independent branches cannot accidentally share a revision.
Reset creates a new generation. Terminal status is separate from the bytes and
survives coalescing; appending after completion/cancellation requires reset.

Wire delivery coalesces to the latest snapshot, keeping the accepted baseline
and one desired snapshot. A changed suffix is staged in bounded messages and
published atomically after validation. Staging never exposes half a Unicode
scalar or partial replacement to a widget. Stale acknowledgements and parser
results cannot overwrite newer generations. Multiple views share a registration.

Initial explicit budgets: 8 MiB UTF-8 source per document, 16 KiB canonical
chunks, 256 KiB maximum update message payload. Additional native aggregate,
parser concurrency, rich-render expansion and cache budgets will be fixed and
measured before acceptance. Huge source retention and bounded rendering are
separate contracts; transcript row virtualization alone is insufficient.

Markdown must be parsed as complete snapshots for semantic correctness: later
reference definitions and incomplete fences can change earlier interpretation.
The pinned Base renderer is a reuse candidate, but its unbounded queues,
tail-only append parsing and measure-all list are not GPUIO contracts. Parser
work must be bounded and stale results rejected. No synchronous OCaml callback
is permitted during native layout or paint. Images use explicit registered
assets; link navigation is delivered as a typed application event.

Selection uses document generation and UTF-8 byte boundaries. Append preserves
existing ranges; overlapping correction invalidates a selection, and reset
clears it. Copying source ranges is independent of viewport membership; copying
rendered Markdown selection follows the renderer's explicit plain/source mode.
Cross-document transcript copy remains an application data operation.

This document is a design checkpoint, not completion evidence. Public widget
signatures, measurements and exact tested limits will be recorded as implemented.

## Syntax implementation

Pinned Syntect 5.3.0 with Two Face 0.5.2+bat-0.26.1 supplies grammars using the
pure Rust regex engine. The actual bundled grammar lookup is tested for OCaml,
Rust, shell, JSON, Python, JS/TS, YAML, TOML and INI. Syntect's smaller default
bundle omits TS/TOML/INI; substituting JavaScript coloring was not selected.
[Syntect API](https://docs.rs/syntect/5.3.0/syntect/parsing/struct.SyntaxSet.html)
and [Two Face](https://docs.rs/two-face/0.5.2+bat-0.26.1/two_face/).

Highlight work is background-only, bounded to 256 KiB, 16 KiB per line and 32768
runs. Cancellation is checked between lines; these are input/work admission
bounds, not a hard wall-clock deadline for one regex. An exceeded limit produces
plain text, without partially coloring a misleading prefix. Grammar/theme
bundles load once. Native scheduling, presentation and measured costs remain
under implementation.

## Current native presentation (under acceptance testing)

`Gpuio.Document.Config` selects Markdown, a code language, or unified diff;
`Appearance.Light/Dark` selects syntax colors explicitly. `View.document`
uses a borrowed registration handle and an optional typed navigation effect.
Native clicks carry the resource and generation as well as the node/handler
identity. OCaml rejects queued navigation immediately after reset/release.
Document registration may start during application initialization; the transport
pump waits for the negotiated welcome before creating native resources.

Two background workers share a latest-request-per-view scheduler (128 live
views). Admission accounts for at most64MiB of conservative parse/cache units;
these units are not measured allocator RSS. Source snapshots have a separate
64MiB aggregate reservation budget. A superseding request cancels old work;
results only install into the matching request serial. Native shutdown waits
for the two worker fences. No idle document polling is installed.

Rich Markdown is limited to64KiB, 4096 AST nodes, depth32, 256 top-level blocks
and16KiB lines. Exceeding a parser/highlighter limit retains the full canonical
source and shows an explicit source fallback. Source pages contain at most64KiB,
1024 lines, with long lines split into16KiB segments on Unicode boundaries.
The source byte interval is visible; gutter numbers retain original line origins.
Previous/next pages do not modify the canonical source. Copy source always
copies the entire source. Native editor selection is page-local in this explicit
huge-document mode; the source API remains available for application-owned ranges.

Literal full-source search runs over rope chunks in the worker using a bounded
KMP matcher. It counts all non-overlapping matches and retains at most4096
navigable ranges. The UI labels that limit. Navigating a match opens its source
page; Markdown can switch back to rendered content. Markdown raw HTML is literal
text, and images resolve only through registered decoded assets. Source copy,
rendered Markdown selection, fenced-code copy and table-source copy are distinct
native operations.

Markdown selection transfer uses logical UTF-8 ranges when the selected text
prefix is still compatible after a full reparse. Select-all is a snapshot: later
streaming bytes do not enter the selection. A generation reset or incompatible
correction clears it. Focused/selected document rows participate in the managed
list retention guard. Offscreen presentation eviction cancels parser work while
leaving conversation-owned source streaming alive.

Local macOS native tests currently cover actual GPUI layout/paint (background
window), Unicode code selection under append, Markdown/table/fence/image fallback,
select-all during streaming, diff preparation, huge-source search, and source
lease teardown. They are not yet a complete keyboard/accessibility/physical
presentation acceptance claim; Linux acceptance and measured costs remain pending.
