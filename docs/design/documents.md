# Revisioned display documents (OCH-14)

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

Source budgets: 8 MiB UTF-8 source per document, 16 KiB canonical
chunks, 256 KiB maximum update message payload. The native store also bounds aggregate charged snapshots/staging to64MiB,
with at most8 staged uploads. Parser/view limits are specified below. Huge source retention and bounded rendering are
separate contracts; transcript row virtualization alone is insufficient.

Markdown must be parsed as complete snapshots for semantic correctness: later
reference definitions and incomplete fences can change earlier interpretation.
The adapted Base renderer accepts externally prepared snapshots. GPUIO replaces
its original parser task/queue path with the bounded worker service below; the
original tail-only append parser is not used. Stale work is discarded. No synchronous OCaml callback
is permitted during native layout or paint. Images use explicit registered
assets; link navigation is delivered as a typed application event.

Selection uses document generation and UTF-8 byte boundaries. Append preserves
existing ranges; overlapping correction invalidates a selection, and reset
clears it. Copying source ranges is independent of viewport membership; copying
rendered Markdown selection follows the renderer's explicit plain/source mode.
Cross-document transcript copy remains an application data operation.

Public interfaces live in `lib/core/{text_source,document}.mli` and
`lib/eio/document.mli`. The [M4 evidence ledger](../evidence/agent-workspace-m4.md)
records tested behavior, measurements and platform limits.

## Document appearance

Each document's light/dark appearance supplies a coherent native text, code,
table and border palette independently of Base's application-wide theme. Table
bodies have an explicit opaque surface (dark `#111318`, light `#ffffff`); headers
keep the document code-background color. Appearance changes preserve the source
and native presentation. The native adapter passes this surface through the
existing table style refinement, so both wrapping and horizontal-scroll renderer
paths use it without changing application-global colors.

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

## Native presentation

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
Paged source shows its byte interval; gutter numbers retain original line origins.
Small code/diff blocks in Flow use a content-sized area capped at360 logical pixels;
explicit Viewport height remains fixed. A complete single page omits paging metadata.
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
lease teardown. They do not claim physical presentation or complete OS IME
acceptance. The reference app also exercises native document expansion through
macOS accessibility; toolbar actions have explicit accessible names. Code/source
uses the native platform monospace family. See the [M4 evidence ledger](../evidence/agent-workspace-m4.md)
for measurements and consolidated platform validation. Linux GUI remains deferred.

### Unified-diff metadata

The native diff parser preserves every source byte, including partial trailing
hunks, while recording file-local line and hunk ranges. A hunk ends before the
next file's header, including rename-only and binary sections. Native gutter
folding therefore cannot consume the following file's metadata. Files with the
same label remain distinct sections within an installed snapshot.

Each body row retains both optional one-based old/new coordinates and the UTF-8
payload range. That range excludes the diff marker and one LF or CRLF ending;
an extra carriage return or a final standalone carriage return remains payload.
No-final-newline annotations have no old/new coordinates. These annotations
count as body rows alongside context, added and removed lines; file and hunk
headers do not. This supplies the metadata needed for the pending line-limit
and richer event APIs, without changing the existing navigation event yet.

Path labels are at most4096 UTF-8 bytes and contain no NUL. They never cause
filesystem access. Header labels remove the conventional `a/` or `b/` prefix;
explicit rename/copy labels retain their full names. Deleted files use the old
label; other files prefer the new label. Quoted Git labels remain literal quoted
strings, without C-escape decoding. Ambiguous unquoted names are left unknown
until explicit file headers or rename metadata identify them. Rows share path
storage with their file rather than allocating a full path per line.

Diff preparation remains bounded to256KiB, 8192 source lines and16KiB per line,
with cancellation between lines and source fallback on limit overflow. Hunk
coordinates whose final line would exceed the supported positive signed-32-bit
range are rejected. File metadata is native preparation data; per-file collapse,
show-more controls and paired old/new line callbacks are still pending public
API work, tracked by OCH-41.

The pending [diff controls design](diff-controls.md) specifies managed/controlled
ownership, visible-source projection, selection transfer and event provenance.
Its native projection tests are foundation evidence, not mounted public API
acceptance.

## Native document accessibility actions

The native presentation exposes its configured document label as a Group.
Toolbar copy/collapse/search/paging/navigation/rendered-view controls use direct
accessibility action handlers sharing the same operation as pointer/keyboard
activation. Markdown code/table copy actions do the same. This avoids GPUI's
coordinate-click fallback hitting a different control when a virtualized document
is retained outside the visible transcript. Actions resolve the current retained
presentation through a weak entity; they do not retain a disposed document or
scroll/focus an offscreen toolbar as a side effect. The chat presentation test
checks exact source copy from a geometrically offscreen retained toolbar in both
Full and Reduce modes.

## Stable geometry during preparation

After a document has installed prepared content, subsequent source updates keep
that content in place while the next parse/highlight job runs. The initial
Updating notice occupies the existing toolbar before any content has been
installed; no dummy source editor is painted while waiting for the first result.
Keyboard traversal uses the toolbar until a real body exists. Inserting a
normal-flow notice for every chunk would enlarge and then shrink the row on each
parse, moving preceding messages in a tail-following transcript even when the
new text did not add a line. Paginated source metadata likewise describes the
installed snapshot and remains present while newer preparation is pending.
Actual content growth, Markdown interpretation and explicit expand/collapse can
still change row height; this does not freeze document layout or tail following.

A generation reset restores the configured initial collapse state when the new
content installs, unless the user has explicitly expanded or collapsed that
generation in the meantime. Record that interaction against the current source
lease generation, including when preparation has not yet caught up. Delayed
preparation cannot overwrite the newer interaction. An append retains collapse
state; a subsequent reset without a new interaction restores the initial value.
