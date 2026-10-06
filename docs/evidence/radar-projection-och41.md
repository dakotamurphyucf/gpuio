# Radar projection options — OCH-41

2026-10-06, source base `00f43c8` plus this change, macOS 14.5 arm64.
The [contract](../design/radar-presentation.md) now has public OCaml construction,
paired Rust decoding, bounded native preparation and public gallery evidence.
This is scoped scale/radius/spacing coverage; rich axis-label content, broader
chart catalog acceptance and milestone 07 remain unfinished.

`Chart_options.Radar` adds Per_axis/Data_max/Maximum and Fit/Pixels, plus label_gap.
Defaults preserve prior behavior. Data_max scans the immutable snapshot once per
preparation; zero-only data stays at the center. Explicit maxima below source
values extrapolate beyond the grid without changing source maxima, values or IDs.
Oversized drawings are clipped to the plot; nonfinite/out-of-budget projected
coordinates fail with Render_limit before tessellation instead of being clamped.
Native label gutters account for gap, with tiny-view proportional bounds.

Options schema 7 has 112 default bytes; the independent fixture is
`test/fixtures/chart-v7-radar-view.hex`. Both languages validate construction/wire
records. Old versions, unknown tags, truncation and trailing bytes are rejected.
Style/data schemas remain -2/1. Matching bridge package revisions are required.

Local checks pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @test/chart/runtest examples/gallery/main.exe -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib chart_
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native --lib --tests --no-deps -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j 2
python3 scripts/test_gallery.py --section chart-radar --images scratch/agents/root-20261004-resumed/radar-gallery-fixed-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-radar --workspace scratch/agents/root-20261004-resumed/radar-installed
python3 -m py_compile scripts/gallery_radar.py scripts/test_gallery.py
python3 scripts/audit_example_docs.py
git diff --check
```

There are 431 passing protocol tests and 97 passing native chart tests. Focused
regressions cover unequal axis maxima, shared maxima across multiple series,
all-zero/empty series, unclamped extrapolation, source provenance, fixed radius,
label anchors, hidden-label gutters and tiny frames. Mesh preparation at scale
factors 0.5/1/2/8 preserves an off-plot vertex's original selection identity and
rejects extreme projection before admitting a mesh. These worker tests are not
four-display GPU acceptance.

The first local walkthrough passed the projection controls and then failed a
new test assertion: it expected Enter in the read-only original-data browser to
commit selection. Source and the existing browser contract establish silent
browsing. The corrected driver checks row 10's raw `Nova / Reliability / 86 / 100`
while retaining Atlas selection, then closes the browser and commits the last
plot mark through native End/Enter. The full rerun passes. No framework behavior
or data-browser assertion was relaxed; the incorrect new expectation was replaced
by checks of both browsing and selection behavior.

The root walkthrough covers all scale modes, radius/gap changes, source values,
update preservation and retirement. The fresh independent consumer adds explicit
selection-retention assertions across configuration changes and Dark/Light/restored
theme interaction. Its executable SHA-256 is
`566ad5ed75dea2181d83e21dce9d88d0baf0fb4f53a5cae83a42cac6a0d315cf`.
Both end with zero image/chart/canvas registrations, zero registered source bytes
and normal child exit. Windows are closed and reaped. The root screenshot was
visually inspected: the explicit maximum expands the polygons beyond the grid,
and the native plot mask clips overflow as documented. No automated pixel metric,
OS IME, VoiceOver or Linux graphical acceptance is implied.

[Raw logs, failed/corrected runs and images](radar-projection-och41-logs.tar.gz)
with a [verified manifest](radar-projection-och41-manifest.json) retain 16 files
(4,047,324 uncompressed bytes). Hosted run 37470525492 is still checking older
`00f43c8`; current-source Linux and hosted checks remain due. The gallery walkthrough
is part of the full Charts section and also runnable as `--section chart-radar`.
