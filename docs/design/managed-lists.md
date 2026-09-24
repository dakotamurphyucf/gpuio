# Managed lists (OCH-13)

Implemented and validated locally and in CI, 2026-09-24.
[PR #11](https://github.com/dakotamurphyucf/gpuio/pull/11) tracks final review and merge.
This document refines the accepted
[managed-list contract](accepted-contracts.md#managed-list-contract). See the
[acceptance evidence](../evidence/managed-lists-och13.md) for exact coverage.

## Ownership and data

`Gpuio.List_collection` is an immutable, ordered collection with unique typed
keys. It stores application data, never views or Bonsai models. Point replacement
preserves position and takes O(log n), making it suitable for streamed updates.
Structural splices and reorder rebuild O(n) positional metadata. Metadata and
loaded application data are O(n); this cost must be reported separately from
active row graphs, native views, measured heights and rendering caches.

Point updates share an immutable `keys` snapshot, so order processing can be
cut off by snapshot identity. `fold_changed_values` uses persistent-map sharing and private value versions
to invalidate replaced values without scanning unchanged history. Prepend and
reorder preserve value versions; explicit replacement changes the version even
when the supplied data is physically identical. `fold_changed_keys` additionally
reports positional changes. Neither operation compares arbitrary application data.

`Gpuio.List_paging` owns two explicit boundaries: ready, loading, failed or end.
At most one request per boundary is active. Before/after pages can complete in
either order; before pages arrive in normal display order. Both splice into the
current collection, preserving intervening streamed updates. Duplicate keys
reject a page atomically. Failed loads require explicit retry. An empty page
must advance the cursor or reach end, preventing a repeated empty-page loop.

Requests carry controller identity, conversation generation and request serial.
Reset, cancellation and retry invalidate previous requests. Obsolete responses
are ignored before inspecting their payload. Identity is private and cannot be
constructed by applications. Cursors are opaque application strings.

`Gpuio_eio.List_paging` adds scoped Eio producers and UI-loop notifications. It
cancels producer fibers on reset/cancel/close and suppresses queued completions.
Its parent scope should be the conversation or application if loading must
survive virtual-row deactivation. No viewport operation implicitly cancels
conversation work. Applications pass I/O capabilities to the loader closure and
use its reactive `value` directly with `Gpuio_bonsai.Virtual_list.paged`, passing
`controls` for generation-checked request/retry/cancel effects. The optional
`on_change` is an additional UI-domain notification.

These modules retain loaded data intentionally. An unbounded conversation needs
an application persistence/windowing policy, rather than a claim that UI
virtualization makes retained application data constant-space.

## Native rendering

The simple list retains every supplied description but builds GPUI elements only
for the native requested range. The managed list retains descriptions and row
computations for viewport plus overscan and explicitly pinned rows. Both use
the pinned GPUI `ListState` for variable-height measurement and scrolling. Its
synchronous render closure consumes available Rust descriptions or a positive
estimated-height placeholder; it never calls OCaml.

An asynchronous native observation must be generated from actual layout, not
only wheel events: resize, data changes and programmatic scrolling all change
the requested range. Focus, composition and selection pin resources independently
of visibility. The list must preserve key plus pixel-offset anchors through
prepend/reorder and height invalidation. Native tail following pauses when the
user leaves the end; jump-to-latest resumes it. Data updates precede commands in
the same accepted transaction. Scrollbar support belongs to this ticket.

The pinned native implementation provides `splice_focusable`,
`logical_scroll_top`, `scroll_to`, `remeasure_items`, `FollowMode::Tail` and
scrollbar geometry. It can retain an offscreen focused item. These primitives are now connected to GPUIO's retained native host. The managed
Bonsai component is implemented; full-history native resource acceptance remains
in progress.

The native state adapter now wraps the actual GPUI `ListState`. State-level
tests preserve key/offset through prepend/reorder, invalidate row heights, pause
following while away from the tail, and explicitly resume it on jump-to-end.
They also reject obsolete order revisions and repeated scroll commands. The state checks run without a window. A separate `native_list` graphical test
uses the production host: 100,000 logical rows, sparse descriptions, measured
row heights, exact key/pixel anchors through prepend/reorder/height changes and
resize, tail jump, focused-row retention and disposal. The extended test also checks wheel pause, scrollbar drag, focused editor
composition through the macOS text client, held selection, intentional source
deletion and full-history resource bounds. These are local macOS checks.

The first native metadata layer uses positive logical row IDs independent of
native node handles. Consecutive IDs are encoded as runs: an initial 100,000-row
order occupies eight bin_prot bytes, verified independently in OCaml and Rust.
An uncompressed native index supports key lookup and anchor relocation. That
index is O(n) metadata; compact wire size is not a claim of constant native
memory. A removed anchor falls forward to its next surviving neighbor, then
backward, then to the new first row, at offset zero. A surviving anchor keeps
its exact offset through prepend/reorder.

The metadata validator caps logical rows at 1,000,000 and runs at 100,000, rejects
overlapping IDs and checks overflow before expansion. Active descriptions have
a separate configurable budget capped at 16,384. These are per-value admission
limits: native tree/session accounting must also charge expanded metadata and
GPUI measurement storage. The existing 1-MiB message and 64-MiB tree budgets still
apply. A fragmented 100,000-row permutation fits in one message, verified in both
languages. Larger compact histories can fit within aggregate admission limits,
but arbitrary reorders beyond 100,000 runs are rejected. There is no staged
metadata transport in this version. The 1,000,000-row validator ceiling is not
a promise that a million-row tree fits the aggregate budget; application data
and description sizes also constrain admission.

## Bridge and stale eviction

`Gpuio.Virtual_list.Config` chooses fixed or estimated row height, overscan,
maximum active rows, tail policy and a native scrollbar. Fixed rows clip at the
specified height; estimated rows are measured by GPUI. `View.virtual_list`
retains all supplied keyed descriptions. The expert managed description supplies
an independent `Order` plus a sparse active set. Orders should be shared across
value-only changes. The reconciler preserves surviving logical IDs across reorder;
a streamed row update sends text and a height invalidation, without the full order.

Native transactions validate list mappings, command targets and metadata before
publication. Logical metadata is charged at 192 admission bytes per row before
expansion; this conservative quota is not a measurement of RSS. The tree and
native list share one immutable index. Viewport events include order revision and
are rejected after that source order changes or their tree revision becomes stale. Requests prioritize pinned rows,
then visible rows, then overscan, with an explicit budget-exhaustion diagnostic.

Native focus can change after OCaml receives a viewport event. The host therefore
snapshots actual row focus, editor focus/composition and active text-selection
gestures immediately before applying a transaction. If a candidate evicts a pinned
row whose logical ID still exists, the entire transaction remains unapplied.
A single reserved `List_retained` reply identifies the submitted revision and
required rows. The OCaml driver discards only that pending candidate, schedules
retention callbacks from the last accepted view, and retries without running
Bonsai deactivation/reset hooks. Explicit source deletion or list removal still
disposes the row. A historical unfocused selection alone does not pin it forever.

Tests cover atomic rollback, explicit deletion, response-reservation release,
source-generation validation, independent OCaml/Rust bin_prot fixtures, and an
actual Bonsai row model that survives a retry without activation/deactivation or
reset. Ordinary accepted removal still runs deactivation exactly once.

## Bonsai v0.17 retention findings

`test/lifecycle/retention_test.ml` verifies optimized and unoptimized graphs:

* Ordinary keyed models are retained after a key leaves an `assoc` input.
* `with_model_resetter` invoked on deactivation removes a standard model after
  it returns to its default value.
* A custom child reset can deliberately preserve a non-default model.
* A previously captured static action can recreate a model after reset.

These are Bonsai's intended state/effect semantics, not a diagnosed upstream
bug. Counting balanced activation/deactivation hooks is insufficient evidence
of bounded retention.

The managed renderer therefore needs both a reset policy for transient models
and a lifetime guard for delayed effects. Persistent preferences belong above
the row in application state. Row-owned async work must be cancelled; pending
results must pass lifetime/generation checks. A custom reset that retains row
state cannot be included in a bounded-default retention claim.

`Gpuio_bonsai.Managed_rows.assoc` now implements the reset wrapper and gives
each visit a `Lifetime.t`. `Lifetime.guard` checks validity when a completion
effect executes. A guard around only the start of an asynchronous operation is
insufficient: guard the effect that eventually injects its result. Re-visiting a
key creates a new lifetime, so an old guarded callback cannot affect it.
Activation hooks can use the guard immediately; deactivation hooks run before
the wrapper resets their model. Tests verify this ordering in optimized and
unoptimized graphs, and inspect the actual Bonsai model after 1,001 visits.
The model returns to the empty keyed map, including after old guarded callbacks
are executed. The component memory test now visits and revisits 100,000 rows;
native full-history resource checks also pass locally.

Creating a separate Bonsai driver for every newly visited row is not the chosen
shortcut: in this pin, driver construction registers a `Ui_effect.Define`
handler in a global table and observer invalidation does not unregister it.
Independent drivers also cannot implicitly capture parent graph values. Keep
the managed rows inside the existing window graph with the explicit
retention contract and the bounded action-history policy below.

## Application API and paging

`Gpuio_bonsai.Virtual_list.component` accepts a typed `List_collection`, a stable
injective `row_key`, a validated config, and `render_row ~key ~data ~lifetime`.
It returns an `Output` with the view, scroll controller, current native viewport,
active row count and budget-exhaustion flag. Give the view a bounded height;
by default it fills its parent's assigned area. Ordinary applications do not
implement viewport membership. Native pins plus optional application pins take
priority within `max_active`; excess application pins return an error.

`Controller.scroll_to`, `reveal` and `jump_to_latest` produce effects. A controller
belongs to its mounted conversation generation; delayed commands from an inactive
generation are ignored. Missing targets are harmless. Change `generation` when
replacing a conversation that could reuse keys. Persistent message/preferences
and conversation-scoped tasks live outside `render_row`.

The `paged` variant consumes the Eio pager's shared snapshot and controls. It
requests Ready boundaries near the visible edges, including short/empty lists.
Failed boundaries need explicit retry; End never loads. `auto_load=false` suspends
new automatic loads, and explicit cancellation stops current work. Merely moving
a row offscreen never cancels a conversation request or stream.

Height invalidations compare against the last *accepted* immutable collection,
not an intermediate observed result. This preserves all streamed changes while a
native transaction is pending. Acceptance advances the baseline. The component
retains one accepted collection snapshot, separately from its bounded row models.
`Output.viewport` is `None` until native layout confirms the current geometry
revision. Nonempty pages wait for that layout before requesting again; empty
cursor-advancing pages can continue without an unnecessary layout barrier.

## Bounded action-history policy

Bonsai v0.17 also keeps a recent action-path trie for stabilization decisions.
Resetting row models does not immediately prune that independent cache. A
20,000-key experiment retained about 771,523 additional words until the normal
age-based pruning ran; afterward it retained about 1,523. This is intended cache
behavior, not an upstream model-retention defect.

The native window driver selects the additive
`Bonsai_driver.Action_history.Release_after_flush` policy. It releases the trie
only after the complete action batch and stabilization. Within-batch dependency
tracking remains intact. The upstream-compatible default is `Keep_recent`.
The fork patch and hash are recorded and reconstructed against the pinned source.

The production-policy expect test visits all 100,000 rows, then revisits them,
with at most 100 active models and a 2-KiB payload per visited model. Weak
references retain at most the active payloads, and none after eviction. Immediate
retained heap growth is below 150,000 words above an already-loaded metadata
baseline. This is a heap bound, not a native RSS measurement. Application records
remain present. A separate test compares dependent static/dynamic action batches
under both policies. The native traversal evidence is recorded separately below.

## Validation status

Local macOS checks cover 100,000 logical records, point updates, atomic collection
changes, concurrent boundaries, retry/end, obsolete responses, managed model
reset/heap bounds and production-driver coalescing. The public
[conversation example](../../examples/virtual_list/README.md) verifies paging,
offscreen streaming, stable prepend anchors and tail resumption through the full
OCaml/Rust bridge. Its composer uses the existing native editor contract.

The native interaction test uses actual platform frames and native input routing.
Its separate full-history stress section explicitly drives GPUI layout/paint,
visiting and revisiting all 100,000 rows with 256 descriptions, row focus handles
and selection objects at a time. Weak probes verify old row selections are
released. Retired text payloads held by native rendering caches are separately
bounded at 512; the observed peak during traversal was 101. After list unmount,
261 tracked payloads remained; window disposal released them all. This is native
resource/cache evidence, not a claim that all caches vanish at row eviction.
Logical order/index/measurement metadata remains O(100,000).

The bulk test does not measure physical display cadence or frame latency. Direct
layout/paint avoids relying on thousands of display-link callbacks while the
owner uses the desktop. Its interaction section still waits for platform frames,
and all paths close the window on failure. The foreground macOS text-client test
covers marked/committed text, not a human-operated IME candidate panel.

Lists expose List/ListItem accessibility roles for rendered content and accept
an accessible name through style. This does not synthesize an accessibility node
for every unloaded row or claim comprehensive screen-reader traversal.
`CAP_VIRTUAL_LISTS` is 67108864; the aggregate bridge mask is 134217727.
Hosted macOS functionality and Linux build/unit checks pass for the implementation;
the linked PR records the final head and merge. X11 also passes the automated list
checks. Wayland stops at an existing combobox clipboard failure before reaching
them. Linux GUI remains informational under OCH-17, per the accepted priority.
