# Cached accessibility callback ownership — OCH-17

Follow-up to `f62271d1`, local macOS arm64. This is a callback-lifetime fix;
OCH-17 and milestone 07 remain open.

## Failure and fix

A window containing a cached document and an uncached button retained the
button's accessibility callback after removing it. The previous frame's callback
map survived until another frame, even though that callback would never be
replayed. An idle window could therefore keep the removed callback owner alive.

`A11y::finish_cache_replay` now clears unused previous-frame callbacks after all
root, deferred, prompt/drag/tooltip and inspector painting. Callbacks replayed by
cached content have already moved into the current live map. Semantic snapshots
remain frame-bounded. This neither disables caching nor changes action routing.

The regression in `rust/native/src/document_cached_owner_test.rs` captures an
owner strongly only in the uncached control, removes that control, and observes
its weak pointer after exactly one explicit draw. It intentionally omits `notify`
on removal: notifying can automatically draw and make a second explicit draw
hide delayed cleanup. It also requires an unchanged cached-child render counter
and exercises that child's live accessibility callback afterward.

The exact single-frame fixture failed before the fix and passed afterward.

## Separate confirmed TextView gap

The later [native lifecycle fix](cached-text-selection-och17.md) addresses this
gap. The following records the failure at this earlier checkpoint.

A diagnostic using a real Base `TextView` inside a cached entity, with the native
`TextSelectionLayer` outside it, selected `Cached 世界` successfully on a fresh
paint. After a parent-only redraw, selection became empty and the child's render
counter rose from 4 to 6. This **does not pass cached TextView acceptance**.

Inspection identifies a lifecycle mismatch: cached scene reuse skips the native
participant registration performed during `TextView::paint`, while the selection
layer still sweeps participants absent from the current frame. That can clear the
selection and cause another refresh. Accessibility-tree replay alone does not
replay this selection-controller lifecycle.

The failing diagnostic source and full failure output are preserved in the
bundle. It is a follow-up reproduction, not an ignored passing-suite test. The
production selection fix was pending at this checkpoint. It must preserve current paint order,
selection and Copy during actual reuse, handle a cached selection layer and mixed
participants, and retire detached participants. Disabling caching, forcing every
frame to rerender, or retaining absent participants indefinitely is not acceptance.

## Validation

All commands use the repository toolchain with `GPUIO_JOBS=2`:

- `./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests`:
  **1,170 passed, 2 ignored**, including the new owner regression and existing
  nested-deferred/action/context cache checks. This suite does not include the
  unresolved TextView diagnostic above.
- `./scripts/gpuio exec cargo clippy --offline --workspace --all-targets --locked -j2 --features gpuio-native/native-canvas-tests,gpuio-native/native-image-tests,gpuio-native/presentation-diagnostics -- -D warnings`: passed.
- `./scripts/gpuio exec cargo fmt --all -- --check` and explicit
  `rustfmt --edition 2024 --check vendor/gpui/src/window.rs vendor/gpui/src/window/a11y/cache.rs`: passed.
- `python3 scripts/vendor_gpui.py --archive <pinned-local-archive> --output <isolated-scratch-directory>`:
  all **159 GPUI files** reconstruct exactly, excluding generated `Cargo.lock`.
  Patch SHA-256: `3135cd7c524c83fc9a4c2410a03ab6cdb98593ae0ae0c47f049eed6c5010fd79`.

These are native TestPlatform checks on macOS. No new OS GUI, VoiceOver, Linux,
physical-performance, installed-consumer or distribution acceptance is claimed.
No test changed the system clipboard or VoiceOver settings. Full release gates,
rollback/context qualification and cached TextView selection remain outstanding.

Evidence: [reports.tar.gz](cached-accessibility-owners-och17/reports.tar.gz) and
[SHA-256 manifest](cached-accessibility-owners-och17/manifest.json). The archive
contains the before/after logs, unresolved TextView reproduction, complete native
suite/lint/format logs, reconstruction result and changed implementation source.
