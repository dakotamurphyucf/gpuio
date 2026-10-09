# Cartesian value directions — OCH-41

Local source overlay on `529230a`, 2026-10-06, macOS 14.5 arm64 / M1 Max.
The [contract](../design/chart-directions.md) adds `Vertical_reversed` and
`Horizontal_reversed` to the public OCaml options and paired native representation.
Existing tags/default bytes remain unchanged; both bridge packages must understand
new tags 2/3. No dependency, synchronous OCaml render callback or new timer is added.

A shared numeric-value projection now governs signed marks, curves, area baselines,
ticks and grids. The category order, source identity, numeric domain and original
data remain unchanged. Horizontal reversal retains height-based reduction and
hit lookup. Bar gradient stops mirror with the value axis. Axis gutters and the
non-Cartesian family layouts keep their existing behavior.

## Validation

- Paired codec tests: all four tags round-trip; Rust independently accepts literal
  tags 2/3 and rejects 4. The old default fixture remains byte-identical.
- Geometry/provenance: signed mixed line/area/bar layers, explicit gaps, three curve
  types, mirrored marks/paths/ticks, and unchanged original source references.
  A narrow/tall 1,000-point plot checks height-based horizontal reduction.
- Native chart-focused tests: 63 pass. Full native library with image/canvas
  features: 987 pass, two existing ignored tests. Existing hit-index tests exercise
  all four directions, including bounded queries over 100,000 marks.
- Actual GPU readback: all 14 chart/gradient cases pass at scales 1, 1.25, 1.5 and
  2, including positive bars growing down/left/right and reversed gradient pixels.
  These hidden-window checks do not establish physical keyboard or VoiceOver behavior.
- Fresh installed consumer: the public gallery builds against the staged package
  and passes the native chart walkthrough. The final driver waits for the exact
  direction status, so `Vertical` cannot accidentally match `Vertical reversed`.
  It exercises seven families and mixed layers, all four directions, identical
  Capacity selection via Home/Enter, data publication, original-data pages,
  disabled state, themes/sizing and repeated page transitions. Images, charts,
  canvases and registered source bytes return to zero; the process/window exits.
- The final captured vertical-reversed and horizontal-reversed views were inspected:
  values grow down and left respectively, mixed layers share their projection,
  ticks reflect the reversed domain and selection still reports Capacity x=0/y=30.
- Strict all-target Clippy, OCaml build/expect/format checks and Rust formatting
  pass. Catalog/link/Python syntax checks cover the changed documentation/driver.

Exact commands (all Cargo and Dune commands use the repository environment):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol --test chart_options
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests chart_
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-canvas-tests --test native_chart_paint
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section charts --workspace scratch/agents/root-20261004-resumed/chart-orientation-consumer
python3 scripts/test_gallery.py --executable scratch/agents/root-20261004-resumed/chart-orientation-consumer/consumer/_build/default/main.exe --section charts --images scratch/agents/root-20261004-resumed/chart-orientation-images
```

Installed consumer SHA-256:
`b6ab2aee84a81f9a35bf631f2a0f73de1afa6f488a05f5aeae6b0e4b10234ad3`.
The [archive](chart-directions-och41/evidence.tar.gz) and
[manifest](chart-directions-och41/manifest.json) preserve the source overlay,
commands, raw logs and owned-window screenshots. The new adjacent
[chart-page walkthrough](../../examples/gallery/charts_page.md) explains the
application, Bonsai, GPUIO and scoped resource boundaries for OCH-48.

This is scoped local qualification. Categorical point/band scales, stacking,
additional labels/tooltips and per-datum options remain chart catalog work.
Current-source hosted checks, broader physical/VoiceOver/performance acceptance,
Linux desktop qualification and final release gates are not established by this
change. The live hosted run at the older `23650c8` checkpoint was preserved;
subsequent changes are published on the review branch until it finishes.
