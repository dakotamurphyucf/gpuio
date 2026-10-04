# Workflow stepper — OCH-41

Local checkpoint, 2026-10-02, macOS, isolated toolchain, worktree based on
`83eb87e`. This is local automated evidence, not a physical desktop, screen-reader
or release acceptance claim. No OS window was opened for these checks.

`Gpuio.Stepper` now provides a validated application-owned model and public view
composition. The navigation gallery demonstrates horizontal/vertical and centered
layouts, custom content/symbols, current stage, relative navigation and per-step/
whole-control disabling around a retained notes editor. The
[contract](../design/workflow-stepper.md) distinguishes positional completion from
business validation and defines latest-model request handling.

Four Core expect tests check:

- Relative requests skip disabled steps, saturate, use latest current, and behave
  deliberately without a selection. Removed/disabled targets cannot be selected
  by stale requests; programmatic selection remains explicit.
- Reordering preserves current identity; removing it clears selection. Empty,
  all-disabled, maximum-size and invalid-label/dimension models have defined
  behavior. Localized accessibility descriptions are validated.
- Real reconciliation preserves native trigger IDs through reorder, relabel and
  orientation changes. Whole-control disable and removal retire activation.
- Rich triggers carry current-step and localized position/status descriptions,
  with one handler per enabled step. Interactive custom descendants are rejected.

A fifth Core test serializes the actual public composition and compares it with
the versioned 2604-byte `stepper-public-view.hex` fixture. The native test decodes
that same fixture and passes it through real Session admission and the retained
renderer on GPUI TestPlatform. It checks positive trigger geometry, four native
focus owners with no extra glyph/connector owners, disabled AX state, Current.Step
and description, Tab/Shift-Tab traversal, Enter/Space press events and accessible
Click delivery. An AX action queued before disabling its target is rejected at
delivery, rather than using its old painted enabled state.

The initial native test reproduced a stack overflow on first draw with the default
test stack. Moving nonrecursive event/focus/probe decoration from `element_body`
into `node_actions.rs` fixes the same fixture with unchanged stack limits. No
smaller tree, new stack environment setting or native-acceptance relaxation is
used. This does not establish arbitrary maximum-depth safety or explain the
separate black-startup-window issue.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline`:
  **580 passed, two existing skips**, including the queued AX-disable regression.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passed.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: passed, including
  all five current Core/fixture tests and the expanded gallery.

The serialized fixture is a cross-language integration artifact generated from
the public composition, not an independent protocol-tag specification; no new
protocol tags or vendor patches were introduced. Full native tests were run
because the stack repair changes shared renderer code. Existing Linux automated
checks, actual macOS keyboard/VoiceOver/visual/resource acceptance and installed
consumer/release checks remain required. The structural catalog audit and `git diff --check` pass. These changes are local
and uncommitted.
