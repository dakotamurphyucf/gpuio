# Rendered glyph selection and copy capacity — OCH-17

Local macOS checkpoint, 2026-10-07. This follows the
[logical pointer integration](rendered-pointer-selection-och17.md) and the
[hosted capacity failure](hosted-run-37592517665.md). Milestone 07 and
OCH-17/OCH-41 remain open.

## Repairs

An actual-renderer regression selected the middle Hebrew letter in `אבג`.
Native Copy returned the correct letter, but no selection highlight appeared.
The old painter treated logical start/end carets as one left-to-right rectangle.
Rendered selection now reuses the existing shaped glyph-cell background painter,
which handles reordered visual spans instead of bridging them with one rectangle.

Pointer hit regions now use shaped visual rows, and endpoint capture uses
`TextSelectionRun::caret_for_position` for directional, grapheme-aware carets.
The run's immutable geometry cache lives in GPUI element state and is shared with
the current frame's endpoint map. Text, shaped-line identity, bounds and alignment
are refreshed together. Worker-owned parsed nodes acquire no thread-bound layout
state, and pointer snapshots retain no old document or layout history.

The same validation round reproduces the SDK failure from both hosted platforms:
the new 128 KiB copy projection rejected eight 60,000-byte plugin alternatives
that fit the SDK's existing generated-string budget. The projection now admits
1 MiB + 128 KiB for generated alternatives plus ordinary text/separators. Worker
reservation uses that maximum and shrinks to actual retained units. The SDK's
1 MiB aggregate/per-string checks, two-worker bound, 64 MiB global reservation
cap and independent plugin work allowance are unchanged.

The existing SDK eight-node acceptance and nine-node rejection remain. The test
now also checks exact untruncated projection/plain Copy equality and bounded
retained accounting. The separate oversized native-alternative test uses the
declared projection maximum, rather than the obsolete 128 KiB limit.

## Native evidence

`native_highlight_document` passes after the repair. Five single-row Markdown
fixtures cover Hebrew, disjoint mixed-direction spans, Arabic, a combining
sequence and a joined family emoji. The test compares actual selection pixels
with the existing decoration's glyph cells, checks exact Copy/direction and
clearing, and rejects painting across unselected visual gaps. The four
single-cluster cases also drag across measured cell edges in both directions,
checking current logical ranges, Copy, direction and visible highlighting.
The disjoint mixed-prefix case checks programmatic range painting, not a
single-cluster pointer drag.

These are actual macOS rendering and native event-dispatch checks, not physical
pointer, AX selection or VoiceOver acceptance. Existing style/streaming,
selection-disable, mixed-participant and shutdown behavior has separate coverage.

The first run fails on missing Hebrew paint. Intermediate test failures are also
retained: the new comparison initially used the wrong RGBA tuple for red, then
incorrectly treated white pixels inside color-emoji artwork as unselected space.
The corrected single-row oracle fills foreground glyph holes per reference
column while preserving required coverage and rejection of visual-gap overpaint.
Neither correction changes production selection or reduces an existing gate.

## Local validation

All commands use the isolated repository environment with `GPUIO_JOBS=2`:

- `cargo test --offline --workspace --locked -j2`: passed, including the SDK's
  original acceptance/rejection boundary and exact 480,008-byte generated Copy.
- `cargo test --offline --locked -j2 -p gpuio-native --lib --features
  native-canvas-tests,native-image-tests`: 1,092 passed / 2 ignored.
- Workspace/all-target Clippy with native canvas/image/presentation diagnostic
  features and `-D warnings`: passed.
- `dune build -j2 @all @runtest @fmt`: passed, including rebuilt applications.
- `native_highlight_document`: passed with the actual GPU and pointer checks
  above, plus existing policy, style/streaming and mixed-document cases.
- Rebuilt gallery native Select All/Copy and close: passed with both Rust
  backtrace variables set to `1`; the owned process exits normally and the
  complete typed clipboard snapshot is restored. No VoiceOver/settings changes.
- Exact Base reconstruction: 237 files, excluding generated `Cargo.lock`.
  Patch SHA-256:
  `01ab7ef1dfca2c8069718a1b5757c8326e6af6160daee1b381704461c4c68f37`.
- Example documentation audit: 429 sources / 266 reviewed groups / 0 pending.

The [36-file evidence archive](rendered-glyph-selection-och17/reports.tar.gz) and
[verified manifest](rendered-glyph-selection-och17/manifest.json) retain commands,
results, initial failures, source patch/hashes and the rebuilt gallery identity.
The first native-suite pass predates the capacity change; native-012 and the
workspace pass qualify the final source.

## Remaining qualification

This does not complete wrapped/aligned rich-selection qualification, multi-click
or cross-participant logical mapping, preserved Select All overrides, custom
glyph/copy-alternative mapping, pointer hot-path performance, rich TextRun
publication or guarded OS AX actions. Broader platform, physical-presentation,
distribution and release gates remain open.
