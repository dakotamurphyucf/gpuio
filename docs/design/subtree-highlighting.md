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

The planned `View.highlight_scope ~config ?on_update children` is a retained scope
with its own asynchronous observer; it does not replace editor ownership or add a
second unrelated callback to every widget. Nested declarations replace the entire
inherited config, including empty declarations. Updating appearance or the active
cursor must reuse match results; removing a scope cancels its work and subscriptions.

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
