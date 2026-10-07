# Mapped multi-click selection — OCH-17

Local macOS checkpoint, 2026-10-07, following
[cross-document ranges](rendered-cross-selection-och17.md). OCH-17/OCH-41 and
milestone 07 remain open; macOS AX selection is not published by this change.

## Implementation

Double-click now resolves the current shaped hit to an actual prepared text
owner. Word selection spans its styled fragments and expands scalar word results
to whole graphemes. Ordinary paragraph selection follows structural separators;
rich-flow triple-click combines the mapped text and inline objects on the clicked
visual line. Repeated words and identical object alternatives retain their actual
owner identity.

Inline objects have a sorted, bounded native-owner index, avoiding a full
projection scan for every visible object during paint. Actual and maximum
preparation units include that allocation. The common range uses distinct
multi-click provenance, updates the same native owners used by paint and Copy,
and survives compatible reflow/append without selecting the appended suffix.

Synchronous clear handlers retire old selections. Their queued empty snapshot
notification now respects a newer local selection installed by the same press,
instead of erasing its logical range. Explicit clear and replacement still retire
ranges in the covered scenarios.

## Evidence

The before-001 TestPlatform regression fails because a double-click copies the
correct word but exposes no logical range. The final tests cover:

- Correct repeated-paragraph occurrence for word/paragraph selection, clearing,
  reflow and compatible append without selecting new text.
- A word crossing inline-code/style fragments; full rich-line selection including
  leading/trailing atomic objects; selecting the second identical alternative.
- Wrapped visual-line selection against the independently published visual
  fragments, rather than assuming a whole word remains on one row.
- Whole combining sequences and joined family emoji.
- Actual macOS native double/triple-click ranges, direction and keyboard Copy,
  followed by the existing cross-document, selection-policy, styling and Unicode
  GPU suite.

Intermediate fixture failures are archived: fragments-003 searched an object's
value instead of its actual accessibility label; wrap-006 assumed `example`
occupied one row, while the 70px layout placed its first letter on the preceding
row. The corrected tests retain strict owner/range/Copy assertions and compare
the wrapped selection against actual visual fragments. Those changes do not
modify production wrapping or relax a release budget.

All build commands use the isolated repository environment, `GPUIO_JOBS=2`:

- `cargo test --offline --workspace --locked -j2`: passed.
- `cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests`:
  1,096 passed / 2 ignored, including final append checks.
- Strict workspace/all-target Clippy with canvas/image/presentation diagnostic
  features: passed.
- `dune build -j2 @all @runtest @fmt`: passed, rebuilding example applications.
- Cargo formatting: passed. Base reconstruction: exactly 237 files, excluding
  generated `Cargo.lock`. Example docs: 429 sources / 266 reviewed groups / 0 pending.
- Fresh rebuilt gallery native Select All/Copy and normal close: passed with
  Rust backtraces enabled; full typed clipboard restoration and process exit
  verified. No VoiceOver or system settings changed.

The [archive](rendered-multiclick-selection-och17/reports.tar.gz) and
[manifest](rendered-multiclick-selection-och17/manifest.json) retain commands,
logs, source hashes and the patch.

## Remaining requirements

The shared native word policy is bounded and character-class based; this is not
full language-aware word segmentation. Unmapped/custom text and zero-byte atomic
alternatives retain native fallback behavior without a fabricated logical range.
Those mappings, streamed Select All endpoint affinity, stale pre-repaint callback
qualification, selection performance and rich AX publication/actions still need
completion. This evidence is not physical-input, VoiceOver, IME, Linux desktop,
physical-presentation or release/distribution acceptance.
