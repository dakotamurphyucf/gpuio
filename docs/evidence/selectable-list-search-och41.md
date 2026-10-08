# Selectable-list search and gallery — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`.
`Gpuio_eio.List_search` and the Collections → **Searchable list** public example
now connect asynchronous search to the [managed component](selectable-list-managed-och41.md).
Physical desktop, VoiceOver, resource/performance and release acceptance remain
open. No upstream or dependency changes were made for this layer.

The later [physical macOS matrix](selectable-list-macos-och41.md) records
repository/installed keyboard and appearance qualification. The unrun statements
below describe this earlier implementation checkpoint.

## Search ownership

One scope-owned controller holds the loaded collection, latest immutable snapshot
and current producer. Search receives an immutable source/query request and can
perform Eio I/O using explicit captured capabilities. A supplied monotonic clock
drives debounce; no timer or task is attached to a virtual row.

Results contain upserts and an explicit visible order. Existing memberships and
hidden loaded records survive a successful merge; fetched records append to the
same source. Duplicates, unknown visible keys and capacity violations reject the
entire result. The default cap is 100,000 loaded records, configurable up to one
million; payload bytes remain application-owned. Eviction is an explicit source
edit, with the normal removal/reinsertion identity semantics.

New queries, refresh and source structure/replacement cancel prior work and fence
already queued completions. Same-source edits retain only surviving visible
memberships while waiting. Point updates declared `refresh=false` must not affect
search matching/order; they preserve the query and visible metadata. Newer point
values win over an older producer's upserts, using persistent-map change tracking.

Snapshots distinguish debouncing, loading, ready, failed, cancelled and closed.
Previous results are explicitly stale while a new query is unresolved. The gallery
disables list interaction then, while leaving the query editor usable. Retry and
cancellation are epoch checked; editor query updates are source checked. Task
admission failures are visible retryable failures. Cancelled fibers remain charged
to the shared Scope task quota until they unwind; replacements cannot allocate
an unbounded work queue. Parent cancellation closes the controller and unregisters
cleanup. The initial empty query exposes the loaded collection without starting I/O.

## Public gallery

The window owns a separate search scope created outside Incremental evaluation
and cancelled on window closure. Its fixture contains 1,000 entries plus section
headings, disabled options and a 16-row active budget. Search uses the real retained
editor, forwards committed observations only and does not search composing text.
Query replacement commands handle their replies explicitly.

The example offers fetched remote entries, a retryable failure, no results,
cancellation, single/multiple selection, both orientations, live detail updates,
new-source reset and row-local Inspect counts. It reports selection, loaded count,
mounted count and independent primary/secondary/context/cancel intents. Filtering
does not delete selected hidden records. Search tasks continue outside transient
rows; page unmount resets component preferences according to the managed contract.

`scripts/test_gallery.py --section selectable-lists` now includes a physical
macOS walkthrough for real query/list keyboard input, filtering, fetched records,
selection preservation, retry, source reset and orientation. The script has been
syntax checked but **not executed** at this checkpoint. It is an acceptance recipe,
not a desktop or IME/VoiceOver pass. Existing desktop-access limitations remain;
an unchanged failed preflight was not retried.

## Local validation

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/runtime`: passed.
  Eight new mock-Eio tests cover exact debounce deadlines, superseded/queued work,
  fetched/hidden membership, streamed-value conflicts, atomic malformed-result
  rejection, retry/cancel, task quota, parent close, reincarnation and invalid
  configuration/query preservation, plus closing with a saturated UI inbox and
  producers suspended on completion delivery. No wall-clock sleeps or external service.
- An additional Bonsai-driver integration test in that suite connects `List_search.value`
  to the real `Selectable_list` component: stale results disable commands, remote
  merges preserve hidden preferences and original targets, live values preserve
  selection, and a new source retires old controllers.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe`:
  passed with the new search adapter and gallery.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` and `git diff --check`: passed.
  The final inbox-saturation test was formatted with the pinned formatter and
  the full runtime suite passed again after that addition.
- `python3 scripts/audit_component_catalog.py`: passed source checksums,
  structural inventory and family ownership. This is not behavior acceptance.
- Python AST parsing passed for the new physical walkthrough and its driver;
  neither script was run against a desktop.
- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-selectable-search-gallery-20261003`:
  passed fresh package staging and independent compilation/linking of the gallery,
  including its real search/component integration (`run=False`). No opam switch
  installation or global toolchain mutation.

No OS windows were opened. Production Rust was unchanged; the preceding native
806-library-test/44-integration-test/AppKit-fixture evidence remains applicable.
Required Linux non-GUI checks and OCH-17 release gates remain separate; OCH-47
Linux desktop qualification remains deferred. The full milestone is not complete.
