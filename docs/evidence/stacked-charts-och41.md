# Stacked Cartesian charts — OCH-41

Implementation based on `28aaee4`, 2026-10-06, macOS 14.5 arm64 / M1 Max.
The [contract](../design/stacked-charts.md) adds typed opt-in native cumulative
bars and areas, retaining the original source values and identity.

## Implementation and checks

The OCaml API adds `Chart_options.Stacking.Grouped | Stacked`; default plots keep
their existing grouped/zero-baseline behavior. Options envelope 3 appends the
stacking field after categorical layout. Explicit paired bytes cover the default
frame and tag values; previous v1/v2 view fixtures are rejected. Chart data and
compiler/Bonsai/GPUI pins are unchanged.

Bars and areas form independent source-order stacks, with line overlays untouched.
Native preparation rejects misaligned numeric layers within either stack family.
Bars aggregate under the explicit policy before accumulating aligned source
intervals. Areas preserve shared lower/upper envelope extrema and missing-value
transitions. Their curves are built once per boundary on that shared grid, then
clipped to each series' defined runs; adjacent Natural tangents remain identical.
Fill paths reverse the computed lower commands, preserving stepped interpolation.
Raw-data browsing is unchanged; tooltip values are distinct from stack bounds.

Seven new native tests cover signed/missing/mixed numeric and categorical data,
exact selection IDs, aggregate source spans and missing-bucket alignment, alignment
rejection, shared-envelope extrema and gaps, identical boundaries across all three
curves/four directions, gap-adjacent Natural tangents, 100k-point bounds,
cancellation and empty data. The first run exposed an invalid test fixture
(None in a numeric bar, which the existing contract rejects); the corrected
numeric fixture uses zero and separate categorical tests retain missing bars.

The full native suite passes **999 tests, two existing skips**. Full protocol
passes **420 tests**. Strict all-target Clippy and Rust formatting pass.
Full OCaml `@all @runtest @fmt` passes. Expectations come from the specified
wire layout; no automatic promotion was used.

Real hidden-window GPU readback passes **24 cases × scales 1, 1.25, 1.5 and 2**.
Eight new cases check both stack families in all four directions, including area
alpha and distinct lower/upper colors. This is GPU pixel evidence, not keyboard,
VoiceOver or physical presentation timing.

The root public gallery passes its real macOS chart walkthrough, including both
stack modes, Grouped/Stacked changes with retained selection, raw missing Wednesday
values, the 15-row data alternative, selected updates from 30 to 31, previous
families/category layouts/directions and zero registered resources after leaving
the page. Captured bar and area views were inspected: original values, cumulative
bounds, negative adjustments, missing observations and unchanged Target overlays
match the documented natural signed stack contract. The driver closes and reaps
its window. No VoiceOver or clipboard settings were changed.

A fresh independently installed public consumer also passes the same complete
macOS walkthrough, including the added stacks and scope cleanup. Its executable
SHA-256 is `f6a7ad3c1ddebdaced544a6b3d8dea1b7eadcb316dfe9dfb72b90ae54b45d6ca`. This installs into a task-local prefix without modifying
any opam switch. Both root and installed windows are closed and reaped.
The [sample companion](../../examples/charts/samples/stacked.md) and extended
[page walkthrough](../../examples/gallery/charts_page.md) provide partial OCH-48
coverage; the every-example inventory remains open.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-canvas-tests --test native_chart_paint
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --executable _build/default/examples/gallery/main.exe --section charts --images scratch/agents/root-20261004-resumed/chart-stacking-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section charts --workspace scratch/agents/root-20261004-resumed/chart-stacking-consumer
```

Ordinal colors, richer chart presentation options, whole-catalog acceptance,
VoiceOver, optimized performance and final-source hosted/release qualification
remain open. Full Linux desktop qualification is deferred to OCH-47. No tickets
are completed by these local results.

The [source/log/image archive](stacked-charts-och41/evidence.tar.gz) and
[verified manifest](stacked-charts-och41/manifest.json) retain the exact source
overlay, terminal command results, captures and installed executable identity.
