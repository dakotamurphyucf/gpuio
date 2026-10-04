# Color palette preview evidence — OCH-41

2026-10-02, local macOS dirty worktree based on `83eb87e`. The public gallery's
inline color input and popup picker share native swatch hover inspection. The
gallery includes an explanatory hint; application code needs no hover callback.

Three new tests in `rust/native/src/color_input_preview_test.rs` mount real GPUIO
native owners on GPUI TestPlatform, dispatch mouse input through hit testing and
inspect the editor bridge, layout and AccessKit tree. They verify:

- Invalid and marked drafts retain text, selection, composition, revision,
  history allocation and focus through hover. Model snapshots and event queues
  remain unchanged. Palette positions do not shift; selected AX state remains
  tied to the model. Duplicate colors retain separate slot identities.
- Read-only, disabled and alpha policies; ancestor pointer/disabled/inert/hidden
  gates; successful commands; handler and palette replacement; stale callbacks;
  removal and native-owner release all handle preview lifetime correctly.
- Read/rejected commands preserve inspection. Native channel capture suppresses
  it. Modal entry and the host's window-inactivity cancellation path clear it
  without manufacturing color edit events.

Passing full native check:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline
```

**620 passed, two existing private-D-Bus tests skipped.** The scoped three-test
run passed before the full check. No protocol bytes changed in this slice.

Strict Rust lint, OCaml tests/formatting and the public gallery build also pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native --all-targets --features native-image-tests --offline -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

The full native suite was rerun after adding the pointer-only policy assertion:
the composing keyboard draft remains intact while pointer interaction is disabled
and restored. Style transactions may emit their normal render acknowledgements;
hover alone emits nothing. Catalog audit and whitespace checks pass. Existing
`block` future-compatibility and duplicate-system-library linker warnings remain.

These are TestPlatform/bridge tests, not real OS IME, VoiceOver, GPU readback or
Linux desktop acceptance. No desktop window was opened. The gallery's physical
visual/input validation remains part of OCH-17. Grouped palettes, panel switching
and internal appearance still remain open in OCH-41. See the
[contract](../design/color-palette-preview.md).
