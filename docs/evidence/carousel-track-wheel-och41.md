# Measured carousel trackpad and wheel — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
This checkpoint adds measured precise-trackpad and line-wheel routing to the native
track adapter. Complete focus/AX and external-control relationships,
gallery/public-driver coverage and physical/resource/release acceptance remain
unfinished. No physical desktop window was opened.

## Behavior and ownership

Precise deltas preview measured logical pixels; terminal release proposes the
nearest card and cancellation proposes nothing. Zero-delta terminal events still
finish the gesture. Finite clamping and continuous normalization reuse the checked
track geometry. Line-wheel bursts propose one measured step. A 28 ms quiet fallback
matches the pinned carousel source (`component-carousel-state.rs.txt` and
`component-carousel-scroll_mask.rs.txt` in `docs/catalog/sources/`).

Native child scrollers get first refusal. Vertical finite-edge gestures can bubble
to enclosing scrollers; horizontal tracks retain their axis. The first useful delta
fixes burst ownership, including reversal and arrival at an endpoint. Accepted
controlled-model updates fence remaining momentum rather than stepping twice.
No per-frame selection or layout events cross the bridge.

The host maintains one cancellable quiet task and private epoch per burst. Wakes
recheck source identity, availability and the latest deadline. Timer replacement
cancels stale epochs; an early wake rearms without resetting the gesture's axis.
Tasks retain weak View/source references. Automatic advance pauses during owned
bursts and previews. Hidden/disabled/inactive state, geometry/source changes,
foreign capture, Escape, removal and close cancel unfinished work. Pointer dragging
can take over from a wheel preview without destroying its visible starting point.

Restarting before paint exposed a motion-context issue during review: finishing a
preview previously discarded the geometry needed by the next preview. It now
retains context and marks a controlled retarget, preserving stale-sample rejection.
Nested-scroll testing also required the background hitbox to permit scroll
propagation while still shielding unrelated mouse input. An initial fixture put
container label text before the carousel, clipping it out of its viewport; fixing
that fixture did not relax admission or visibility checks.

## Evidence scope

Four pure wheel tests cover pixel deltas, terminal/cancel behavior, quiet extension,
epoch retirement, endpoint ownership, line bursts, interruption and invalid input.
Four production-host GPUI TestPlatform tests cover:

- Both axes, cross-axis rejection, reduced-motion manual input, painted offsets,
  nearest-card requests, controlled selection and absence of per-frame events.
- Early timer wakes, gesture restart before paint, quiet completion and line-wheel
  model echoes without duplicate navigation.
- Native child scrolling and axis-specific handoff to an enclosing scroll view.
- Escape, foreign capture, changed geometry, disablement, hidden/inactive state,
  handler replacement, unmount and simulated window close while a task is live.

These exercise real adapter dispatch on a simulated platform. They do not establish
physical macOS trackpad/mouse, keyboard/IME, VoiceOver, hardware GPU or Linux desktop
acceptance. Public OCaml API and bridge payload shapes are unchanged. Fresh
installed-consumer and physical release checks remain separate requirements.

## Validation

Run in the repository's isolated environment with `GPUIO_JOBS=2`:

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

Passed: **698 native library tests/two existing private-D-Bus skips**, four
transport tests, strict all-target Clippy, full OCaml tests/format/gallery build.
The focused track subset has 37 passing tests; the full suite includes the final
cross-axis/reduced-motion variants. Catalog and whitespace audits also pass.
No dependency pins or API payloads changed. This checkpoint does not close
OCH-41/OCH-17.
