# Message follow presentation — OCH-41

2026-10-03, local macOS arm64, milestone worktree based on `83eb87e`.
This is local implementation evidence, not physical desktop qualification or
ticket closure. See the [composition contract](../design/message-follow-presentation.md).

The Collections message preview now composes an animated **Follow latest**
button and optional bottom fade over the existing managed list. Independent
jump/fade/motion switches use ordinary public Views, styles and Animation.Config.
No new protocol field, native controller, fork change or OCaml frame loop is
needed. The existing list controller owns the action and tail state.

The jump/fade targets are shown only for a measured viewport that is neither
following nor at its end. Unknown/short/empty viewports do not suggest unread
content. Overlay geometry does not enter ordinary flow or alter the list width.
Hidden jump content becomes inert immediately, below the animation wrapper so
the exit can finish. Its clipped 48px slot settles below the viewport; an invisible
inert hitbox cannot shield underlying messages. This intentionally uses a larger
slide than the pinned styled widget's eight-pixel offset.

## Physical follow-up — 2026-10-08

The [macOS walkthrough](message-follow-macos-och41.md) adds repository/installed
input and anchor evidence, plus an example-only sizing repair. It does not
retroactively turn the original TestPlatform evidence into physical qualification.

## Validation

- Core composition tests exercise unknown/following/at-end combinations and
  show/hide, jump disabling, fade removal and zero-duration updates. Reconciled
  updates recreate no controls/list nodes and issue no list configuration, order,
  row or scroll operations. Hidden input is inert while its animation owner is not.
- A production Host test uses a 1,000-item managed list with a 12-row budget on
  GPUI TestPlatform. Native time samples check outgoing geometry, unchanged list
  bounds/anchor/owner, native AXClick delivery while available and rejection once
  hidden, finite frame demand, wheel scrolling beneath the settled hidden overlay,
  immediate reduced-motion open/close, and release of animation/list owners.
- Full native library suite passes: **812 tests**, with two existing private-D-Bus
  skips, using
  `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib`.

Final local checks also pass:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe`:
  full OCaml expect suite and gallery executable.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-native -p gpuio-table-adapter --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings`.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt`, `git diff --check`, the structural
  catalog audit, and Python AST parsing of the updated gallery driver.
- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-message-follow-gallery-20261003`:
  `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`. Public libraries
  were staged independently; no unrelated opam switch was modified.

No wire or production Rust changes were needed for this composition; the native
addition is its Host regression test. The Collections physical walkthrough adds overlay activation,
hidden-accessibility checks and an optional screenshot; it has not run. No OS
window was opened for this work. VoiceOver, real GPU/scroll/input/resource checks,
remaining catalog work and release validation remain open. Linux desktop
qualification remains deferred OCH-47; required non-GUI Linux checks stay separate.
