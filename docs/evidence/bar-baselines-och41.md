# Source-owned bar intervals

OCH-41, 2026-10-06/07, local macOS 14.5 arm64 on Apple M1 Max, after
`8e806f4`. The [baseline contract](../design/bar-baselines.md) now has local
codec, native unit/GPU, public-gallery and fresh installed-consumer evidence.
This qualifies the scoped feature, not the whole catalog or release.

`Chart_data.Bar_baseline.create` and `with_bar_baselines` attach immutable
data-unit origins to stable series/datum pairs. Original observations remain
endpoints. Baselines coexist with dense brushes, survive reorder, and clear
independently. Exact bars preserve original endpoints; Sum/Mean require a common
origin within a bucket, and stacking requires it among participating series at
each position. Incompatible bases produce `Chart.Error.Invalid_config`.

## Validation

- OCaml expect tests and Rust protocol tests compare independently composed
  schema-3 bytes, reject old schema 2, truncation/trailing bytes, invalid scalars,
  references, duplicates and ordering. Count bounds apply before allocation.
  A 100,000-record case exercises both sidecars without increasing the combined
  16 MiB envelope. Clearing either sidecar preserves the other and original data.
- Native reduction tests cover numeric/categorical inputs, missing observations,
  exact/Sum/Mean, signed stacks, position-specific bases and incompatibility.
  A single endpoint of 1e-100 against a baseline of 1e100 survives unchanged.
  Domain, inspection and original-data checks retain source endpoints and origins.
  Replacement tests charge both sidecars and release held snapshot storage.
- The full protocol suite passes **466 tests**. The final full native unit run
  passes **1,062 tests / 2 ignored**; a later focused run passes all **5 baseline
  tests**, including the added signed/missing stack case. These are distinct runs.
- The actual GPU chart-paint suite passes, including **32 new interval cases**:
  numeric/categorical inputs × four directions × four synthetic scales
  (1, 1.25, 1.5, 2). Independent filled/empty pixel controls distinguish origins
  and endpoints. Synthetic scales do not qualify physical monitor transitions.
- Strict native Clippy with all test features/targets and full Dune
  `@all @runtest @fmt` pass. The example documentation audit reports
  **427 sources / 265 reviewed groups / 0 pending**.

The gallery's Bar backgrounds branch adds Zero, Shared 40 and Individual
20/40/60 choices. Root and fresh installed-consumer native walkthroughs pass:
original Batch 01 endpoint 33 with base 40 in all four directions; original-data
table exposure; a four-category Mean endpoint 49.5 with base 40; explicit failure
for incompatible Individual+Mean and recovery to Shared; stable IDs/bases after
reorder; zero reset; branch teardown/reacquisition; and final zero registered
resources/source bytes with normal application shutdown. Captures were reviewed.
Long original-data cell text can ellipsize visually; its complete AX value is
checked, and the native inspection card displays origin and endpoint separately.

The independently installed consumer uses a fresh package prefix and copied
example, with matching source hashes and successful extension catalog handshakes.
It still uses the repository's toolchain/composed Rust backend; this is not
clean-machine installation or signed distribution acceptance. A GPT-6.1 Sol
documentation agent updated the adjacent fixture/Bonsai walkthroughs; the parent
reviewed the API calls, state/effect flow and lifetime explanations against code.

Initial native failures were stale version-2 fixtures and an invalid-version
control that became valid version 3. The OCaml fixture expectation likewise
needed the independently reviewed version/tag bytes. The first GPU compile
attempt used f64 pixel coordinates where the test helper requires f32; only
test coordinates changed. Two gallery attempts timed out because `wait_text`
does not traverse AX tables; adding Home alone did not fix that. Explicit
`wait_find(..., search_files=True)` now checks the actual original-data value.
Failed logs/captures are retained alongside passing evidence.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --lib chart_reduce::baseline_tests --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-tests,native-canvas-tests,native-image-tests --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j2
python3 scripts/test_gallery.py --section chart-backgrounds --images scratch/baseline-root
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/baseline-installed
python3 scripts/test_gallery.py --section chart-backgrounds --executable scratch/baseline-installed/consumer/_build/default/main.exe --images scratch/baseline-consumer
python3 scripts/audit_example_docs.py
```

Current view/options/style/data schemas are **-2/9/-9/3**, requiring matching
packages. Hosted run 37558498842 targets older `8a98315`, not this implementation.
Current-source Linux build/unit/consumer checks remain required; Linux desktop
qualification stays deferred under OCH-47. Physical IME/VoiceOver, presentation
latency and distribution/API release gates remain open. Arbitrary pixel-bound
fill callbacks remain a separate catalog difference.

The [raw logs/captures](bar-baselines-och41-logs.tar.gz) and
[SHA-256 manifest](bar-baselines-och41-manifest.json) retain exact commands/exits,
source/executable hashes and installed-source comparisons. Every archive member
was hash-verified.
