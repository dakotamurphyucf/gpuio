# Calendar presentation evidence — OCH-41

2026-10-02, local macOS dirty worktree based on `83eb87e`. The public calendar and
date-picker view APIs now accept a bounded `Calendar.Appearance`. It controls
1..12 months, geometry and theme colors. The gallery offers 1/2/3/12 month choices
and uses its appearance/scale controls for the calendar and date popup.

## Verified behavior

Two Core expect tests check count/geometry validation, theme resolution, missing
theme atomic failure, appearance-only reconciliation, stable calendar identity,
reset to defaults and exact independently encoded Op86 bytes. The entire OCaml
test suite, formatter and gallery build pass.

Two native viewport tests cover stable first-pane behavior while moving within a
span, minimal scrolling when moving beyond it, count changes and both civil-domain
boundaries for all counts 1..12. Seven existing calendar state tests also pass.

A native TestPlatform test mounts the actual retained host with two months,
selects a range with keyboard input across February/March, and verifies:

- Shared selection, valid cursor-month snapshot and stable first displayed month.
- Exactly one target for dates repeated in neighboring six-week grids.
- AccessKit focus identifies the cursor day.
- Explicit ShowMonth aligns the first pane even when the cursor already belongs
  to the requested month; appearance-only changes retain revision/focus/owner.
- Three months wrap in a narrow container while retaining configured cell height.
- Reset returns to one month and removal releases the mounted owner.

The AX harness initially looked for an `active_descendant` node property; GPUI
exports the active day as AccessKit tree focus. The geometry harness initially
compared a logical 40-point height with physical AX pixels; it now uses the test
window scale factor. Neither correction weakens the intended behavior.

The complete native library suite passes **617 tests, with two existing private-
D-Bus skips**. The new native transaction test rejects wrong kinds/invalid values
atomically, checks allocation accounting and default-reset release. Together with
existing native and protocol calendar tests and the new codec fixture, the scoped
integration command reports **13 passing tests**. Truncations, trailing bytes,
invalid counts, geometry and colors are rejected by the Rust decoder.

## Commands

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --features native-image-tests --lib calendar_ --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol \
  --test calendar --test calendar_presentation --features native-image-tests --offline
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --features native-image-tests --lib --offline
```

These commands pass. The final native run includes the narrow-layout assertion;
only the native test harness changed after the full OCaml/gallery check.
The full protocol suite also passes **320 tests**. Strict all-target Clippy passes
for native/protocol with `native-image-tests` and warnings denied:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
```

Catalog audit and `git diff --check` pass. Final public documentation clarifies
single-month flexible width and explicit ShowMonth alignment; those doc comments
pass the pinned formatter independently. No production behavior changed after
the full-suite checks.

No OS window was opened. TestPlatform does not qualify physical macOS keyboard,
IME, VoiceOver, GPU output or Linux desktop behavior. Measured resources, installed
consumers and release qualification remain open. The paired backend/library must
both understand Op86; existing calendar snapshot bytes are unchanged. See the
[contract](../design/calendar-presentation.md) and the
[pinned source review](../catalog/calendar-color-review.md).
