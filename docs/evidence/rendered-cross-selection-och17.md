# Cross-document logical selection — OCH-17

Local macOS checkpoint, 2026-10-07, following the
[glyph-selection repair](rendered-glyph-selection-och17.md).
OCH-17/OCH-41 and milestone 07 remain open. This implements native selection
ranges across participants; it does not publish macOS AX selection yet.

## Behavior and ownership

A real-window regression drags from ordinary text through a prepared Markdown
document to another ordinary-text node. Copy already returns the right text,
but before this change the intermediate document exposes no logical selection.
The archived `before-native-001` run fails on that missing range.

Window snapshots now retain anchor/cursor participant ordering from registered
document order. The document adapter combines that ordering with endpoint
ownership and coverage to derive its directed local range. Endpoint documents
validate their captured positions against the current preparation. Intermediate
documents cover their complete installed projection in the gesture's direction.
There is no selected-string search or screen-Y inference.

The existing staged owner update applies these ranges to native selection,
painting and exact plain Copy. Window gesture/autoscroll/scope/retirement ownership
and source-format Copy reconstruction remain intact. An adopted pointer range is
not reported as an external adapter request.

## Verification

The actual macOS `native_highlight_document` suite passes. New assertions cover
forward/backward full intermediate ranges and both endpoint documents' logical
ranges, direction, origin and Copy. Existing cases also exercise focus-independent
Copy, local Select All, reorder, source replacement, unmount and recycled nodes.
These are native event-dispatch/GPU checks, not physical input or AX acceptance.

A new TestPlatform test uses three identical documents and partial text endpoints.
It checks exact forward/backward range equivalence, a fully selected middle,
Copy, reorder ownership, and retirement of every range when the middle source is
replaced. Its first two runs clicked the center of the wider paragraph layout
rectangle, outside actual glyphs; those fixture failures are retained. The final
fixture sorts visual row bounds and targets text, preserving all partial-range
assertions.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test --offline --workspace --locked -j2`: passed.
- `cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests`:
  1,093 passed, 2 ignored.
- Workspace/all-target Clippy with native canvas/image/presentation diagnostic
  features and `-D warnings`: passed.
- `dune build -j2 @all @runtest @fmt`: passed; rebuilds the example applications.
- `cargo fmt --all -- --check`: passed.
- Exact Base reconstruction: 237 files, excluding generated `Cargo.lock`.
- Example documentation audit: 429 sources / 266 reviewed groups / 0 pending.
- Rebuilt gallery native Select All/Copy and normal close: passed with Rust
  backtraces enabled, complete typed clipboard restoration and child-process
  exit verified. No VoiceOver or system settings changed.

The [archive](rendered-cross-selection-och17/reports.tar.gz) and
[manifest](rendered-cross-selection-och17/manifest.json) retain source hashes,
commands, logs, failures and the patch. Multi-click, streamed Select All remapping,
custom glyph/copy mapping, hot-path performance and rich AX actions/publication
remain required work. This checkpoint supplies no new Linux desktop, IME,
VoiceOver, physical-presentation or distribution acceptance.
