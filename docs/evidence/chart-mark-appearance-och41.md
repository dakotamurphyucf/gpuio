# Native chart mark appearance — OCH-41

2026-10-06, macOS 14.5 arm64, source base `0e7ca7e` plus this change.
The [appearance contract](../design/chart-mark-appearance.md) now attaches through
`Chart_style.create ~appearance`: independent series fill/stroke/curve/legend,
marker fill/border/radius/visibility, and stable-ID bar brushes/physical corners.
Bars support local backgrounds and signed base-to-tip, domain and explicit-value
ramps. Explicit area/radar fills carry their own alpha. Native workers resolve
IDs, prepare geometry and hit bounds; paint does not call OCaml.

Style schema is **-5**; matching OCaml/native packages are required. The independent
192-KiB appearance envelope has a paired maximum encoding of 173,447 bytes. The
parent envelope is now 384 KiB, with a maximally populated style/config test covering
all caption/axis/palette/appearance fields and the outer 1-MiB limit. Decoding and
retained-byte accounting include nested appearance. `Invalid_config` appends error
tag 4 without retagging existing errors; conflicting stacked-area curves report it.

## Local evidence

- Core chart expect checks and the full Rust protocol suite pass (445 tests;
  the later expanded parent maximum/error fixtures also pass separately).
- Native tests cover stable IDs, inherited/partial overrides, missing observations,
  uniform aggregate agreement versus fallback, signed stacked bounds in all four
  orientations, independent path alpha/stroke/curve and exact enlarged marker hits.
  Hidden markers preserve selection using default hit tolerance. A radar regression
  checks axis identity distinct from series identity and unchanged original selection.
- Actual GPU readback adds 64 cases: eight signed/direction ramps, four clipped
  value-ramp plateaus, physical corners, bordered datum markers, independent area
  alpha and a gradient path, each at scales 1, 1.25, 1.5 and 2. Existing chart pixel
  cases pass too. These are hidden-window synthetic-scale checks.
- The public `chart-marks` driver passes Line, Area, Bar, Categorical, Stacked bars,
  Stacked areas, Radar and Mixed layers. It checks native actions, keyboard selection,
  source-range provenance after explicit aggregation, original-data browsing, all
  four bar directions, both themes, source updates and zero-resource teardown.
  Initial driver assertions incorrectly expected a single value after selecting
  a sum; corrected assertions check the actual contract's 12-sample/2-category
  ranges. No production behavior or acceptance gate was weakened for those errors.

- The final native library suite passes **1,050 tests**, with two existing skips.
  The production mounted-view suite passes source replacement, asynchronous updates,
  labels, streaming across two windows and cleanup. Full Dune `@all @runtest @fmt`,
  workspace strict Clippy and feature-enabled strict Clippy pass. Existing capped
  vendor warnings remain; this is not a zero-warning claim.
- A fresh installed consumer builds against staged public libraries, checks both
  catalogs and passes the same complete native gallery walkthrough with resources
  and source bytes returning to zero. No switch/default was changed.
- Final screenshot review shows the complete plots, path alpha, signed/value ramps,
  physical corners and marker borders. Large radar markers can overlap captions;
  the guide explains using the existing label-gap control. No automatic marker
  collision avoidance is claimed.

Root executable SHA-256: `736b207a8a1f4914127441b74cbf84a4b2e341d6558abe0698a1da2306d4dde8`.
Fresh installed executable SHA-256: `a0ca67bc14988ab0331369c5ea8bb49769bbf1f59fb23653b20652cb0102cf1a`.
The [logs and selected screenshots](chart-mark-appearance-och41-logs.tar.gz) and
[verified manifest](chart-mark-appearance-och41-manifest.json) retain exact commands,
environment, binary hashes and outcomes. Both final drivers closed/reaped their apps.

The pure [preset guide](../../examples/gallery/chart_marks.md) explains constructors,
ID extraction, inheritance, limits and adaptation. The [page guide](../../examples/gallery/charts_page.md)
traces the actual Bonsai state/effect/style/native update. A GPT-6.1 Sol documentation
agent contributed the three guides; parent review checked them against final code.
The inventory covers 421 source files in 262 reviewed groups. Documentation review
is distinct from native acceptance.

Dense arbitrary backgrounds/patterns, per-datum bar baselines, scalar area baseline
and rich inspection remain explicit catalog work. This change does not complete
OCH-41 or OCH-17. Hosted/current-source Linux checks, physical presentation,
VoiceOver, OS IME and the wider release gates are not established by these tests.
No VoiceOver settings changed. Linux desktop qualification remains OCH-47.


```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section chart-marks --images scratch/agents/root-20261004-resumed/mark-api/gallery-images-qualified
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-marks --workspace scratch/agents/root-20261004-resumed/mark-api/installed-marks
python3 scripts/audit_example_docs.py
git diff --check
```
