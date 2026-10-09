# Shared scrollbar values, codec and geometry — OCH-41

Local checkpoint, 2026-10-03, macOS checkout, base `83eb87e` plus the milestone
working tree. This implements the first part of the
[scrollbar contract](../design/scrollbar-presentation.md).
**View attachment, native rendering/input/accessibility, runtime ownership and
gallery integration remain unfinished.** Existing ordinary scrolling and legacy
list/table bars are unchanged by these standalone modules. OCH-41 stays open.

## Implemented

Core `Scrollbar` supplies checked axis/mode, label, base/hover/pressed track and
thumb appearance, and finite motion descriptions. Geometry is finite 0–16384
logical pixels; times are 0–60 seconds, rounded up to milliseconds. All theme
tokens resolve before encoding, including inactive states. Shared
`Style.Expert.background_to_wire` reuses existing solid/sRGB/Oklab conversion.

`Wire.Scrollbar` and Rust `scrollbar::Config` have paired layouts and independent
43-byte default / 259-byte full-state fixtures. The fixtures were constructed
from primitive bin_prot encodings, separately from both serializers. The initial
fixture generator swapped the int16/int32 marker constants; comparison caught
that error and the fixtures were corrected (FE=int16, FD=int32). Production
encoders were not changed to accommodate it. Rust's standalone decoder checks
its 8192-byte input bound, bounded label, variant/option tags, dimensions,
durations, resolved colors/gradients and exact consumption. There is no new
attaching operation or existing operation-layout change in this checkpoint.

Native `scrollbar_geometry` is a pure geometry/drag projection module. It works
in positive offsets and viewport-local coordinates; the future adapter must
convert the existing handle convention. It measures both axes together, reserves
only eligible corners, clamps tiny viewport envelopes, insets, radii and minimum
thumb lengths, and retains a stable maximum-width interaction envelope. Captured
dragging reuses the pixel grip with current measurements. Zero thumb area or zero
travel produces no pointer adjustment. No offsets, callbacks or native handles
are owned by this model.

Native `scrollbar_presentation` resolves rest, track hover, thumb hover and
pressed parts against a supplied palette, without global-theme mutation.
Pressed inherits the base rather than hover-only values; transparent overrides
and gradient spaces remain exact. Default colors follow the supplied foreground.
These parts are not yet painted by the production Host.

## Verification

Commands use the repository's isolated environment with `GPUIO_JOBS=2`:

| Command after `./scripts/gpuio exec` | Result |
| --- | --- |
| `cargo test --offline --locked -j2 -p gpuio-protocol` | 357 passed, no skips; three scrollbar codec tests include independent fixtures, every truncated prefix, unknown tags, trailing bytes, geometry/time/label/color/gradient boundaries |
| `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib` | 760 passed; two existing private-D-Bus tests skipped outside their required isolated bus |
| `cargo clippy --offline --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings` | Pass |
| `cargo fmt --all -- --check` | Pass |
| `dune build @test/view_api/runtest` | Pass; independent bytes, checked constructors, duration rounding and theme resolution; initial expectation whitespace reviewed and explicitly corrected |
| `dune build -j2 @runtest @fmt examples/gallery/main.exe` | Pass: full OCaml tests, formatting and gallery build |

The native additions contain six geometry tests (including 2,000 generated
cases) and three cascade tests. They cover zero/oversized dimensions, both-axis
corner reservation, monotonic clamped drag projection, resize during capture,
nonfinite inputs, large finite extents, stable hover envelopes, inherited versus
transparent styles, palette changes and invalid hidden-state rejection.

The catalog structural audit and `git diff --check` also pass. No dependency
pins or vendor files changed for this foundation.

The complete native suite uses pure models and TestPlatform. No OS window was
opened. This is not physical macOS keyboard, mouse, IME, VoiceOver or GPU evidence,
and no new Linux qualification is claimed. Remaining verification and runtime
integration must be recorded before feature/catalog acceptance.
