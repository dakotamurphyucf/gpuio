# Subtree highlighting

OCH-41 design, in implementation. The configuration foundation is separate from
mounted support: no highlighting capability is advertised until native rendering,
queued observations and the public gallery pass. Existing `Document.Config.search`
keeps its current literal, case-sensitive contract.

## Public vocabulary

`Highlight.Query` owns a nonempty UTF-8 literal, case sensitivity and whole-word
policy. `Highlight.Range` owns a nonempty half-open UTF-8 **byte** interval.
`Highlight.Appearance` resolves theme colors and corner radius.
`Highlight.Spec` combines an optional query, explicit ranges, appearance, optional
active match index and virtualized match offset. `Highlight.Config` is an ordered
list of specs. An empty config deliberately blocks inherited highlighting.

`View.highlight_scope ~config ?on_update children` is a retained scope
with its own asynchronous observer; it does not replace editor ownership or add a
second unrelated callback to every widget. Nested declarations replace the entire
inherited config, including empty declarations. Updating appearance or the active
cursor must reuse match results; removing a scope cancels its work and subscriptions.
Its Core/Bonsai declarations, reconciliation and native tree storage are wired.
The retained-tree collector and GPUI executor service now connect to mounted
ordinary/selectable views, source caching, base/structural visibility and queued
observations. Installed code/diff/source-mode pages also have native matching and
rounded painting. Prepared Markdown text fragments now share a validated native
paint adapter. Custom text and image placeholders use explicit renderer projections;
loaded images contribute no glyphs. Broader acceptance and the public gallery remain
in development.

Queries default to case-insensitive matching. Matching uses Unicode scalar
lowercasing on both source and query, without normalization or full case folding.
Whole-word boundaries require neither adjacent scalar to be alphanumeric or `_`.
Matches are leftmost, nonoverlapping and cannot cross a logical group or newline.
A combining mark is not an alphanumeric boundary character under this deliberately
literal policy. Empty search fields use an empty config rather than a zero-width
query. Multiple specs may overlap and have independent active indices.

Ranges index the scope's ordinary text projection, with one newline between
nonempty logical groups. Adjacent ordinary text children form one group; other
children and nested scope boundaries split groups. Empty text is transparent.
An explicit range can span groups, paints only intersected text and counts once;
a separator-only range counts zero. Offsets splitting a UTF-8 scalar or exceeding
the current projection are invalid, never rounded. The source snapshot validates
these constraints after mounting, since streaming can invalidate a formerly valid
range. Read-only Markdown/code/diff support queries through their native text
projections; they are excluded from explicit-range offsets. Editable controls are
excluded: this API does not expose or mutate editor selections or marked text.

For each spec, query matches precede explicit ranges in the local match numbering.
Query matches follow source order, independent of painting order. Explicit ranges
retain caller order. `match_index_offset` is the number of **matches** before this
mounted subtree, not its row index. `active_index` is zero-based in that larger
sequence. Compare by subtraction after checking the lower bound, avoiding overflow
when the offset is near the signed 64-bit maximum. Native counts cover mounted
content only; an application supplies offsets for virtualized rows.

Colors resolve through the supplied `Theme` when constructing the appearance.
Defaults are its `accent` at 30%/65% opacity, multiplying any existing alpha;
explicit colors retain their alpha. Rebuild the appearance on theme changes.
Radius defaults to 2 logical pixels, with a supported range of 0..64.

## Bounds and ownership

Configuration bounds: at most 16 specs, 4096 explicit ranges across the whole
config, 4096 UTF-8 bytes per query and 256 KiB encoded configuration. Text may
contain whitespace; queries may not contain NUL or malformed UTF-8. A spec needs
a query or at least one range. Indices are nonnegative signed 64-bit integers;
byte endpoints use explicit `int64` units rather than JS floating point or
machine-dependent native integer conversion. Constructors and Rust decoding both
validate, with aggregate list bounds checked before allocation.

Native matching must use immutable source snapshots on a bounded worker pool,
with cancellation, revision/generation fences and explicit capacity outcomes.
Admission must account for source projections, query-sized origin rings, match
records and retired results still referenced by a paint. A completed result is
owned by the mounted scope, never an unbounded global history cache. Source/tree
structure changes invalidate projections; matcher changes invalidate matches;
appearance/active-index/offset changes only invalidate paint. Nested declarations
are excluded from ancestor snapshots so ancestors never count overridden matches.

The initial query kernel (`highlight_search`) streams UTF-8 chunks using KMP and a
query-sized ring of original-byte positions. It does not build a lowercased copy
or an origin map proportional to source length. A shared request budget currently
allows 64 MiB of inspected source bytes across groups/specs and 16384 stored
matches. Storage exhaustion preserves exact total counts with an explicit missing-
ranges flag; work exhaustion/cancellation discards the request's partial results.
It compiles a query once per spec and reuses it across groups. Shared budgets cap
65536 group visits and 262144 chunk visits, including empty text; empty fragments
cannot bypass cancellation or work limits. It checks cancellation at entry,
completion, chunk boundaries and at most 1024 scalars apart during scanning.
These are kernel bounds, not yet a complete scheduler admission policy.

Before mounting integration, define typed pending/ready/invalid-range/capacity
observations with per-spec counts and completeness. Do not silently report a
truncated count as complete or retain stale highlights while a new source/query
is pending. Configuration/range failure must not discard unrelated tree updates.
Matching results and callbacks must be queued outside layout/paint; stale jobs
cannot publish to a replaced handler or closed window. Exact worker/output budgets
will be recorded with the scheduler implementation and measured workloads.

### Projection and result model

The native projection contains ordered logical groups of immutable text runs.
Each run has a generational node ID plus a fragment number; fragment zero is the
ordinary label, while native document adapters may expose multiple painted runs.
Keys are unique within a projection. Ordinary groups participate in explicit
byte offsets; native document groups participate only in queries. Rope-backed
document snapshots retain their original resource reservation and are scanned as
chunks. Building the projection does not flatten their text.

Projection limits are 4096 groups, 16384 runs and 16 MiB of source text, counting
shared sources conservatively per occurrence. Empty input groups/runs consume
admission limits even though they contribute no text. A completed result contains
per-spec exact match counts, per-spec stored match counts, and at most 32768 paint
spans. Each span identifies its spec, zero-based local match ordinal, run and byte
interval; it stores no colors or cursor state. Cross-run matches keep one ordinal.
Results retain a prefix of complete matches: if every span of the next match will
not fit, none of it is stored and later matches are counted without painting.
At most 16384 complete matches are stored across the request.

All explicit ranges are checked against the same immutable ordinary projection
before matching starts. The first invalid spec/range yields a typed out-of-bounds
or scalar-boundary error and no partial paint result. Separator-only ranges are
valid but count zero. Updating text may therefore turn a formerly valid config
into a range-error observation without rejecting the surrounding UI transaction.

Mounted observation states distinguish Pending, Ready (per-spec total/stored
counts), Invalid_range (spec/range index and reason), and Capacity (source/work/
admission limit). A truncated Ready result has exact counts; a work-limit result
does not. Empty overrides resolve to Ready with no specs. Handler/generation and
source revisions fence asynchronous delivery, independently of paint visibility.
`Observation.epoch` is positive and local to the mounted scope. State also has
Failed with Source_unavailable, Worker_failed or Epoch_exhausted; it does not
pretend those failures are complete zero-match results. Count/range payloads are
validated against the current config on both native admission and OCaml dispatch.
Counts have a dedicated bounded sequential bin_prot reader; invalid observations
become typed protocol errors rather than escaping from the event decoder.

The appended bridge tags are Kind50 `Highlight_scope`, Op57
`Set_highlight_scope`, and Event64 `Highlight_observed`. A missing configuration
is invalid; an explicit empty config is a valid nested override. Observers are
optional. Changing config rotates an existing observer binding while keeping
child nodes; changing only the closure uses the latest committed callback. The
scope cannot emit semantic Press events. Observation delivery uses the existing
bounded ordered mailbox and its overload contract, without bypassing input/
response barriers or window-retirement accounting. Capability advertisement is
deferred until native production/rendering and public acceptance pass.

### Worker pool contract

`highlight_collect` walks the validated retained tree in structural order. Own
content is collected for built-in Text/Container nodes, matching GPUIX text/div;
widget metadata and editor values are excluded. Consecutive Text leaves share
one group, with empty leaves transparent. Non-text siblings split groups even
when their content is hidden or overridden. The adapter supplies visibility and
the exact installed native-document groups, with Pending/Unavailable failures
instead of fabricated or partial counts. Empty scopes skip traversal; range-only
scopes do not wait for document queries. The collector limits traversal to 32768
visited nodes (including empty/ineligible nodes), checks projection limits before
appending, and retains original text Arcs without flattening. Validated tree depth
is at most 128. The document provider must bound its own prepared output before
returning it; collector validation does not bound allocations inside a provider.

`Projection.same_source` compares ordered groups, generational run keys and source
identities independently of presentation. Equal immutable strings may reuse the
old projection; document snapshots require the same Arc, preserving installed
revision fences. A mounted cache must invalidate on visibility, page, tree/source
and installed-document changes and may keep its existing Arc when comparison
succeeds. The mounted cache and installed source-page provider are implemented.
Code, diff and Markdown source-mode pages use their installed snapshot interval.
Rendered Markdown uses the immutable fragment table of its installed prepared AST.
Unspecified custom renderers still report SourceUnavailable until they declare an
explicit displayed-content contract; partial counts are not Ready. GPUIO image
placeholders and literal HTML now use the framework-owned text path, while loaded
images explicitly declare non-text content.

Native page projections can now use `Source::document_slice(snapshot, bytes)`.
The constructor validates ordered, in-bounds UTF-8 byte endpoints before creating
an opaque slice. Matching streams only that rope interval and returns page-local
byte offsets. Source equivalence requires both the same snapshot identity and the
same interval, so even a same-content new installed revision retires old paints.
The slice retains the original document-store reservation; highlight admission
counts the displayed interval separately. This primitive now connects to the
installed code/diff/source-mode document presenter and rounded editor painter.
Rendered fragments use `Source::document_text`, retaining the original snapshot
and requiring both its identity and the prepared fragment allocation to match.

An application-wide native pool owns at most 128 mounted scope entries and two running jobs, with
64 MiB of conservative admission units for queued/running/ready/retired data.
Admission accounts for retained source/projection/configuration, the maximum
possible bounded paint output for that input, and query scratch space. These
units are a quota, not an allocator-RSS measurement. Charges follow immutable
request data through worker completion and every retained paint result.
Once work finishes, unused output and scratch reservations are released; the
source and actual output remain charged through their last reader. This avoids
holding worst-case match storage for a completed no-match query.

Changing source or matchers immediately retires the old ready state, cancels its
job and advances a scope epoch. Cosmetic updates keep that epoch/result. An
admission failure also clears old highlights; it does not keep painting a stale
query. A failed scope can retry admission explicitly. Only one job per scope runs
at once; an updated scope queues its latest request behind cancellation of the
old job. Fair round-robin dispatch prevents a constantly changing first scope
from starving others. Completion checks pool identity, task identity and scope
epoch before publishing. Dropping a scope or closing the pool cancels work and
retired completions cannot revive it. A native service must dispatch these work
objects on the background executor and drain them at shutdown; no OCaml callback
may run from the worker or paint.
Opaque pool-local scope IDs let that service route accepted completions to their
own windows. A dropped work/completion ticket is detected during pool maintenance
and releases its worker slot with a typed failure instead of hanging indefinitely.

`highlight_host` now provides that GPUI service: a two-slot completion channel,
deferred bounded worker dispatch and refreshes routed to the owning native window.
Only work/completion data cross threads. It has no idle polling and never calls
OCaml. Scope drop and window close explicitly cancel jobs, including handles that
outlive their OS window. Abandoned-ticket cleanup cannot overwrite a closed
state. Epoch exhaustion is distinct from ordinary admission capacity.

Application abort, protocol shutdown and platform quit all join worker destruction
signals. Workers use nonblocking completion delivery, so synchronous quit does
not wait for UI callbacks. Signals remain reachable during asynchronous shutdown
so a reentrant synchronous quit can wait on the same workers. Final cleanup reaps
discarded completion tickets. The native test exercises actual executor wakeups,
closed-window cancellation and repeated cleanup with two in-flight jobs. This is
service lifecycle evidence, not yet mounted highlighting/painting acceptance.

## Shaped-text paint adapter

`highlight_paint` resolves the prepared per-run matches to appearance without
rerunning queries. A Paint and its geometry cache retain the worker's Ready lease,
so retiring a scope does not release its reservation while a paint reader still
owns the source. Cache identity includes source, truncation policy and GPUI shaped
line identities; bounds, alignment and line height are taken from the current
paint. The cache stores font-run metadata and references GPUI's shaped glyph data,
without creating a glyph-sized position copy. Monotonic glyph indices use binary
lookup; other layouts use GPUI's own index lookup.

The underlay emits a rounded quad per intersected visible visual row. It uses
actual wrap boundaries and per-row alignment, with downstream start affinity,
rather than guessing positions from character widths. It maps retained source
slices through start/end/middle truncation and never highlights a synthetic
ellipsis. A clipping mask skips off-screen rows; GPUI clips the painted pixels.
For reordered shaped text, logical endpoints alone are insufficient. The shared
`ReorderedTextGeometry` indexes visual glyph cells by their logical source-cluster
ranges and emits separate visual spans; an unmatched intervening glyph is not
painted merely because it lies between the selected logical endpoints. Wrapping
clips those spans to each actual visual row before alignment. A partial range
inside a reordered shaped cluster paints its complete cell. Matching/count order
remains logical source order.

Monotonic layouts keep the existing fast path without a per-glyph index. Ordinary
text caches reordered geometry with the shaped line; source editors lazily cache
it with their wrapped layouts and clear it on replacement. Markdown constructs it
once per line per paint pass, outside the match loop. This geometry is proportional
to shaped glyph count; whole-application CPU/memory budgets remain a separate gate.
The change does not alter GPUI's native caret, selection or accessibility mapping.

Native selection backgrounds paint afterward. Active-index comparison subtracts
the virtualized offset after checking its lower bound, avoiding signed overflow.

The helper and mounted ordinary/selectable and prepared Markdown text views pass
focused native GPU tests, including declared custom text and dynamic image
placeholders. Hebrew/Arabic, mixed direction, wrapped RTL and aligned ordinary
text now have targeted native glyph-cell/pixel evidence; source/Markdown adapters
also paint the middle Hebrew query. This is not exhaustive script/IME/caret
acceptance, and highlighting has not yet passed the
milestone's application-scale performance budget.

## Mounted ordinary-text integration

Each native View owns scope states keyed by generational NodeId. A state keeps its
worker handle, current configuration, a weak source reference and bounded per-run
paint caches. Rejected/retired sources are not retained by an uncharged strong
cache reference. Tree revision or structural visibility identity invalidates
collection; equivalent projections reuse the original worker source. Cosmetic
changes preserve result and epoch. Source/matcher changes clear old paints before
new work can publish; empty configurations complete immediately without a job.
The mounted observation epoch is independent of a recreated worker handle.

Search visibility currently follows explicit/query hidden branches, navigation
selection and base Display/Visibility fields (last declaration wins). Inertness,
disabled input and modal input gates do not remove visually present text. Dynamic
state-style visibility and animated navigation still require dedicated integration
checks. Actual virtual-list recycling, installed native pages, controlled
tabs/disclosures and native responsive branch changes now have focused evidence.

A scope marks itself during paint. The window sweeps unused scopes only after the
complete paint effect cycle, including lazy children and deferred surfaces; render
return is too early for that cleanup. After sweeping, observations require the
current tree revision, current visibility identity and valid handler/configuration.
Changed observations enter the existing ordered input mailbox, never a synchronous
OCaml call from layout or worker code. A new observer receives the current sample;
unrelated layout changes do not duplicate it. Actual scope reclamation invalidates
admission-failed scopes once for retry without introducing idle polling.

Ordinary StyledText and selectable text share the same search underlay. The
selectable widget retains its existing selection, native focus and input handling;
selection backgrounds paint above the search wash. Scope removal releases caches
and cancels matching; retired frame readers continue to carry their reservation
until dropped.

## Installed native-document adapters

The public [Find & highlight gallery](../../examples/gallery/highlight_page.ml)
shows a complete Core/Bonsai/Eio integration, including scoped documents, native
query editing, paired observations, cursor clamping and nested exclusions. Its
[native evidence](../evidence/subtree-highlighting-och41.md) is narrower than full
highlighting release acceptance. Match selection changes color; automatic reveal
is not part of this scope API.

The code/diff/source-mode and prepared Markdown text adapters are implemented.
Declared custom text and dynamic image projections are also implemented; broader
acceptance remains open. The document presenter keeps the last installed revision
visible while the next parse is pending. Its highlighting source must use that same installed snapshot,
mode and page, with a separate identity that also changes for collapse/expand and
native page changes. Publishing a new parse must invalidate the enclosing scope
and wake the root without reentering its current render. Before first installation,
collection reports Pending; collapsed bodies contribute no text. A completion for
a prior installed page must never decorate the replacement page.

The source editor exposes shaped visible lines and their original buffer-byte
offsets internally. Its background pass now accepts immutable rounded range washes,
preserving syntax backgrounds below and active-window native selection above.
An Rc-owned provider retains the match-result lease through its last frame reader;
replacing/editing text clears prepared ranges. Only read-only buffers accept this
API. Range count, UTF-8 endpoints and radius are validated; invalid input clears
old washes. The frame retains its own provider and paints visible buffer lines,
using actual soft wraps and alignment rather than treating displayed/folded rows
as a contiguous byte string. Native checks now cover vertical/horizontal source
viewport clipping, tab/Unicode prefixes and retained background ownership during
scrolling. Scrolling does not change the installed source page or its count.
Native hunk-fold mapping and application load tests remain required.

The enclosing scope's source/observation stamp includes a shared document identity.
Native document invalidations replace that identity and defer a root wakeup, avoiding
reentry during child rendering. The painter verifies its fragment key and exact
installed snapshot/page against Ready's projection. Cosmetic updates reuse result
and background owners; page or installed revision changes cannot reuse stale paint.

Bounded Markdown preparation now builds `DisplayedText`: at most 4,096 immutable
fragments and 64 KiB of text, with consecutive IDs in structural display order.
Paragraph/heading formatting stays in one run; images and custom objects split
runs, and each cell and code block is independent. The table exists before any
paint, including virtualized blocks. It is not the copy/AX representation: those
representations include separators and alternative text that may not be glyphs.
Unspecified custom nodes are counted explicitly and prevent whole-document query
acceptance. A Rust renderer can implement the projection contract below.

The prepared inline state carries its fragment identity; `InlineFlow` retains the
original fragment byte interval when splitting by font size, inline code or wraps.
`TextBackgrounds` validates the complete layer array, UTF-8 ranges, radius and
32,768-range aggregate bound. Installation requires the same prepared table Arc.
Replacement or ordinary asynchronous parsing clears the old provider. Before
painting a fragment, its exact text allocation and displayed byte slice must agree.
Formatting backgrounds paint first, rounded query washes next, glyphs next and
native selection last. Providers retain Ready leases through their last frame.
No matching, I/O or OCaml callbacks run in paint. Cosmetic changes preserve the
AST, selection and measured list layout; streamed parsing retains old fragments
until the replacement is actually installed.

Focused native GPU tests cover headings, cross-format text, inline and fenced
code, table cells, wrapping, rounded corners, selection, streaming, collapse and
owner disposal. Broader script/bidi, tabs/folding/scroll, rich-object accessibility, concurrent
windows/virtual rows and application-scale resource/performance acceptance remain
required. The current Markdown range geometry uses GPUI's index lookup; dense
match/long-line benchmarking and measured optimization remain release work.

## Custom Markdown presentations

`MarkdownPlugin::presentation` declares `Text`, `NonText` or `Opaque` from already
prepared resources. No I/O, layout or matching is allowed in that resolver. `Text`
is painted by TextView itself, bypassing the arbitrary plugin renderer, so its
searchable bytes and glyphs agree. Atomic inline objects retain their existing
copy text, accessibility label, links and selection controller; passive glyph
children add no second input controller. Their selection layer paints above the
child content. Text blocks retain whole-document selection/copy; this does not
claim complete partial block-selection or accessibility acceptance.

`NonText` keeps the native renderer and contributes no searchable text. Its
fallback also contains no glyphs, so a missing or invalid element cannot turn an
AX/copy alternative into an unprojected label. `Opaque` is the default for an
unspecified renderer and fails complete query collection explicitly. Renderers
must declare `NonText` honestly; this is a trusted Rust extension contract, not
inspection of arbitrary native elements. Built-in low-level image nodes without
this declaration remain opaque; GPUIO intercepts its images through the safe plugin.

Each parsed custom occurrence has a fresh retained identity even when a plugin
clones a template. Immutable projection data is indexed by that identity; matching
order remains the structural fragment vector's order. Passive rendering creates
frame-local readers and never shares a mutable controller between occurrences.
The existing fragment/byte/layer admission limits also bound projected custom text.

GPUIO declares literal HTML as text, image placeholders as `[Image: alternative]`,
and decoded images as non-text. The presenter caches extension handles rather
than creating a new configuration every frame. An image-resource change rebuilds
the bounded installed AST's displayed projection on the UI thread without parsing
or querying, clears old paint owners, preserves logical selection/list position,
and invalidates the enclosing scope. Ordinary async parsing remains distinct.
The original document revision is unchanged by a resource-only update. Native
paint caches retain no extra projection owner when there are no actual washes;
an individual fragment with zero matches does not suppress other fragments.

## Pinned comparison and acceptance

Compared with GPUIX `18e695ed0ee8121a7793413ca795e08eda2a13df`:
[`HighlightSpec`](../catalog/sources/gpuix-host.ts.txt), native
`packages/native/src/text/search.rs` and `renderer.rs::resolve_highlight`.
GPUIO deliberately uses UTF-8 bytes instead of JS UTF-16 units, validates malformed
configuration explicitly, and bounds allocation/work rather than retaining every
match without a cap. The nearest-scope, multiple-spec, range, cursor, virtualized
offset and cosmetic-cache behaviors remain part of the target.

Acceptance requires paired configuration/event fixtures, Unicode/boundary and
cross-run tests, overlapping specs, empty/nested scopes, active-index overflow,
worker cancellation and resource reclamation. Native tests must cover ordinary
and selectable text, Markdown/code/diff, clipping/wrapping, selection precedence,
streaming source replacement, virtualized offsets and independent windows. A
public gallery find bar must demonstrate count/cursor updates, theme changes and
teardown. Configuration-only tests do not establish any of those mounted claims.
