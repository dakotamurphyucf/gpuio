# Custom axis and grid presentation — OCH-41

2026-10-06, macOS 14.5 arm64, source base `f5875f5` plus this change.
The [contract](../design/chart-axis-presentation.md) exposes bounded immutable
`Chart_axis` and `Chart_grid` values through `Chart_style.create`. Applications
choose explicit numeric/category/physical-fraction ticks, preformatted captions,
per-caption color/font/alignment, line positions and independent line/label
visibility. Grids accept independent positions, widths, colors and odd/even dash
patterns. Native preparation never calls OCaml to format, measure or paint.

Style schema is **-4**; options/view/data remain **9/-1/1**. Matching OCaml/native
packages are required; older styles are explicitly rejected. Counts and text
lengths are checked before decoding allocations, retained storage is charged,
and excessive grid geometry fails with RenderLimit instead of partial rendering.
Both full tick lists and independently full pie/Sankey captions fit the validated
192-KiB style envelope. Original source values, IDs and selection stay unchanged.

## Local evidence

- Independent OCaml/Rust axis/grid bytes, malformed inputs, truncation, old
  schemas, numeric/text/color/count bounds, theme tokens and retained storage
  pass. The complete protocol suite passes **441 tests**.
- Geometry tests cover explicit/automatic ticks, four orientations, reversal,
  category-ID reordering and unknown/mismatched targets, empty captions retaining
  grid positions, constant domains and unchanged original paths/marks. Paint
  tests cover independent brushes, widths, positions and visibility. Odd dash
  cycles reset per line; cancellation and the 16,384-fragment limit are checked.
- Actual GPU readback passes **32 custom-axis/grid cases**: numeric/categorical
  sources, all four orientations and scales 1, 1.25, 1.5 and 2. It checks line
  placement/colors, odd dash painted/gap spans and absent default boundaries.
  The preceding 128 axis-visibility cases and existing chart families pass too.
- The production mounted-view harness checks actual caption font/color pixels,
  before/after placement, four orientations and hide/return. Existing pie,
  Sankey, radar, source replacement, streaming and teardown cases pass.
- Screenshot review of the first passing public walkthrough exposed a vertical
  endpoint clipped against the legend when the horizontal axis floated inside
  the plot. An independent regression failed with **14 instead of 28 pixels**
  of caption height. Custom vertical axes now reserve endpoint space; default
  gutters remain unchanged. Final native tests pass **1,042**, with two existing
  skips, and the mounted-view harness passes again. The repaired public screenshot
  shows the complete lowest tick caption.
- Workspace-wide strict Clippy, native-feature strict Clippy, Rust formatting
  and the full Dune `@all @runtest @fmt` checks pass. The full Dune run preceded
  the final Rust-only gutter repair; final native tests/lint and the rebuilt
  public gallery cover that repair. No OCaml implementation changed afterward.
- The public `chart-axes` walkthrough passes five presets on line, categorical
  and candlestick charts, orientation changes, original-value keyboard selection,
  complete original-data browsing, both theme transitions and source updates.
  Leaving the page reaches zero image/chart/canvas registrations and source bytes.
  Final root executable SHA-256:
  `096e62dc60b7376ad419d48721d1864d8c7b6efb25cf3a0bbb501834cca01f25`.

The fresh installed consumer builds, passes its catalog check and passes the same
complete `chart-axes` walkthrough with zero-resource cleanup. Its executable
SHA-256 is `6eceeefa0a7e896e18f6210d4fdcb993156b75f4599b238fedd4995d29aa1e75`.
No test process/window remains. An earlier driver run against a stale root binary
is excluded from acceptance; the rebuilt root and installed binaries above are
identified independently.

The [logs and before/after screenshots](chart-axis-presentation-och41-logs.tar.gz)
and [verified manifest](chart-axis-presentation-och41-manifest.json) retain 17
files (6,385,652 uncompressed bytes). The final focused protocol rerun also passes
after the initializer-only fix for hosted lint. No assertion or release gate was
relaxed to obtain these results.

The [pure preset walkthrough](../../examples/gallery/chart_axes.md) and
[page walkthrough](../../examples/gallery/charts_page.md) explain actual OCaml
constructors, Bonsai state/effects and theme-to-native updates. The documentation
inventory now covers 419 sources in 261 reviewed groups. That review is distinct
from native acceptance.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-axes --images scratch/agents/root-20261004-resumed/chart-axis-final-gallery-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-axes --workspace scratch/agents/root-20261004-resumed/chart-axis-final-installed
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
python3 scripts/audit_example_docs.py
git diff --check
```

GPU scales are synthetic hidden-window tests; the gallery driver exercises native
macOS controls and keyboard selection. Neither establishes VoiceOver, OS IME,
physical display transitions or presentation timing. No VoiceOver setting was
changed. Hosted/Linux qualification of this source remains required. OCH-41 and
OCH-17 remain open for broader catalog, accessibility, performance and release
requirements; Linux desktop qualification remains OCH-47.
