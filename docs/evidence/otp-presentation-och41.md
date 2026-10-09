# Segmented OTP presentation — OCH-41

Local checkpoint, 2026-10-02, macOS, isolated toolchain, worktree based on
`83eb87e`. These checks use GPUI TestPlatform and open no OS windows. This is
local automated evidence, not physical macOS input/IME, VoiceOver, visual,
resource or release acceptance.

Public `Otp_input.Appearance` and optional Core/Eio view arguments now configure
grouping, bounded cell dimensions and theme-resolved paint colors without
replacing the native editor. The gallery has grouping and size toggles with
palette colors. See the [contract](../design/otp-presentation.md) for exact
ceil-sized grouping, focus-border treatment and composition behavior. Caret
blink was separate at this checkpoint; its subsequent implementation and checks
are recorded in [caret evidence](otp-caret-och41.md).

Two Core expect tests cover constructor boundaries, changed-only reconciliation,
theme changes, native owner retention, default normalization/reset, no fabricated
editing events and atomic unresolved-token rejection. They also compare Op81
against an independently constructed fixture. Rust checks the same fixture,
round trip, truncation/trailing bytes and invalid settings. An independent
sequential geometry calculation agrees for all 1–32 lengths and group counts.

A retained native TestPlatform check types into a real OTP model, then changes
appearance. It verifies six painted background/focus borders, ordinary/group
gaps, total width, shared caret/hit geometry and unavailable stale IME bounds
before the new layout. Native identity, complete snapshot, undo availability
and focus survive. Starting provisional Unicode composition, changing grouping
again and resetting defaults keeps the composition as one continuously shaped
field. This exercises native model/layout behavior, not a macOS input method.

Native admission checks retained policy accounting and release on reset,
unchanged seed, wrong-kind rejection and atomic rollback of invalid compound
updates. No vendor patch, additional native input owner or timer is introduced.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test -j2 -p gpuio-protocol --offline`: **313 passed**.
- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --test otp_input --offline`:
  **581 library passes, two existing skips; seven OTP admission passes**.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: passed, including
  the two Core/fixture tests and the updated public gallery.

`python3 scripts/audit_component_catalog.py` and `git diff --check` also pass.

Actual macOS and required Linux automated checks against the final release
revision remain required. Linux desktop qualification is deferred to OCH-47.
All changes here are local and uncommitted; this does not complete OCH-41.
