# Automatic client frame — OCH-41

Local implementation checkpoint, 2026-10-04, macOS arm64. Worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`; changes are uncommitted. Dependency
pins and vendor patches are unchanged. This is not physical compositor acceptance.

## Delivered behavior

[Window_frame](../design/window-frame.md) supplies validated immutable geometry
through `Window.Config.create` and `App.open_window`. Linux Custom chrome requests
transparent client decorations. Actual server fallback and macOS use OS framing.
The native frame preserves the full platform shadow inset through tiling and
fullscreen while removing visual padding/borders independently per tiled side.
A root Border_color styles its border; the public custom-chrome gallery follows
its current theme. The wrapper introduces no native focus owner or OCaml callback.

Resize cursors and gesture dispatch share the same half-open GPUI hitboxes and
corner precedence. Initial independent inclusive gesture checks disagreed with
painted hitboxes at their far boundary; the regression caught that discrepancy
and production now uses one hitbox definition. Native resize admission rechecks
current geometry, active/resizable/fullscreen policy and competing capture.

Popups resolve within current content bounds, including asymmetric tiling.
Explicit point placement remains in drawable-window coordinates. Sheets, modal
backdrops, palettes, menus, choices, tooltips and toast stacks use content extents;
window-level deferred surfaces translate to its origin. Snapshot content dimensions
retain their previous raw drawable-viewport meaning. Explicit sheet insets apply
inside the frame and must not duplicate its shadow/border.

## Completed local checks

Commands use the isolated repository environment with `GPUIO_JOBS=2`.

- `./scripts/gpuio exec cargo test -p gpuio-protocol --test window --offline --locked -j 2`:
  **6 passed**. Independent OCaml/Rust configured-open bytes include both default
  frame floats. Rust rejects nonfinite/out-of-range geometry and all partial frame
  payloads; valid extreme boundaries round-trip.
- `./scripts/gpuio exec cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j 2`:
  **888 passed, two existing macOS private-bus skips**. Geometry covers all 16 tiled
  edge combinations, zero/tiny/normal extents, shadow extremes and fullscreen.
  Resize tests cover corners, disabled tiled edges, transparent gutters, half-open
  boundaries and overlapping small-window bands. Explicit TestPlatform client
  rendering checks content bounds and preserved focus through all tiling/fullscreen
  states at normal and small sizes. Popup tests cover asymmetric content origin,
  anchor translation and explicit points without double-counting client insets.

- `./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe`:
  **passed**, including the public frame-constructor/invalid-wire expect test and
  independent configured-open fixtures. Gallery links against the rebuilt backend.
- `./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j 2 -- -D warnings`:
  **passed**. Cargo retains its existing dependency future-compatibility notice
  for `block 0.1.6`; the macOS link step reports existing duplicate-library warnings.
- `./scripts/gpuio check-fmt`: **passed**. Catalog audit still accounts for
  146 pinned modules across 43 families; that is structural coverage, not complete
  behavior acceptance.
Local logs and per-ticket recovery notes are under the ignored
`scratch/agents/root-20261003-release-notices/`; they are not build inputs.

## Limits

TestPlatform rendering explicitly supplies client geometry on macOS; it does not
exercise a real X11/Wayland compositor or OS resize request. No new physical GUI,
IME/accessibility, hosted Linux or clean-machine distribution result is claimed.
Existing sheet/palette/toast regressions run against TestPlatform; real client
compositor overlay fitting and scale transitions remain Linux qualification work
in OCH-47. Standard/Hidden chrome retain their previous behavior.

The window family stays open for focused-input/global-selection helper mappings
and physical gallery qualification. OCH-41/OCH-17 and milestone 07 remain open.
