# Chart inspection editors and aggregate identity — OCH-41

2026-10-06, macOS 14.5 arm64, base `647e266` plus this change.
This extends the [initial renderer evidence](chart-inspection-renderer-och41.md).
No production adapter repair was needed: the existing retained editor and chart
lifetimes satisfy the new cases. OCH-41 and the broader release remain open.

## Native evidence

The new `native_chart_inspection` executable runs the existing ordinary-button
checks in a fresh foreground window, then tests both `Input` and `Textarea`:

- Tab enters the editor; native text insertion changes its draft. Left moves the
  editor caret without changing the chart selection. A child font/style update
  preserves draft, revision, selection and focus.
- Actual macOS `NSTextInputClient.setMarkedText` starts a marked composition.
  Publishing reordered data and completing the new chart geometry preserve the
  composing editor and its snapshot. `insertText` commits the composition.
- Hiding a targetless slot while composing ends the old client's composition and
  native focus before a new paint. Late `insertText` cannot edit through that
  retired handler. Explicit Focus commands return `FocusBlocked` while hidden.
- Returning the target retains the editor's draft. Resetting the source generation
  also ends composition/focus before paint and rejects late text. Original-data
  browsing hides the card and blocks focus; fresh inspection recovers. Unmount
  removes the editor instance.

Separate actual GPU/button cases cover **Sum, Mean and OHLC**, each with one and
four input values. The four-value datasets deliberately use non-monotonic IDs:

- A one-value aggregate remains aggregate content; it cannot use first-datum
  singular content. Matching aggregate metadata renders actual button pixels.
- Equal selection/publication numbers from another resource do not grant access.
- New source revisions retire content before paint, including when first/last IDs
  and count stay equal but an interior ID/value changes. Old metadata stays hidden
  after replacement geometry.
- Aggregate selection itself clears on publication. New metadata does not create
  a preview; a fresh native preview/commit is required. Explicit rebinding then
  restores the content. A button gesture spanning retirement remains rejected
  after return, while a fresh gesture works.

The combined foreground test passes with final native metrics `(41, 12, 2, 0, 0)`
and zero chart source bytes after teardown. The test closes its window and workers.
The first editor fixture incorrectly gave a single-line input two rows; admission
correctly rejected it. Another test initially assumed aggregate selection survived
publication, contrary to the established contract. Both test assumptions were
corrected; no production rule was weakened.

These are real GPU pixels, GPUI event dispatch and AppKit text-client calls.
They do not qualify a physical keyboard, a Japanese IME candidate panel, VoiceOver,
or Linux desktop behavior. Existing native composition commit/unmark policy is
preserved; no keyboard-source or VoiceOver settings are changed.

## Harness and validation

Inspection checks now have their own foreground mode in the shared chart harness.
The old mounted-chart/input suites remain. CI is configured to compile the new
executable on both platforms and run a separate macOS execution step with the existing build/consumer
prerequisites and its own 90-second limit. This makes the growing lifecycle cases
independently reproducible without rerunning all chart/radar scenarios.

Use `GPUIO_JOBS=2` with the isolated repository environment:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_inspection --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
./scripts/gpuio exec cargo fmt --all --check
```

Native feature library tests pass **1,053**, with two existing ignored tests.
Strict feature-enabled Clippy, the existing foreground chart-input suite and
Rust formatting pass. Workflow YAML parses; the example inventory remains
421 source files / 262 reviewed groups / 0 pending. The existing hidden-window
mounted-chart regression also passes, with final metrics `(177, 19, 2, 0, 0)`;
the foreground input suite ends at `(112, 7, 2, 0, 0)`. Both release chart source
bytes and native retained work. No OCaml code or runtime dependency changed in
this checkpoint; the preceding full Dune evidence remains historical.

Final qualification results, exact commands and source hashes are in the
[log archive](chart-inspection-editors-och41-logs.tar.gz) and
[verified manifest](chart-inspection-editors-och41-manifest.json). No current-source
hosted or Linux run is inferred from the workflow edit or local macOS results.

Remaining inspection work includes queued accessibility/command/popup actions,
nested clipping and multi-chart/window interaction, rich-row convenience APIs,
and root/fresh-installed public-gallery walkthroughs. Overall catalog and
OCH-17 performance, accessibility, distribution and release gates remain required.
