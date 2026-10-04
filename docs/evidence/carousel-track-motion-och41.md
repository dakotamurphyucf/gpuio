# Measured carousel motion — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
This checkpoint adds native duration/easing movement to measured retained tracks.
**Component and milestone acceptance remain open.** Native track gestures/keyboard,
automatic clock tasks, complete focus/AX policy, gallery/public-driver integration
and physical/resource/release qualification remain required.

## Implementation

Core/Bonsai `View.carousel_track ?motion` uses the opaque checked
`Carousel_track.Motion` API: immediate, default 200 ms ease-out, or positive custom
duration up to ten seconds with existing animation easing. Paired Op97 changes
presentation independently of selection revision and keyed children. Native tree
admission checks the owner kind and numeric envelope atomically.

The native controller retargets from accepted paint, preserves a surviving anchor's
visible coordinate across geometry changes where legal, and normalizes continuous
loop offsets every frame. Small-track boundary jumps remain immediate. It retains
one native owner per item. A private sample identity rejects obsolete paint;
reduced/inactive/unavailable/hidden/replaced owners settle or clear their history.
Frame requests follow eligible accepted paint; motion has no timer, polling loop,
selection callback or per-frame layout event.

This implements the pinned carousel's movement function using duration/easing.
It does not reproduce the pinned styled component's theme spring trajectory.
See the [contract](../design/carousel-track.md#native-track-motion).

## Evidence scope

Five deterministic controller tests cover painted versus speculative retargeting,
stale samples, resize anchors, removal/axis changes, immediate boundary jumps,
150 repeated continuous selections with per-sample coverage, reduced/unavailable
state, cleanup and numerically extreme easing. They do not operate a platform UI.

The actual host test uses GPUI TestPlatform on both axes with unequal cards. It
checks intermediate paint and AX bounds in matching units, clicks the moving
retained child, retains its focus handle, reverses travel from its painted position,
and sees no extra carousel observations during unchanged-geometry animation.
It checks reduced motion, deactivation/reactivation, hidden/show behavior and
unmount, including disposal of the activation subscription and zero tree bytes.
The existing measured renderer test continues to cover padding, resize, effective
loop modes, neighboring controls, zero-size/empty geometry and teardown.

Initial test failures were harness errors: platform activation is asynchronous,
painted quads are in device pixels while test positions use logical pixels, and
bridge removal requires children to be removed before their parents. The final
test drains activation, converts scale explicitly and uses valid removal order.
No production policy was weakened to make these tests pass.

Two Core expect tests cover duration boundaries/rounding, independent binary bytes,
and motion changes that emit exactly one presentation operation without remounting
children or updating the serialized model. The independent Rust protocol vector
checks truncation/trailing bytes and invalid durations/easing; native admission
checks wrong kinds, atomic failure, revision retention and presentation reset.

These are simulated-platform rendering/routing and pure bridge checks. No OS
window, physical keyboard/IME, VoiceOver, hardware GPU or Linux GUI claim is made.

## Validation

Commands use the isolated repository switch and `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --test carousel_track
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests \
  -- -D warnings
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed: full native library **680 tests/two existing private-D-Bus skips**,
transport **4 tests**, full protocol **339 tests/no skips**, strict lint and full
OCaml tests/format/gallery build. Catalog structure and whitespace checks pass.
No upstream dependency pin or unrelated switch changed.


A fresh installed gallery consumer builds successfully with `run=False`:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/carousel-track-motion-installed-gallery
```

An independent executable against that staged installation also imports both
Core/Bonsai track constructors with a custom 350-ms linear Motion, verifies the
serialized presentation operation, reduces a synthetic layout/navigation request
and rejects a callback after close. It prints `GPUIO_INSTALLED_TRACK_MOTION_PASS`.
Its command uses the isolated switch, staged `OCAMLPATH=.../installed/lib` and
`dune exec --root .../consumer -j2 ./motion_probe/probe.exe`. This is public API/
package evidence, with no native application run. The scratch probe is not a
build dependency. Physical/public gallery track interaction remains unfinished.
