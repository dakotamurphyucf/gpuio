# Measured toast reflow and lifecycle painting — OCH-41

Local checkpoint, 2026-10-03, macOS checkout at base `83eb87e` plus the milestone
working tree. This advances the [presentation design](../design/toast-presentation.md)
and the locally qualified [layering API](toast-layering-och41.md).
**Public Motion, production phase scheduling and delayed dismissal are unfinished.**
The production Host currently selects immediate geometry and no lifecycle preview;
the animation evidence below exercises the native widget through TestPlatform.

## Implementation

Reflow keeps separate axis trajectories and advances layout-discovered targets
from the last committed paint timestamp. A target changed by streamed content on
every frame therefore continues moving; an unchanged width trajectory is not
restarted by unrelated height changes. Interruption preserves painted velocity,
and only the final current-epoch sample may commit.

The renderer samples the width before measuring text, then combines the current
natural height with the horizontal center and vertical anchor edge. It animates
that composed edge position, not independent stack-height and card-offset springs.
A growing bottom card stays attached to its bottom edge, and new content is not
clipped behind an old animated height. Wheel scrolling settles layout motion.
Expanded bottom stacks follow growing content only when already scrolled to the
bottom. Fully offscreen old positions after a resize snap into the usable surface;
a width spring reaching zero cannot prevent measurement of a positive-width card.
Inactive, disabled and reduced-motion paths settle without replaying old travel.

Each card may carry an opaque lifecycle preview. The widget applies its opacity
and vertical slide, then invokes a native after-paint observer only after child
painting. A transparent or initially slid-out entry remains eligible through its
unslid layout bounds, while current opacity and transformed bounds determine
interaction. Ending cards remain inert. Only accepted painted lifecycle samples
may request further frames; hidden layers, rejected previews and zero-area owners
do not sustain animation. This callback carries no synchronous OCaml event.

Accepted child removal now immediately prunes retained native geometry, even if
the window does not draw again. Hidden stack teardown releases its descriptors.
The Host retains child editor ownership and the existing hard submission bound.

## Verification

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` with repository pins:

```sh
cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
cargo test --offline --locked -j2 -p gpuio-native \
  --test toast_delivery --test toast --test toast_geometry \
  --test toast_lifecycle --test toast_reflow \
  --test toast_layering --test toast_placement
cargo clippy --offline --locked -j2 -p gpuio-native --all-targets \
  --features native-image-tests,native-canvas-tests -- -D warnings
cargo fmt --all -- --check
```

The full native library suite passes **741 tests with two existing private-D-Bus
skips**. All **22 focused tests** pass. Strict native all-target Clippy, Rust formatting, the catalog structural audit
and `git diff --check` pass.

Nine widget tests include actual text measurement during width animation,
per-frame streamed height changes, interrupted expansion, scrolling, resize,
reduced motion, transparent first paint, child-paint-before-lifecycle-commit,
interrupted exit, inert Ending content and no further frames after completion.
Nine pure reflow tests cover axis independence, painted velocity, speculative
frames, hidden layers, immediate removal/generations and numeric-domain fallbacks.
A production Host test verifies geometry and editor release during accepted
removal before any subsequent draw. Existing layering and placement Host tests
continue to pass.

No OS windows opened. These are deterministic pure/TestPlatform checks, not
physical macOS GPU/input/IME/VoiceOver acceptance. No new Core, protocol, installed
consumer or Linux run is claimed for this native-only change. OCH-41, OCH-17 and
the milestone remain open. Logs and recovery notes are under
`scratch/agents/root-20260929-m7-resumed/`, prefixed `toast-motion-renderer-`.
