# Native pattern brushes

OCH-41, 2026-10-06. Local macOS 14.5 arm64, Apple M1 Max, implementation after
`f211d38`. This is scoped feature evidence; the catalog and release tickets remain
open. The [design](../design/native-pattern-brushes.md) defines physical pixel
units, transparent gaps, validated bounds, native quantization and schema -8.

## Checks

- Paired OCaml/Rust byte expectations cover both ordinary Fill and chart Brush;
  live decoding checks truncation, trailing bytes, invalid dimensions and the
  previous chart style schema. Theme resolution rejects unknown tokens.
- The full protocol run passed 455 tests; the subsequent chart-style target passed
  six tests including the added explicit rejection of schema -7. The native unit
  suite passed 1,052 tests, with two existing tests ignored.
- Full Dune `@all @runtest @fmt`, strict native Clippy with all three test features,
  and Rust formatting passed. Vendored dependency warnings remain.
- The actual GPU `native_image_views` test changes the same retained ordinary
  View between solid red, checkerboard, slash and transparent backgrounds.
  A 64×64 sample contains 4,096 red pixels for solid, 2,048 for checkerboard,
  alternating adjacent checker cells, bounded nonzero slash coverage and no red
  for transparent. Existing sRGB/Oklab gradient and native image/control checks
  also pass. `native_images` separately checks resources; it is not the brush
  pixel test.
- The root public gallery passes both presets across line, area, bar, categorical,
  stacked bars/areas, radar and mixed layers. Bar and stacked-bar patterns retain
  original source selection in all four value directions. Area/bar light/dark
  changes pass. Original data browsing and explicit aggregation transitions pass;
  page exit returns registered counts and source bytes to zero, and normal close
  exits successfully. A line without an area has no filled region to pattern.
- A fresh installed consumer passes the same full native walkthrough and cleanup.
  Its counter/document-profile catalog handshake passes. The copied chart helper
  sources match the root byte for byte. Light area slash and dark bar checkerboard
  captures were reviewed for visible filled geometry and transparent gaps.

The first gallery failure was an incorrect expectation that changing a preset
would replace the page's committed aggregate observation with a single point.
The page owns that observation separately; the next Home/Enter commits under
current sampling. The corrected driver checks retention before committing.
The next failure waited for Radar preparation while the chart was entirely below
the viewport; the driver now reveals each family before waiting for Ready.
Both failure logs and screenshots are retained. No rendering timeout was hidden
or reclassified as a successful run.

Screenshots supplement the keyboard/AX assertions and ordinary-View pixel test.
They do not certify every density/scale, physical IME, VoiceOver, Linux desktop
behavior or physical presentation latency. Dense per-datum appearance and scalar
area/per-bar baselines remain separate catalog work.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_style --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --test native_image_views --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-tests,native-canvas-tests,native-image-tests --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
python3 scripts/test_gallery.py --section chart-marks --images scratch/pattern-root-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/pattern-consumer
python3 scripts/test_gallery.py --section chart-marks --executable scratch/pattern-consumer/consumer/_build/default/main.exe --images scratch/pattern-consumer-images
```

The consumer stages public packages into a fresh isolated prefix and builds the
example against them, without installing into an opam switch. It still uses the
repository's pinned toolchain/backend sources; this is not clean-machine or
signed distribution qualification. Hosted run 37549499328 targets `54173d2`, so
it cannot qualify these additions. Linux builds for this change remain pending.

The [raw logs and selected captures](native-pattern-brushes-och41-logs.tar.gz)
and [SHA-256 manifest](native-pattern-brushes-och41-manifest.json) retain
26 members (8,404,813 uncompressed bytes), including failed
preconditions and source/executable hashes. Root theme capture filenames initially
named the pressed toggle; installed-consumer captures use the resulting theme and
explicitly reveal the full chart. This screenshot-only adjustment changes no
behavior assertion.
