# Measured carousel keyboard and deadlines — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
This checkpoint adds viewport keyboard routing and native automatic advancement.
**The component remains incomplete:** native pointer gestures, full offscreen/
partially visible focus and AX policy, gallery/public-driver coverage and physical/
resource/release qualification remain required. No physical window was opened.

## Behavior

The viewport has a retained native focus handle. Home/End and arrows on the model's
axis enqueue ordered requests only when the viewport itself is focused and fresh
geometry has been published. Modified keys and bubbled descendant keys retain
normal ownership. Requests leave controlled selection unchanged until OCaml reduces
them; the existing external Previous/Next buttons keep their Core callbacks.

Each mounted source owns at most one cancellable automatic deadline. It uses the
existing checked geometry/clock state, executor time, a source-specific ticket and
weak View ownership. It waits for settled visible paint; hover, contained focus,
inactive/reduced motion, pointer capture, application dragging or focus/modal
restrictions pause eligibility. Updates and every wake recheck current state.
Resuming waits a full interval, while ordinary frames preserve an eligible deadline.
One emitted proposal remains pending until model or measured geometry changes.
There is no polling or synchronous OCaml callback.

A new geometry epoch retires pending proposals even if its canonical stop map is
unchanged. Rebinding the handler creates fresh native ownership; unmount/window
close cancel tasks and subscriptions. Only accepted current-source paint marks
movement settled for scheduling.

## Tests and limits

Three added host tests use the real GPUIO adapter on GPUI TestPlatform:

- Both-axis Home/End/arrows, ordered repeated requests, wrong-axis/modified keys,
  descendant button precedence, disabled input and unchanged controlled selection.
- Exact interval boundaries despite intermediate frames, one outstanding proposal,
  acknowledgement, hover/descendant-focus pause with full-interval resume, unmount,
  released weak state, activation-subscription removal and zero retained tree bytes.
- Settled-motion gating, changed geometry with unchanged stop map, new proposal
  epochs, reduced motion, deactivate/reactivate, hide/show, handler replacement,
  old-deadline cancellation and full window-close cleanup with an armed timer.

The lifecycle test initially expected immediate resume after reactivation while
TestPlatform had reset its pointer to `(0,0)` inside the track. The test now moves
outside before checking resume. The production hover pause was correct and was
not weakened. The earlier motion/renderer tests also run as regressions.

These are simulated-platform keyboard/layout/routing/timer checks, not physical
keyboard/IME, VoiceOver, hardware GPU, Linux GUI or installed GUI acceptance.
No public API shape, protocol payload or dependency pin changes in this checkpoint.
The preceding installed Core/Bonsai API check remains prior package evidence;
this change is validated in the local native host.

## Validation

All commands use the repository's isolated environment, `GPUIO_JOBS=2`:

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

Passed: **683 native library tests/two existing private-D-Bus skips**, four
transport tests, strict all-target lint and full OCaml tests/format/gallery build.
The focused track subset contains 22 passing tests. Catalog structure and
whitespace checks also pass. No hosted CI or physical platform acceptance is
claimed.
