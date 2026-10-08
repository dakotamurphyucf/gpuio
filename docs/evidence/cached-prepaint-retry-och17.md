# Cached prepaint retries and source replacement — OCH-17

Follow-up to `968e423b`, local macOS arm64. This checkpoint fixes a reproduced
cache-retry crash and verifies source replacement on real cached Base TextViews.
It does not complete milestone 07.

## Reproduction

The native retry fixture uses GPUI's public `Window::transact` path, which a list
uses to abandon prepaint and retry after determining autoscroll. It first adds a
semantic node, prepaints an already cached document and then aborts. A successful
second attempt prepaints the same child at the same bounds without that extra
node. Before the fix, the second attempt panicked in accessibility replay:
`range end index 4 out of range for slice of length 3`.

The first attempt had overwritten `ViewElementState.prepaint_range` with offsets
into its current frame. The transaction discarded that frame data, but did not
undo the saved range. The retry then used the new offsets to read the previous
painted frame's snapshot. This was an index-lifetime mismatch, not malformed
application content.

## Fix and tests

`ViewPrepaintState` now holds the candidate prepaint range for an individual
attempt. Replay keeps reading the previous painted range; only successful paint
commits the new range. Fresh state left by an aborted attempt is marked unpainted
and cannot be mistaken for a reusable scene. Valid existing caches still reuse
their scene during both attempts; the fix does not force them to re-render.

`rust/native/src/document_cached_retry_test.rs` covers four combinations of an
aborted initial render versus a retained cache and direct versus nested-deferred
controls. Three repeated retries preserve the exact native node-ID set, omit the
discarded node, keep the cached child render counter unchanged and route the live
button's accessibility action correctly. This exercises the production transaction
and cache machinery through TestPlatform; it is not a claim of exhaustive rollback
for arbitrary third-party state mutations.

A separate test in `document_cached_text_test.rs` establishes real cache reuse,
prepares a selection request, then replaces the text. The old request returns
`StaleRequest`; ordinary entity invalidation updates the semantics without a
manual refresh; old selection/text disappear; selecting the new CJK-containing
content works and survives later cache reuse. This source-update case passed
before the retry repair and remains passing afterward.

## Validation

`GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests`:
**1,175 passed, 2 ignored**. The final source also passed these checks through the
same isolated toolchain (`GPUIO_JOBS=2`):

- Strict workspace/all-targets Clippy with native canvas/image tests and
  presentation diagnostics, `-- -D warnings` (36.15 seconds).
- Workspace `cargo fmt --all -- --check` and explicit edition-2024 rustfmt for
  the changed GPUI source.
- `cargo test --offline --locked -j2 -p gpuio-native --features native-tests --test native_editor --test native_document`:
  actual macOS editor/document behavior (71.03 seconds).
- `cargo test --offline --workspace --locked -j2` (144.50 seconds).
- `dune build -j2 examples/gallery/main.exe` (101.91 seconds).
- `python3 scripts/test_macos_text_selection.py --rendered-only --output <fresh-scratch-directory>`:
  actual macOS heading, CJK, joined-emoji and code range setters, exact Copy, and
  stable caret (3.91 seconds). UTF-16 ranges are `[33,17]`, `[91,2]`, `[96,11]`,
  `[392,7]`, and caret `[392,0]`. Owned app exit 0; clipboard restored and verified;
  VoiceOver settings untouched. Binary SHA-256:
  `0dfba5c007797eee2fc6e97338b58c4b00f15b4274ab3c8d657991b8c7e887d7`.

The native fixture proves transaction/reuse behavior; the real OS run is a public
gallery regression, not a claim that it induces a cache rollback. Durations are
validation timings, not accepted performance budgets.

All **159 GPUI files** reconstruct exactly from the pinned archive and patch
(excluding generated `Cargo.lock`). Patch SHA-256:
`b490783ed759f01f3751804b599e3f9ce42a6ece515fa56255233a4469756e95`.
Base is unchanged from `968e423b`.

macOS accessibility/VoiceOver, consolidated gallery/catalog acceptance,
performance/resources, notices/distribution/provenance and required current-source
hosted/Linux checks remain milestone work. Earlier scope/platform policy is
unchanged; no Linux graphical qualification is claimed here.

Evidence: [reports.tar.gz](cached-prepaint-retry-och17/reports.tar.gz) and
[SHA-256 manifest](cached-prepaint-retry-och17/manifest.json), containing the failing
reproduction, final checks, implementation/source snapshots, reconstruction and
actual OS report. Source provenance is the dirty tree based on `968e423b` plus
the archived patch and source files, before the delivery commit.
