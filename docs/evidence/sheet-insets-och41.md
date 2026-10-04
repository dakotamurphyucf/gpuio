# Sheet inset evidence — OCH-41

2026-10-02, macOS/arm64 checkout `milestone-07-gallery-release`, base `83eb87e`
plus uncommitted milestone work. These are native GPUI TestPlatform checks, not
physical GPU, keyboard/IME or VoiceOver qualification. No OS windows were opened.
See the [contract](../design/sheet-insets.md).

The pinned archive was checked against SHA256
`909c00c97bbfce11607eef7d3eccd01201d591af9d26cd9ebadfe0c90fca502f` before extracting
the unmodified `component/window_border.rs` and `component/title_bar.rs` snapshots.
The catalog manifest records each source hash. The source distinguishes a custom
client frame's shadow/border insets from a theme top margin of 34 pixels. GPUIO's
explicit inset contract maps application layout without copying that wrapper's
offsets into unrelated OS windows.

The native test found a real containment defect: padding/borders forced a
34-pixel minimum box even after sheet width/height had been constrained to one
pixel. Native sheet layout now bounds decoration contributions across base and
highlight states. Percentage padding uses the containing width, matching the
pinned Taffy implementation. This fixes painted and accessible bounds together;
it does not merely change the expected geometry or hide an oversized panel.

Four native host tests cover:

- Every edge with reserved space; actual paint/AX bounds, modal flag, autofocus,
  background input blocking, accepted dismissal versus requests, live reset,
  retained native editor focus handles/button owners, clicks and restoration.
- Native resize to a one-pixel panel, Escape dismissal while tiny, growth back to
  ordinary geometry, edge changes and retained native editor handles.
- Inset changes halfway through entry without restarting time or identity;
  eventual autofocus and no idle frame requests after settling/removal.
- Overlapping focus and actual hovered presentation, percentage/large padding
  and border widths, restored ordinary styles and stable panel containment.

A pure geometry test covers proportional compression, zero/subpixel viewports
and all four extent directions. The existing top-sheet entry-focus regression
also passes in the focused run. One independent codec test covers four edges,
explicit zero and reset bytes, every truncated prefix, each invalid float field
and trailing input. One atomic admission test covers invalid node targets,
nonfinite/negative values, owner removal/kind changes, unchanged revision/retained
bytes on rejection and zero retained bytes on teardown.

Focused native/codec/admission checks passed:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 --offline \
  -p gpuio-native --features native-image-tests --lib sheet_
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 --offline \
  -p gpuio-protocol -p gpuio-native --test sheet_insets
python3 scripts/audit_component_catalog.py
```

The gallery exposes reserved app-chrome space and all four edges using public
Core/Bonsai APIs. Full regression, public-package consumer and physical desktop
qualification are tracked separately below as they complete. This checkpoint
does not complete OCH-41 or milestone 07.

Full native library regressions pass **656 tests with two existing private-D-Bus
skips**. Full OCaml tests, formatting and the gallery build pass. Two new Core
expect tests verify finite bounded edges, omission of default metadata, retained
sheet/child identity through inset/reset updates and independent fixture bytes.
The initial full build found a Dune formatting newline and an unqualified gallery
edge constructor; both were corrected before the successful final run.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --offline --features native-image-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

Full protocol passes **331 tests with no skips**; strict native/protocol Rust lint
passes with warnings denied:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
```

The fresh installed public-package gallery build passes:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/sheet-installed-gallery
```

Result: `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
Packages are staged without mutating the opam switch. The copied public gallery
and native extension backend build successfully. This is build acceptance only,
not packaged-runtime or clean-machine distribution acceptance. All check
processes finished; physical macOS and current Linux release gates remain open.
