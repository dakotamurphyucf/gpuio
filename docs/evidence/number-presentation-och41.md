# Numeric input presentation — OCH-41

Local checkpoint, 2026-10-02, macOS, isolated repository toolchain, dirty worktree
based on `83eb87e`. Native rendering/input tests use GPUI TestPlatform; no OS
windows were opened. This is not physical macOS input, IME, VoiceOver, visual,
resource or release qualification.

`Number_input.Appearance` and `View.number_frame` add integrated adornments,
passive custom step content, bounded geometry and theme-resolved part styles to
the same retained native numeric editor. The Numbers gallery exposes frame and
custom-symbol switches, an interactive Qty prefix and units suffix. See the
[contract](../design/number-presentation.md).

Verified behavior:

- Core rejects inappropriate geometry, layout/state styles, interactive step
  content and nonnumeric roots. Adding/removing the frame preserves the numeric
  owner; surviving role identity remains stable. Theme-only changes send only
  presentation; a missing theme token fails without advancing reconciliation.
- Independent Op82 bytes agree with OCaml encoding and Rust decoding. Decode
  checks cover truncation, trailing data, geometry bounds and list limits.
- Native admission validates all four structural roles, passive button content,
  kind, geometry and styles; invalid changes roll back revision and retained
  payload accounting. Reset removes presentation and content atomically.
- TestPlatform verifies full-width focused frame paint, the same InputState and
  unfinished draft/committed value, retained undo history and provisional Unicode
  composition during appearance changes. Native Up stepping still works.
- Base/hover/pressed button colors are painted. A native hold starts its timer,
  repeats after 400 ms, and releases capture/timing when new button geometry
  moves its target. Side/stacked/hidden layouts preserve editing state; hidden
  controls remove their AX button nodes. Config updates correctly advance the
  numeric revision even when draft/value/selection/focus stay unchanged.
- Queued auxiliary AX activation observes a newly disabled numeric parent.
  Read-only leaves the independent auxiliary action available. Removing the
  frame disposes its ordinary button owner while retaining the numeric editor.

The first repeat assertion failed because the test window was inactive. Explicit
TestPlatform activation corrected the setup; production eligibility still
requires an active window. This did not activate a physical macOS window.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test -j2 -p gpuio-protocol --offline`: **314 passed**.
- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --test number_input --offline`:
  **585 library tests and six numeric admission tests passed**, two existing skips.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passed.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: passed.
- `cargo fmt --all -- --check`: passed.

`python3 scripts/audit_component_catalog.py` and `git diff --check` pass. These
changes remain local and uncommitted. Current CI, required Linux automated checks,
physical macOS acceptance, measured resources and clean-consumer distribution
remain open. Full Linux desktop qualification remains deferred to OCH-47.
Application-controlled/current-value step policy is still a separate unimplemented
catalog gap. This checkpoint does not complete OCH-41 or milestone 07.
