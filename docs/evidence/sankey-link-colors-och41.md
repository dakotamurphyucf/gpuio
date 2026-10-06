# Sankey ribbon colors — OCH-41

2026-10-06, macOS 14.5 arm64 / M1 Max. Implementation follows `c22a61c`;
intervening documentation checkpoints end at `0244af1`. Scoped local checks and
both root and independently installed gallery walkthroughs pass. See the
[contract](../design/sankey-link-colors.md) and
[OCaml sample guide](../../examples/charts/samples/sankey_presentation.md).

`Chart_options.Sankey.create ~link_color` accepts Source (the unchanged default),
Target or Gradient. Worker preparation resolves both node endpoints by ID, applies
ordinal/palette color and link opacity, and retains the selected brush with the
existing mesh. A gradient spans one GPUI path containing the complete ribbon;
there is no per-triangle restart, per-frame OCaml callback or extra tessellation.
Positive-flow geometry, zero-flow omission, source values and hit identity remain.
Prepared storage accounting includes the brush metadata.

Options schema advances 4 → 5, adding one bounded enum tag and making the default
fixture 101 bytes. Style/data remain -2/1. The paired view fixture is
`chart-v5-link-colors-view.hex`; old options-4 frames and unknown tags are rejected.
Matching OCaml/Rust packages are required.

## Validation

- Full protocol suite: **429 passed**. Independent OCaml/Rust bytes cover the
  default, all three policies, previous-version rejection and invalid enum tags.
- Full native suite: **1012 passed, two existing skips**. The new test checks
  reordered node IDs, ordinal mappings, existing alpha multiplied by opacity,
  identical geometry/vertex counts/retained charges across policies and absent
  zero flows. Its focused rerun also passes after a test-only lint correction.
- Full Dune `@all @runtest @fmt`, strict Clippy and rustfmt pass.
- Hidden-window actual GPU readback passes at **1, 1.25, 1.5 and 2** scales.
  Target ribbons have the target color; gradient left/right regions have opposite
  dominant channels, adjacent pixels stay continuous and clipping still holds.
  This is GPU raster evidence, not foreground keyboard or display-timestamp evidence.
- The root gallery and fresh installed consumer both pass their complete chart
  walkthrough: seven families/mixed layers, selection, updates, inspection,
  categorical/ordinal/stacked behavior, data browsing and cleanup. Each new
  gradient preset changes 144,296 sampled plot pixels versus default; target-color
  changes 178,670. Raw selection stays 100 (tiny flow 0.01), update becomes 101,
  original data retains five rows, and final resource/source-byte counts are zero.
  The installed build, catalog preflight and first GUI attempt all pass.

The root gradient screenshot was inspected: violet at the source transitions
continuously to teal at the target, with unchanged labels/node positions. The
root executable SHA-256 is
`d86af7bbd4e3c7420a962b2e49d97dd3f5a1f185910381abd88bc5e464c95c08`;
the independently installed executable is
`e8f4195ead72395b18a2a686b681a1d352f7e11a139e62697ce084600ae16170`.
Only the root run saved screenshots; both logs retain pixel assertions and hashes.

## Failures resolved during qualification

The first protocol run found a stale version-4 assertion in the stacking test;
the explicit expectation was updated to 5 in both languages. No expectation was
automatically promoted. The first Clippy run rejected a nested test conditional;
a let-chain preserves the same test cases and the final strict check passes.

The first gradient GPU check used adjacent-channel tolerance 3 and saw 113 → 109.
The pinned `vendor/gpui-apple/src/shaders.metal` deliberately dithers gradients by
up to ±2/255 RGB and ±3/255 alpha. With half-alpha over black, adjacent variation
can approach eight channel units before slope/rounding. The documented tolerance
is now 10, while the separate endpoint-contrast requirement stays greater than
50. All four scales pass. This corrects a test assumption; it is not a claimed
renderer defect or fix. Initial failed logs are retained with the successful runs.

[Archive](sankey-link-colors-och41/evidence.tar.gz) and
[per-file hashes](sankey-link-colors-och41/manifest.json) contain the source overlay,
raw logs, selected root screenshots, binary hashes and command record.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --lib link_brushes_follow_ids_and_opacity_without_changing_geometry_or_budget
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --test native_chart_paint --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section charts --images scratch/agents/root-20261004-resumed/chart-link-colors-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section charts --workspace scratch/agents/root-20261004-resumed/chart-link-colors-consumer
python3 scripts/audit_component_catalog.py
python3 scripts/audit_example_docs.py
git diff --check
```

Outer-column/above-middle label placement, other chart options, whole-catalog
acceptance, VoiceOver, optimized performance and final release gates remain open.
Linux GUI stays deferred under OCH-47. Hosted run 37447717604 covers older head
`548bcde`; it does not certify this implementation.
