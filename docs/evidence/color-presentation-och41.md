# Color presentation evidence — OCH-41

2026-10-02, local macOS dirty worktree based on `83eb87e`. Labeled grouped/featured
palettes and bounded internal appearance are available through the public Core
and Eio APIs. The gallery uses nine color families and a larger featured row;
its compact popup example bounds its height with ordinary scrolling style.

Three new Core expect tests cover section/count/label/geometry invariants,
duplicate colors, mutual exclusion of flat/sectioned inputs, theme resolution,
atomic failure and presentation-only reconciliation without owner replacement.
The Op87 bytes match an independently constructed fixture. A new Bonsai-driver
test changes appearance and sections while a popup is open, checks the same input
ID, resets presentation and verifies no application-value commit.

A native TestPlatform test verifies retained editor entities, marked text, model
snapshot/revision and palette focus handles across grouping and styling. It
checks group AX labels, distinct duplicate-color slots, featured/ordinary widths,
native pointer selection, focus after regrouping, and default reset. Existing
hover and color state tests continue to pass. Final full native command:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline
```

**621 passed, two existing private-D-Bus skips.** Scoped native color checks pass
14 tests. An additional native admission test checks exact retained metadata
accounting, wrong kinds, palette-count mismatch, rollback and combined changes
in either operation order. The new strict protocol fixture test checks every
truncation, trailing bytes, labels/counts/section limits, geometry and colors:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol --test color_input --test color_presentation --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
```

The scoped check passes nine native tests and one protocol test. The complete
protocol suite passes **321 tests**. Existing color config/snapshot/command
fixtures remain unchanged; the optional presentation is a separate operation.

Strict lint and the full OCaml tests/format/gallery command pass after the native
paint adjustments:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

Rustfmt, catalog audit and whitespace checks pass. Existing `block`
future-compatibility and duplicate-system-library linker warnings remain.

No OS windows were opened. TestPlatform exercises native owners, layout, hit
testing, the editor bridge and AccessKit, not physical OS IME, VoiceOver or GPU
readback. The checkerboard uses fixed native quads with a whole-swatch rounded
mask per tile; physical large-radius/theme/scale visuals remain an OCH-17 check.
Shared popup-trigger semantics are recorded in the
[picker evidence](picker-triggers-och41.md); tab validation follows below.
See the [contract](../design/color-presentation.md).


## Native Palette/HSLA tabs

2026-10-02 continuation, same dirty base `83eb87e`. `Panels.all/tabs` and the
optional `Appearance.panels` now reach the same native input in Core/Eio views
and plain/rich popups. Gallery inline/popup examples use Palette and HSLA labels.
The updated desktop walkthrough has been syntax-checked, not executed here.

Five new TestPlatform tests exercise initial selection on mount, All/Tabs
changes, retained editor identity and snapshots, one roving tab stop, Left/Right/
Home/End, native mouse and AccessKit Click/Focus, and panel roles/selected state.
They verify composition preservation in visible hex under presentation updates;
valid/invalid/composing channel settlement when hidden (including pending native
changes before observer delivery); user activation's blur policy; capture cancel,
release and late mouse-up; rejected hidden/opaque-alpha actions; readonly,
disabled/inert/hidden/pointer/modal gates; stale handlers and owner/editor teardown.
Explicit Focus reveals an eligible channel and cancels a held drag.

The first tests exposed two defects, repaired before acceptance: AX Click's
fallback synthesized mouse-up into a captured drag, and GPUI mouse-down moved
focus before the activation handler checked the old editor. Direct AX actions
and settlement by native stamp/interaction fix both; regressions pass. These
changes are in the GPUIO adapter, without changing the vendored GPUI widget.

A fourth Core test validates tab labels and presentation-only reconciliation.
The popup-driver test now checks tabs on the same open input with no app commit.
Both languages match the extended independent Op87 fixture; strict Rust checks
reject unknown panel/initial tags and bad labels, and native admission verifies
label storage accounting and atomic failure. Nine native admission and two
protocol presentation tests pass. Earlier color policy/command bytes are unchanged.

The full native command above passes **626 tests, two existing private-D-Bus
skips**. The full OCaml `@runtest @fmt` plus gallery build passes. These are local
native-host and driver tests with no OS window; physical IME/keyboard, VoiceOver,
GPU and Linux desktop coverage remain unclaimed. The complete protocol suite
passes **322 tests, no skips**, and the strict all-target native/protocol Clippy
command above passes. Rustfmt, catalog audit and whitespace checks pass.
