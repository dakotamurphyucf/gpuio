# Managed lists (OCH-13)

Implementation in progress, 2026-09-24. This document refines the accepted
[managed-list contract](accepted-contracts.md#managed-list-contract); it is not
an assertion that native virtualization has shipped.

## Ownership and data

`Gpuio.List_collection` is an immutable, ordered collection with unique typed
keys. It stores application data, never views or Bonsai models. Point replacement
preserves position and takes O(log n), making it suitable for streamed updates.
Structural splices and reorder rebuild O(n) positional metadata. Metadata and
loaded application data are O(n); this cost must be reported separately from
active row graphs, native views, measured heights and rendering caches.

Point updates share an immutable `keys` snapshot, so order processing can be
cut off by snapshot identity. `fold_changed_keys` uses persistent-map sharing
to invalidate changed rows without scanning the unchanged history. It reports
conservative invalidations, not semantic equality of arbitrary application data.

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
publish the immutable snapshot supplied to `on_change` directly to Bonsai.

These modules retain loaded data intentionally. An unbounded conversation needs
an application persistence/windowing policy, rather than a claim that UI
virtualization makes retained application data constant-space.

## Native rendering plan

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
Bonsai component and full-history resource acceptance tests remain in progress.

The native state adapter now wraps the actual GPUI `ListState`. State-level
tests preserve key/offset through prepend/reorder, invalidate row heights, pause
following while away from the tail, and explicitly resume it on jump-to-end.
They also reject obsolete order revisions and repeated scroll commands. The state checks run without a window. A separate `native_list` graphical test
uses the production host: 100,000 logical rows, sparse descriptions, measured
row heights, exact key/pixel anchors through prepend/reorder/height changes and
resize, tail jump, focused-row retention and disposal. Wheel/scrollbar gestures,
IME/selection and full-history resource bounds still need expanded acceptance
coverage.

The first native metadata layer uses positive logical row IDs independent of
native node handles. Consecutive IDs are encoded as runs: an initial 100,000-row
order occupies eight bin_prot bytes, verified independently in OCaml and Rust.
An uncompressed native index supports key lookup and anchor relocation. That
index is O(n) metadata; compact wire size is not a claim of constant native
memory. A removed anchor falls forward to its next surviving neighbor, then
backward, then to the new first row, at offset zero. A surviving anchor keeps
its exact offset through prepend/reorder.

The metadata validator caps logical rows at 1,000,000 and runs at 32,768, rejects
overlapping IDs and checks overflow before expansion. Active descriptions have
a separate configurable budget capped at 16,384. These are per-value admission
limits: native tree/session accounting must also charge expanded metadata and
GPUI measurement storage. The existing 1-MiB message and 64-MiB tree budgets still
apply. Highly fragmented large reorders may need staged metadata transport;
that integration and precise supported limits remain to settle before release.

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
are rejected after that source order changes. Requests prioritize pinned rows,
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
are executed. Full-history heap/native-resource validation remains.

Creating a separate Bonsai driver for every newly visited row is not the chosen
shortcut: in this pin, driver construction registers a `Ui_effect.Define`
handler in a global table and observer invalidation does not unregister it.
Independent drivers also cannot implicitly capture parent graph values. Keep
the managed rows inside the existing window graph and validate the explicit
retention contract without changing the Bonsai fork unless evidence requires it.

## Validation status

The collection/paging expect tests cover 100,000 logical records, point updates,
range traversal, atomic invalid changes, concurrent boundaries, retry/end and
obsolete responses. That is data-layer evidence, not bounded native-view or
Bonsai-row memory evidence. Scoped producer tests cover queued delivery,
cancellation and conversation isolation. Native anchoring, real focus/IME,
scrollbars, full-history active-resource budgets and platform gates remain.
