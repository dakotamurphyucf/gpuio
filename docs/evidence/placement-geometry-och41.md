# Point/corner placement evidence — OCH-41

2026-10-02, macOS/arm64 checkout `milestone-07-gallery-release`, base `83eb87e`
plus uncommitted milestone work. These tests use GPUI TestPlatform and do not
qualify physical GPU, keyboard/IME or VoiceOver behavior. No OS window was opened.
See the [contract](../design/placement-geometry.md) and
[pinned source review](../catalog/overlay-review.md).

Four Core expect tests cover checked coordinate/margin construction, retained
popup/child IDs across geometry-only changes and reset, and independently
constructed Op93 bytes. Public Tooltip/HoverCard/menu helpers also submit the
geometry correctly while dialogs omit the unsupported metadata. One Rust codec test covers all four corners, margin-only
and reset records, every truncated prefix, invalid corner tags, nonfinite and
out-of-range values, and trailing bytes. One native admission test verifies
atomic target/config validation and final-owner removal/kind changes; rejected
transactions retain the old revision and allocation count, and teardown returns
retained tree bytes to zero.

Three positioner tests cover side/alignment/gap/flip behavior and point corner
math, client inset, edge clamping, zero/oversized margins and oversized content.
Five native host tests exercise:

- All four actual popup corners, painted bounds, hit testing, accessible identity,
  retained native button/focus owners, outside dismissal and anchor restoration.
- Current-viewport resize without a transaction and bottom-edge clamp without a
  side flip.
- Controlled Tooltip and HoverCard point placement and live geometry/reset with
  retained content owners.
- Point-placed menu roots with independently anchored/flipped submenus, shared
  viewport margins, retained navigation and native command activation.
- Existing ordinary absolute dialog insets with modal accessibility, autofocus
  and background pointer blocking. No additional dialog positioning API is needed.

Initial host harness failures were corrected to use TestPlatform's resize event,
respect outside-pointer focus changes before expecting restoration, and compare
submenu placement with the actual row bounds rather than the whole popup width.
Those were test assumptions, not production defects.

Focused checks passed:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/view_api/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 --offline \
  -p gpuio-protocol -p gpuio-native --test placement_geometry
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 --offline \
  -p gpuio-native --features native-image-tests --lib host::popup::tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 --offline \
  -p gpuio-native --features native-image-tests --lib placement_geometry_test
```

The public gallery offers an open popup with live point/anchor, corner and margin
controls. Physical visual/keyboard/AX acceptance and whole-ticket/milestone
acceptance remain open. Sheet platform-safe-inset mapping remains separate work.

The full native library suite passes **651 tests with two existing private-D-Bus
skips**. Full protocol passes **330 tests with no skips**. Full OCaml tests,
formatting and gallery build pass; the structural catalog audit also passes.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --offline --features native-image-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/audit_component_catalog.py
```

Strict Rust lint passes with warnings denied:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
```

A fresh installed public-package gallery consumer also passes:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/placement-installed-gallery
```

Result: `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
The script stages packages separately without changing the opam switch. This
proves the copied gallery/backend builds against the installed public interface;
it is not packaged-runtime or clean-machine distribution acceptance. All check
processes finished. The fourth Core helper regression and formatting check also
passed after the full suite, with no later production change.
