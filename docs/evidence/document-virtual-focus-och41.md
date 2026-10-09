# Virtual document control focus — OCH-41

Local macOS arm64 checkpoint on `83eb87e865c86717a8bc51b9db6fe1f379d909a9`
plus the working tree. These tests mount the production host on GPUI TestPlatform;
they do not open OS windows or establish physical keyboard, IME or VoiceOver acceptance.

## Behavior

Tab previously skipped native actions when their document block was outside the
virtual viewport. A second regression demonstrated that merely revealing the block
was insufficient: a tall code block's action row lies below its glyphs.

The reader now retains non-tab-stop scopes for AST blocks that may contain native
controls. Logical links retain their existing navigation phase; native controls
then follow block order and the native tab order within each block. Disabled,
hidden and preview-clipped controls are excluded. Preview clipping never scrolls
hidden controls into eligibility.

GPUI exposes read-only snapshots of eligible rendered tab stops and measured
candidates. Clipped candidates supply reveal geometry only: they remain excluded
from ordinary Tab traversal and keyboard handlers. The reader scrolls first and
requires a subsequent painted frame to admit the target. These records participate
in GPUI's existing cached-paint replay; the queries do not change focus or flush
pending key chords.

Realization advances at most one block/control per frame, bounded by the prepared
AST and its native controls. It runs after an actual paint, without idle polling.
A weak callback, cancellation epoch, captured interpretation/input guard and focus
anchor prevent obsolete work from stealing focus. Source replacement, pointer/
scroll input and non-Tab input invalidate pending work. Host composite traversal
handles document exit, including modal/tab-index eligibility, without accidentally
reentering a native child.

## Test coverage

Seven added production-host cases cover:

- Offscreen code actions and activation carrying the exact code snapshot.
- Tall code rows, distant tables, both Tab directions and leaving the document.
- Empty/disabled action rows reaching the enclosing focus order without trapping.
- Read-only focus queries and moving focus elsewhere before a pending paint.
- Native profile inline, block and code controls in both directions, with a queued
  event carrying the installed source generation/revision.
- Rapid Tab input while realization is pending, without restarting logical links
  or scrolling away from the pending native target. Repeats coalesce until a
  frame determines the next stop; reversing direction cancels pending realization.
- Source reset before the pending reveal paints, while retaining the old picture
  during replacement preparation. The test performs input and reset in one app
  update so the test scheduler cannot paint between them.

The targeted actions/profile run passes **18 tests**, including all seven new
cases. The standalone pinned Base text/selection suite passes **235 tests**. The
final full native suite passes **876 tests**, with two existing private-bus tests
skipped on macOS. Strict all-target lint, the public gallery build, repository
formatting, structural catalog audit and whitespace checks all pass.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-native -p gpuio-document-sdk -p gpuio-extension-sdk \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
```

The standalone Base command follows the [document accessibility evidence](document-accessibility-och17.md)
with the local GPUI/AccessKit/Taffy overrides and `--locked`, so it does not modify
dependency selections.

Both vendor patches reconstruct from locally cached, hash-verified pinned upstream
archives: GPUI **156 source files**, Base **235 source files**, byte-identical.
The ignored Base standalone Cargo.lock is a local build artifact, excluded from
this source comparison. Patch SHA-256 values:

- GPUI: `52aa27fac4452e7529c4682f479f54d5ebe731ee192f07c7f1a8c82509bc288d`.
- Base: `e32084961fb5253ae60520493fba15c3b3475a35389ebf704231b62559b25c4a`.

No dependency revision or shared switch changed.

## Remaining qualification

A subsequent [selection/focus checkpoint](document-profile-selection-och41.md)
adds traversal across 96 passive plugin candidates. A subsequent [nested-scroll checkpoint](document-profile-scroll-och41.md) covers independent viewport clipping, focus exit and wheel reveal on TestPlatform. Physical keyboard/accessibility acceptance requires further qualification. A parent document can reveal a control
in its own vertical list; it does not take over an arbitrary plugin's independent
scrolling contract. The existing two-phase link/control order is preserved.

Full catalog coverage, macOS resource/performance/IME/VoiceOver/GPU acceptance,
notices/distribution/API/review and required Linux nongraphical checks remain in
OCH-41/OCH-17. This checkpoint does not complete either ticket or milestone 07.
