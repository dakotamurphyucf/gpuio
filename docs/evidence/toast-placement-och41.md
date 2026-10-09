# Toast placement and window margins — OCH-41

Local macOS development checkpoint, 2026-10-03, base `83eb87e` plus the milestone
working tree. Uses the repository's isolated toolchain and pinned dependencies.
See the [contract](../design/toast-placement.md) and
[source review](../catalog/notification-review.md).

## Implementation

`Toast.Placement` supplies eight anchors and checked per-edge window insets.
`Toast.Stack.create ~placement` preserves the existing four-corner/default API;
combining `corner` and `placement` is rejected. Op105 sets or clears only placement,
without changing toast lifetime metadata, owners, handlers or active-time clocks.
Native admission validates stack kind/bounds, charges fixed optional metadata and
releases it on reset/removal. Center anchors align inside the usable inset rectangle.

Native placement clamps opposing insets proportionally to the viewport and bounds
stack width/height. Each stack's actual inset rectangle clips descendant focus.
An unusable rectangle hides accessibility content, removes input eligibility and
focused descendants, and pauses expiry while preserving native children. Becoming
usable restores input eligibility and schedules the native clock once. No permanent
frame or polling timer is added. Focused text editors retain their existing caret
animation independently of placement.

The Feedback gallery's **A quiet confirmation** cycles all eight placements and
toggles asymmetric reserved margins. These controls use the public Bonsai API.
Layered stacks, interaction expansion and coordinated motion remain required work;
this checkpoint does not claim them.

## Verification

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless otherwise stated.

- `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib`: **729 passed, two existing private-D-Bus skips**. The new production Host test covers all eight anchors with asymmetric insets, retained editor/close focus identity, reset to legacy margins, zero-area focus/input/AX exclusion, restore/typing and teardown.
- `cargo test --offline --locked -j2 -p gpuio-protocol --test toast_placement`: **one passed**. Independently assembled bytes check set/reset, truncations, trailing bytes, invalid anchor and all four invalid inset positions.
- `dune build -j2 @test/view_api/runtest @fmt examples/gallery/main.exe`: **pass**, including three new expect tests for checked construction, paired fixture and placement-only reconciliation/no-op/reset.
- Final `dune build -j2 @runtest @fmt examples/gallery/main.exe`: **pass**.
- `cargo test --offline --locked -j2 -p gpuio-protocol`: **351 passed, no skips**.
- `cargo test --offline --locked -j2 -p gpuio-native --test toast_placement`: **one passed**, including the final optional-metadata charge/reset assertions.
- `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings`: **pass**.
- Final focused native Host rerun adds direct deadline-state checks: an initially running 60-second timeout pauses while the usable rectangle is empty, remains nonterminal and resumes after clearing placement. This test **passes** alongside retained text, input and accessibility checks.

`GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
--workspace scratch/agents/root-20260929-m7-resumed/toast-placement-installed-gallery`
passes against freshly staged installed packages (`run=False`). This builds the
new public gallery controls without installing into an opam switch.
Scratch logs use `toast-placement-`
under `scratch/agents/root-20260929-m7-resumed/`.

A native regression caught the full-window wrapper normalizing away the inset
origin. Retaining a full-window outer element and placing the inset frame inside
it fixes all eight positions. The tiny-window regression then caught retained
focus on an invisible editor; native visibility gating now blocks typing and
provides a hidden semantic ancestor without destroying that editor.

## Remaining qualification

Tests use TestPlatform; no operating-system window opened. In the physical gallery,
save a notification and cycle anchors while it is visible, toggle margins, resize
the window and check hover/focus pause, close/timeout, both themes/scales and
VoiceOver. Verify the reserved title-bar space and pointer behavior in the actual
GPU-rendered app. This walkthrough is authored but unrun. Physical macOS, resource,
CI, Linux automation, distribution and release acceptance remain separate gates.
