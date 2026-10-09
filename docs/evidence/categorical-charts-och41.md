# Typed categorical charts — OCH-41

Local implementation on `1d96d3d`, 2026-10-06, macOS 14.5 arm64 / M1 Max.
The [contract](../design/categorical-charts.md) now has public Core constructors,
paired bounded wire readers, native preparation, original-data access and an
OCaml gallery example. Numeric series keep their existing behavior.

## Behavior and compatibility

Category IDs are typed separately from datum and series IDs. The declared domain
controls order; IDs need not increase and equal labels do not merge identities.
Series align with that domain using explicit `None` values for missing data.
Line/area gaps remain disconnected, and absent bars remain in the original table.
No implicit sorting, zero filling or unknown-category fallback is introduced.

Native Auto/Point/Band options control uniform spacing and padding. Mixed layers
share category centers in all four value directions. Reduction uses projected
category positions, including padding. Sum/Mean bars retain their source interval,
exclude missing values from arithmetic and cover that interval's band extent.
Tooltips distinguish aggregate category ranges from exact values. Original-data
pages expose category labels/IDs and Missing without creating a dataset-sized UI.

The preparation adapter borrows numeric/categorical points and labels; it does
not clone a second numeric dataset. Source admission now charges category record
capacities, and the existing decode workspace allowance covers the separately
bounded domain and points plus shared text. Worker/plan quotas and cancellation
remain enforced. All labels remain native; no synchronous OCaml render callback
is added.

Data envelope version 1 appends contents tag 5. Existing data variants/bytes remain
valid. **The options record advances to version 2** for the layout field; matching
OCaml/Rust packages are required. Old options are explicitly rejected. The old
view fixture remains as rejection evidence, and the new view fixture is specified
independently. No compiler, Bonsai, GPUI or other dependency pin changes.

## Verification

- Independent categorical bytes include non-monotonic category IDs, equal labels,
  UTF-8 text and a missing bar. OCaml/Rust agree on the literal fixture and enforce
  membership, exact order, unique identities, finite values, limits and exact frame
  consumption. Every truncated fixture and trailing bytes are rejected.
- New options tests cover Auto/Point/Band, finite padding bounds, literal Point
  payload bytes and rejection of the older nested options record.
- Native tests cover four directions, actual hit queries, stable selection IDs,
  category ticks, missing rows, Sum/Mean arithmetic and full source intervals.
  Padded bucket membership is checked against an independent small example.
  A 100,000-category case checks bounded retained marks, source indices and
  horizontal/reversed-horizontal equivalence. Signed mixed layers exercise all
  three curves and verify that area paths stay on their own side of a gap.
- Actual hidden-window GPU readback passes 16 cases at scales 1, 1.25, 1.5 and 2,
  including categorical point spacing and the missing categorical bar. This is
  GPU pixel evidence, not foreground input or VoiceOver acceptance.
- The root gallery and a fresh installed public consumer pass the real macOS
  walkthrough: Auto/Point/Band changes preserve Home/Enter selection, duplicate
  Research labels retain category IDs 42/99, Code's missing bar is browsable,
  updates retain datum selection, and all previous families/directions still run.
  Repeated page transitions end with zero registered images/charts/canvases and
  source bytes; both drivers exit/reap their owned windows.
- The captured Band view was inspected: domain order and labels, the missing bar,
  mixed line centers and category-aware selection text match the contract.

The full native library suite passes 992 tests (two existing ignored); the full
protocol suite passes 419 tests. Full OCaml `@all @runtest @fmt`, Rust formatting,
Python syntax, catalog mapping and changed-document relative links pass. Final
suite logs are retained in the archive.
Strict all-target Clippy passes;
no warnings are suppressed in the new code. Existing upstream deprecation warnings
remain in the raw build logs. The initial protocol run's sole failure was the old
test intentionally using version 2 as an invalid options version; it now verifies
rejection of version 1. Expectations were updated from the explicit new wire layout,
not automatically promoted.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-canvas-tests --test native_chart_paint
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --executable _build/default/examples/gallery/main.exe --section charts --images scratch/agents/root-20261004-resumed/chart-categorical-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section charts --workspace scratch/agents/root-20261004-resumed/chart-categorical-consumer
```

Installed gallery SHA-256:
`31c482f65e764b04d11ab943b846036127e9eaf01f80b7f593db9ff82c6f8dbb`.
The [archive](categorical-charts-och41/evidence.tar.gz) and
[manifest](categorical-charts-och41/manifest.json) retain the exact source overlay,
logs, commands and owned-window captures. The [sample walkthrough](../../examples/charts/samples/categorical.md)
and updated [gallery walkthrough](../../examples/gallery/charts_page.md) explain
Core types, Bonsai state, native options and the scoped Eio publication path.

The explicit aligned-series API and standard band formula are intentional limits;
this is not a generic sparse accessor/scale API or a reproduction of the pinned
source's 30-pixel band-width cap. Stacking, ordinal colors, richer labels/tooltips,
other presentation options, VoiceOver, optimized performance and final-source
hosted/release acceptance remain open. Linux desktop acceptance is deferred to
OCH-47; compilation or these local tests do not establish it.
