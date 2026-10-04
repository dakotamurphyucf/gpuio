# Managed table presentation — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`.
Core/Bonsai table configuration now carries `Table.Appearance`: native stripes,
thirteen theme-resolved part colors and shared/per-column leaf-header/body padding.
The Result table gallery adds Striped rows, Table part colors and Compact cell
padding. See the [contract](../design/table-appearance.md). This does not complete
physical GPU/desktop qualification or the other table catalog gaps.

## Implemented behavior and checks

- Appended optional Op112 preserves existing Config/Column/Behavior encodings.
  Independent Rust/OCaml fixtures cover set/clear. Decode validates bounds,
  unique parts/IDs, finite padding, RGBA range and unknown tags; native admission
  additionally validates owner, current schema and retained-byte quota atomically.
- Colors and stripe mode update without advancing geometry revision. Shared or
  per-column padding changes/reset advance it; stale geometry input is fenced.
  Core reconciliation tests cover theme changes, missing tokens, padding and reset.
- The production Host TestPlatform test preserves native entity identity, cell
  selection and keyed vertical offset. It measures changed cell bounds for zero
  padding and verifies shared/per-column precedence and reset. Internal part
  colors are distinct from root appearance, so Column_border does not change the
  outer frame's color. These are model/layout checks, not GPU pixel evidence.
- A short striped table exposed a real panic in the old host delegate: decorative
  filler rows attempted to look up logical data IDs. The regression reproduced it;
  filler rows now have inert native identities, without OCaml records or focus
  handles. Short/empty transitions retain bounded tree/handle counts; the final
  accessibility assertion exposes exactly one semantic row for one data row.
- Header background now paints once, instead of again inside pinned/scrolling
  sections. Explicit header foreground applies without disabling root font
  inheritance. The scoped extraction adaptation is documented in
  `rust/table/UPSTREAM.md`.

## Validation checkpoint

The complete native run after the filler fix passed **808 library tests** with
two existing private-D-Bus skips, plus **9 table admission tests**:

```
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test tables
```

Full protocol tests passed **364 tests**, no failures/skips:

```
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol
```

The final short-table accessibility assertion passed in a focused repeat of the
production Host test. Strict lint also passes:

```
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-native -p gpuio-table-adapter --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings
```

The full OCaml suite and gallery build passed, including three new expect tests
for paired values/validation, reconciliation/theme/geometry and retained Bonsai
cell lifetimes/selection through appearance changes and reset:

```
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe
```

`GPUIO_JOBS=2 ./scripts/gpuio check-fmt`, `git diff --check` and the structural
component catalog audit passed. A fresh installed-gallery build also passed:

```
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-table-appearance-gallery-20261003
```

This staged the public packages and independently compiled/linked the complete
gallery (`run=False`), including the final native filler fix. It did not install
into another switch or launch an OS window.

## Physical evidence still required

A GPU readback case has been added to the existing native table style test. It
checks a translucent header in pinned and scrolling regions, explicit header
foreground, row-gap stripe coverage and reset. It has **not run** in this session.
No OS windows were opened; TestPlatform checks do not establish macOS rendering,
physical input, VoiceOver, performance or resource acceptance. Required Linux
non-GUI checks remain separate; OCH-47 Linux desktop qualification stays deferred.
