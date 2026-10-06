# Categorical and boundary axis visibility — OCH-41

2026-10-06, macOS 14.5 arm64; source base `4fe365b` plus this repair.
Reviewing the remaining chart-axis catalog work exposed two existing rendering
defects. This fixes the current `Axes.x`/`Axes.y` contract; it does not complete
custom tick, axis placement, label styling or grid-dash APIs.

Categorical geometry intentionally has no numeric x-domain. The painter used
that domain's presence to decide whether to emit **either** axis line. Categories
therefore had tick labels and optional grid lines but no axis strokes. A new
unit regression failed with zero meshes where one x-axis mesh was required.
The painter now admits Cartesian, categorical and candlestick axes explicitly,
while retaining the absent categorical numeric domain.

Actual GPU readback independently found a second defect in a numeric chart:
at scale 1, the vertical axis's one-logical-pixel stroke was centered on the left
plot boundary, half clipped, and missed every sampled pixel. The new pixel test
failed before the repair. Boundary strokes now sit half a logical pixel inside
the clip (bounded by half the extent for tiny plots). Source projections, tick
positions, hit tests and selection data are unchanged. Both x/y flags still
refer to data axes and transpose with the existing orientation rules.

The final GPU harness passes 128 new numeric/categorical cases: each x/y flag
combination, all four orientations, and test scale factors 1, 1.25, 1.5 and 2.
It samples actual green axis pixels separately on the left and bottom boundaries,
checks disabled boundaries stay absent and the plot interior remains black.
All existing family, stack/radius/gradient and clipping cases pass too.
These are hidden-window GPU tests with synthetic scale overrides, not physical
monitor transitions, foreground input, VoiceOver or presentation timing.

All 1,032 native unit tests pass, with the two existing skips unchanged. The
production `native_chart_view` harness also passes, including font/label pixels,
source publication/reset, radar input/resource lifetime and two-window streaming
cleanup. Strict Clippy with both native test features and Rust formatting pass.
No protocol or OCaml API changed, and no new public-gallery/installed-consumer
acceptance is claimed for this repair. Their preceding pie-caption results remain
scoped to `4fe365b`. No test window/process or temporary VoiceOver setting remains.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib categorical_axis_strokes_do_not_require_a_numeric_x_domain --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

The [before/after logs](chart-axis-visibility-och41-logs.tar.gz) and
[verified manifest](chart-axis-visibility-och41-manifest.json) retain the original
failures and final checks. No assertion, rendering threshold or release gate was
relaxed to obtain the passing result. OCH-41/OCH-17 remain open. Live hosted run
37515832448 covers the preceding `4fe365b`; this repair still needs current-source
Linux/hosted qualification. Linux desktop qualification remains OCH-47.
