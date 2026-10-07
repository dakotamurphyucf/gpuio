# Area baseline qualification

OCH-41, 2026-10-06. Local macOS 14.5 arm64, Apple M1 Max, after `fd81d6e`.
The [design](../design/area-baselines.md) defines per-series data-unit baselines,
shared stacked offsets, source identity and style schema -9. This is scoped
feature evidence, not completion of the catalog or macOS release gates.

## Validation

- Full protocol suite: 457 passed. Independent OCaml/Rust bytes encode baseline
  42; constructors and live decoding reject nonfinite/out-of-bound values and
  the previous style schema. A maximum appearance with present baselines on all
  128 series occupies 174,599 bytes, below the unchanged 192 KiB budget.
- Native unit suite with `native-image-tests`: 1,055 passed, two existing tests
  ignored. New checks cover numeric/categorical domains and fill closure in all
  four value directions; baselines below, within and above source values;
  unchanged source identity; stacked positive/negative bounds and all three
  curves; mismatch rejection; missing-value gaps; empty fallback; and unaffected
  line/bar geometry.
- Actual hidden-window GPU readback: 32 new cases, baseline 0/1 × four directions
  × scales 1/1.25/1.5/2. At fixed source values, opposite filled/empty pixels prove
  the fill moves to the correct side of the baseline. Existing chart family,
  gradient, clipping and geometry cases pass in the same test.
- Full Dune `@all @runtest @fmt`, strict Clippy with all three native test features
  and Rust formatting pass. A subsequent scoped gallery build incorporates the
  baseline preset's final legend-preservation edit and passes formatting.
- Root and fresh installed-consumer gallery walkthroughs pass the baseline
  preset with original-value selection, all four area/stacked-area directions,
  light/dark area presentation, original-data browsing, zero registered resources
  and source bytes, and normal shutdown. Unaffected families retain their source
  selections. The installed counter/document-profile catalog handshake passes.
  Reviewed light-area and dark-stacked-area captures show the raised domain and
  native inspection value 30, baseline 20, endpoint 50. Screenshots supplement
  actual keyboard/AX and unit/GPU assertions.

The first native run exposed an empty-area regression (domain 0–0 instead of the
existing 0–1 fallback). Domain construction now preserves that fallback and the
existing zero contribution from bar layers, while accounting for area baselines
when a chart has defined values. Subsequent native tests pass. The initial
protocol maximal-size assertion and paired style version expectations were
updated explicitly for the additional option and -9 tag; no test output was
blindly promoted. The first full Dune run only failed those two already-corrected
style expectations; the complete rerun passes.

The gallery helper and its adjacent guide explain actual Bonsai preset state,
setter effects, public API calls and native preparation. The preset changes only
area baselines: other mark styling, sampling and legend colors stay inherited.
The source observation remains separate from presentation; original values and
stable IDs do not become shifted values. Native stacked inspection bounds carry
the offset while retaining the original contribution.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-tests,native-canvas-tests,native-image-tests --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe @fmt -j2
python3 scripts/test_gallery.py --section chart-marks --images scratch/baseline-root-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/baseline-consumer
python3 scripts/test_gallery.py --section chart-marks --executable scratch/baseline-consumer/consumer/_build/default/main.exe --images scratch/baseline-consumer-images
```

The installed consumer uses a fresh staged prefix and public OCaml packages,
with the repository's pinned composed backend/toolchain. This does not establish
clean-machine packaging or signing acceptance. Hidden-window pixels and gallery
AX/key actions do not certify physical IME, VoiceOver, physical presentation
latency or Linux desktop behavior. Dense per-datum backgrounds and per-bar
baselines remain explicit catalog work. Hosted run 37549499328 targets older
`54173d2`, not these changes.

The [raw logs and selected captures](area-baselines-och41-logs.tar.gz) and
[SHA-256 manifest](area-baselines-och41-manifest.json) contain 24 members
(7,544,950 uncompressed bytes), including initial failures, exact
commands/exits and source/executable hashes. The copied consumer chart helper
sources match the root byte for byte.
