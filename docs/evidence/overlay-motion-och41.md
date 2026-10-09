# Modal entry evidence — OCH-41

2026-10-02, local macOS/arm64 checkout `milestone-07-gallery-release`, base
`83eb87e` plus the uncommitted milestone work. This is deterministic GPUI
TestPlatform evidence. No physical OS window was opened for these tests.

`Overlay.Motion.Enter` now drives native dialog/alert fade and slide, and sheets
at all four edges, inside the actual deferred viewport surface. Immediate remains
the default. Paired Op91 bytes and atomic final-tree admission cover the public
option. The gallery offers **Animate opening**. See the
[contract](../design/overlay-motion.md).

Three host regressions exercise all six modal kinds: actual painted panel alpha
and bounds, moving pointer hitboxes, accessibility geometry and stable identity,
Tab routing, background AX rejection, transparent first-frame dialog blocking,
backdrop changes without replay, reduced motion, disabling/re-enabling, hidden
state retirement and fresh entry, early close, owner cleanup and zero settled
frame requests. Hidden/show validation restores identical style and size before
checking the exact new entry offset.

The tests exposed a real focus issue with top sheets: entry can place the first
controls entirely outside the viewport, making the ordinary first-frame focus
selection fall back to the scope without later autofocus. The native manager now
holds the modal scope during entry, resolves the first eligible child after
settlement and respects earlier user focus in a visible child. The dedicated top
sheet regression verifies eventual autofocus without user input; the all-kinds
regression verifies that click/Tab choices during entry are not overwritten.

Initial test-harness failures were corrected: accessibility coordinates require
physical-to-logical pixel conversion, disposed node generations cannot be reused,
and alert dialogs cannot request outside-pointer dismissal. Those failures are
not reported as production defects.

## Completed checks

In the isolated repository environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --offline --features native-image-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --offline --features native-image-tests --lib overlay_motion_test
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol \
  --offline --test overlay_motion
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
```

Full native library: **638 passed, two existing private-D-Bus skips**. The final
strengthened host regression also passes separately (three tests). Protocol:
**328 passed, no skips**. Atomic admission and independent codec fixture: one
test each. Strict Rust lint passes. The pure motion-state test covers
first-paint start and permanent settlement under reduction/disable/completion.
The authored desktop gallery walkthrough passes Python syntax checking only.

Full OCaml tests, formatting and the public gallery build pass with:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

Two new Core expect tests verify motion-only reconciliation preserves modal and
child ownership, and Op91 matches independent bytes. A fresh staged gallery
consumer also passes:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/overlay-motion-installed-gallery
```

It reports `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
The script stages public packages separately without modifying the opam switch,
copies the gallery and rebuilds its native backend. This is a consumer build,
not installed-app runtime or clean-machine distribution acceptance. Physical keyboard/IME/VoiceOver/visual/GPU and
measured resource acceptance, current Linux automation, CI/review/publication
remain separate release work. Managed tooltip motion and exact positioning/inset
mapping remain catalog work. No ticket or milestone is marked complete.
