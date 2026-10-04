# Measured carousel background dragging — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
This checkpoint connects measured background pointer dragging to the native host.
**Trackpad/wheel handling, complete focus/AX and external-control relationships,
gallery/public-driver integration and physical/resource/release acceptance remain
unfinished.** No physical desktop window was opened.

## Behavior

Axis-lock decisions use pointer coordinates; previews and release targets use
measured track pixels. The pending origin follows accepted paint until claim.
Claimed capture supports movement outside the viewport, finite clamping and
continuous-loop rebasing without cloning native item owners. Release proposes a
nearest logical item; the pinned quarter-viewport boundary rule preserves intentional
first/last wrapping for short jump-mode tracks. Rust never independently updates
the controlled selection. Until the application accepts a new selection, release
returns toward its current target from actual paint.

The drag region preserves nested input precedence. Mouse-down is installed on the
viewport Div before its automatic focus handler, after nested handlers. An outer
Region alone ran too late: GPUI's own viewport focus handler had already prevented
default. The repair changes listener placement; it does not ignore prevented child
input. Captured movement/release remain on the region. Frame routes share the
admitted config instead of cloning the item-ID payload per event listener.

Geometry, model/presentation/source changes, Escape, hidden/inactive state, lost
capture, unmount and close retire the gesture. Foreign capture is preserved.
Automatic advancement is ineligible during a drag or preview. Direct manipulation
works under reduced motion; settlement follows the existing motion policy.

## Validation scope

Three pure gesture tests cover unequal item sizes, axis lock/rejection, measured
snap targets, finite bounds, deliberate short-track boundary wrap, large continuous
cycle movement, invalid coordinates and following paint before claim. A motion
regression covers preview, obsolete samples, release to the controlled target and
retargeting without speculative position changes.

Three production-host GPUI TestPlatform tests cover:

- Both-axis pixel movement, out-of-bounds capture, nearest-card requests, unchanged
  controlled selection and no per-frame selection/geometry events.
- Native child-button precedence and cancellation for Escape, foreign capture,
  same-viewport geometry changes, disabled/hidden/inactive state, handler replacement,
  unmount and actual simulated window close with an active gesture.
- Native editor pointer text selection, retained editor focus, arrow input and typing.
  The track neither captures that pointer nor emits navigation requests.

The editor fixture initially left a removed child in its parent's child list;
strict admission rejected it. Replacing the parent's child before removing the old
node fixed the fixture. No admission validation was relaxed.

These are actual adapter tests on a simulated platform, not physical mouse,
trackpad, keyboard/IME, VoiceOver, hardware GPU or Linux desktop acceptance.
The existing measured layout, motion and automatic-deadline tests run as regressions.
There are no public API shape, bridge payload or dependency-pin changes here.

## Validation commands

Use the isolated repository environment with `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --test carousel_track
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed on the final shared-config route implementation: **690 native library
tests/two existing private-D-Bus skips**, four transport tests, strict all-target
lint and full OCaml tests/format/gallery build. The focused track subset has
29 passing tests. Catalog structure and whitespace checks pass. This does not
close component acceptance or the remaining release gates.
