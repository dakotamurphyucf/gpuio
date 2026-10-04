# Slider presentation checkpoint — OCH-41

2026-10-02, macOS, local dirty worktree based on `83eb87e`, isolated repository
toolchain. No OS windows were opened. This is build, codec/admission and
TestPlatform evidence; it does not establish physical desktop acceptance.

## Implementation

Core `Slider.Fill` / `Slider.Appearance`, optional Core/Bonsai/Eio view appearance,
Op84 and native retained presentation provide remaining-side single fill,
independent rail/thumb/focus colors and bounded geometry. Values, focus handles
and the same single/range owner survive visual changes. Target-size changes cancel
capture through the existing rollback/event path. The gallery exposes fill,
colors and larger thumbs alongside axis/scale/value controls. No dependency pin
changed; no timers were added. The initial static-presentation checkpoint is followed by interaction rings below.

See the [contract](../design/slider-presentation.md) and
[pinned behavior review](../catalog/slider-review.md).

## Local checks

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless specified:

- `dune build -j2 @check`: passed after public API/wire integration.
- `dune build -j2 @test/view_api/runtest`: passed; two new expect tests cover
  bounds, unresolved theme atomicity, reset, retained owner and independently
  assembled Op84 fixture bytes. No expectations were promoted.
- `cargo test -j2 -p gpuio-protocol --offline`: **317 tests pass**, including
  exact Op84 bytes, truncated/trailing payloads and malformed fill/dimensions/colors.
- `cargo check -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline`: passed before new test additions.
- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --test slider --offline`:
  **593 library tests and six slider admission/routing tests pass**, with two
  existing library skips. New admission checks reject malformed policy and wrong
  kinds atomically, account retained bytes and restore storage on reset.

The mounted test checks actual painted quad geometry/colors for both axes and
single/range values, focus/owner/revision retention, simulated pointer capture,
paint-only preservation, geometry-change cancellation and default restoration.
The first test draft attempted to begin a gesture outside input dispatch; normal
simulated mouse dispatch supplies the correct event lifetime. Test imports also
needed explicit GPUI/wire type aliases. Neither correction changed production
input behavior. No OS window, physical mouse/keyboard, VoiceOver or GPU image
readback is claimed here.

Final checks also pass:

- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`.
- `cargo fmt --all -- --check`.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`, including rebuilt
  native archives linked into the OCaml executables.
- `python3 scripts/audit_component_catalog.py` and `git diff --check`.


Physical macOS validation, resources, Linux automated checks, current hosted CI,
review/publication and distribution gates remain open for the milestone.


## Native interaction rings

The follow-up adds two fixed-size spring states per slider and one pending weak
frame callback. It uses the existing analytic solver and GPUI clock, with a
critically damped 180ms response, exact endpoints and a two-second maximum.
Hover derives from current native thumb targets; pressed feedback follows the
captured thumb. Painting changes no slider revisions, values, focus or bridge
observations. Admission reserves 512 additional bytes per slider for these
states/callback metadata; this is quota accounting, not measured RSS.

Two solver/lifetime tests and a mounted fake-clock test cover retarget continuity,
settling, actual zero pending frames at rest, no hover bridge events, reduced
motion (including during travel), captured pointer motion, range-thumb independence,
capture loss, disable/read-only, hidden display/visibility, opacity, pointer/inert
policy, offscreen placement, inactivity, node removal and Close with an externally
retained owner. Explicit Close now calls `sync_sliders` after session teardown.
This retires visual state/capture even if another native reference survives.

The initial test build required qualifying the ordinary Rust test attribute to
avoid GPUI's macro import; no recursion-limit change was made. New native tests
use simulated dispatch and clock advancement, not sleep or OS windows.

Checks with the same isolated prefix:

- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --test slider --offline`:
  **596 library tests and six slider tests pass**, with two existing library skips.
- After the final two lifecycle assertions, `cargo test -j2 -p gpuio-native --features native-image-tests --lib slider_view::ring --offline`:
  all three focused tests pass.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passes.
- `cargo fmt --all -- --check`: passes.

`dune build -j2 @runtest @fmt examples/gallery/main.exe` also passes after rebuilding
the native archive and linked executables. Catalog audit and `git diff --check`
pass. The wire payload is unchanged; the preceding 317-test protocol result remains
current. No expectations were promoted.
Physical visual/input/AX and resource validation, Linux automated/current CI,
review/publication and distribution acceptance remain open.
