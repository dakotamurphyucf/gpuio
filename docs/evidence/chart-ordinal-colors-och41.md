# Stable ordinal chart colors — OCH-41

Implementation based on `746b29b`, 2026-10-06, macOS 14.5 arm64 / M1 Max.
The [contract](../design/chart-ordinal-colors.md) adds explicit identity-based
colors independently of the current order of chart data. Local qualification below passes; broader catalog/release acceptance remains open.

## Implementation

Public `Chart_style.Key` distinguishes series, pie slices, Sankey nodes and candle
movement. `Ordinal.create` validates an explicit ordered domain of up to 1024
unique keys, a cyclic range of 1–32 colors and an optional unknown color. The
style constructor resolves theme tokens. Without an unknown override, unmapped
entries retain ordinary position-palette behavior. Source labels and selection
identities remain unchanged.

Native workers resolve one bounded vector shared by marks and legend swatches.
Sankey ribbons inherit their source-node color. Paint performs no ordinal lookup
and never calls OCaml. Configuration vectors count toward tree retention and
worker/result admission; prepared colors count toward the render-plan quota.
A regression verifies a maximum-domain configuration remains charged after its
owner and pool close, until the retained Ready reader is dropped.

Style envelope version 0 replaces the formerly unversioned record. A legacy
nonempty palette length cannot be mistaken for this zero prefix. Paired literal
fixtures cover all five key tags and the combined view; old view/style bytes are
rejected. Options version 3 and data version 1 remain unchanged. The packages on
both sides of the bridge must match; no compiler or dependency pin changed.

## Completed local checks

- Full protocol: **422 tests pass**. Truncation, bounds, invalid identities,
  duplicate domains, unknown tags, empty-domain semantics and legacy rejection
  are covered independently of the OCaml codec.
- Full native library: **1005 pass, two existing skips**. New tests cover every
  family, reordering, namespaces, cycling/unknown fallback, actual prepared paint
  colors, selection titles, cancellation and retained memory accounting.
- Full OCaml `@all @runtest @fmt` and strict all-target Rust Clippy pass. Expect outputs
  were reviewed and corrected manually; no automatic promotion was used.
- Hidden-window GPU readback: **28 cases at scales 1, 1.25, 1.5 and 2 pass**,
  including reordered pie colors and unknown/fallback policies. This proves
  rendered pixels, not physical presentation latency or keyboard behavior.
- The root public gallery's real macOS chart walkthrough passes. Owned screenshot
  pixels verify legend colors before/after reordering, unknown fallback and
  restoration. Selected Research follows its stable ID and updates from 40 to 41;
  the original-data table remains available. Existing family/category/stack and
  direction checks pass, and page retirement leaves zero registered resources.
  The reordered screenshot was visually reviewed against the source and domain.

Initial checks found test-fixture mistakes (a candle field name and expecting a
rectangle for an isolated zero-flow Sankey node) and an OCaml optional-argument
order mismatch. These were corrected before the passing checks above. Isolated
nodes remain present in the legend even when they have no visible flow rectangle.

A fresh independently installed public gallery also passes the complete chart
walkthrough, including ordinal swatch pixels, stable selection, fallback changes
and zero-resource teardown. Its executable SHA-256 is
`4b0d17ca103bdd722aa1a12669ce300a8bf856d6a99f5455062b2039064b8d84`.
The task-local installation does not modify any opam switch. Both drivers close
and reap their windows; no VoiceOver or clipboard settings were changed. The [sample companion](../../examples/charts/samples/ordinal_colors.md)
and [page walkthrough](../../examples/gallery/charts_page.md) provide partial
OCH-48 coverage; the every-example inventory remains open.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-canvas-tests --test native_chart_paint
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --executable _build/default/examples/gallery/main.exe --section charts --images scratch/agents/root-20261004-resumed/chart-ordinal-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section charts --workspace scratch/agents/root-20261004-resumed/chart-ordinal-consumer
```

Richer chart labels/tooltips and per-datum presentation options, consolidated
catalog acceptance, accessibility, optimized performance, final-source hosted
checks and distribution remain open. Linux desktop qualification is deferred to
OCH-47; these local results do not establish Linux GUI or whole-release acceptance.

The [source/log/image archive](chart-ordinal-colors-och41/evidence.tar.gz) and
[verified manifest](chart-ordinal-colors-och41/manifest.json) retain the exact
source overlay, command results, owned captures and installed executable identity.
