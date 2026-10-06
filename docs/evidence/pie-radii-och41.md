# Pie fixed and per-slice radii — OCH-41

2026-10-06, macOS 14.5 arm64, source base `52df9d1` plus this change.
The [contract](../design/pie-radii.md) adds global Fit/Pixels radius and bounded
stable-ID inner/outer radius overrides. Existing donut fractions, source weights,
angular intervals and original-value selection are preserved. Unknown overrides
wait for their IDs; equal radii omit a wedge/caption without removing source data.
This is a scoped chart increment; outside captions, leader-line styling, label
spacing and broader catalog/release requirements remain open.

Options schema 8 uses 114 default bytes and explicitly rejects older revisions.
Both languages check finite geometry, unique positive IDs and the 256-entry bound.
The native decoder bounds the list before reading entries; standalone options
admit at most 16 KiB. Chart configuration's decoder and retained-byte accounting
include the new bounded storage. Style/data/view versions remain -2/1/-1;
matching OCaml/Rust packages are required. Historical rejection fixtures remain
unchanged; current view fixtures carry the new nested options.

The independently written 44-byte custom payload is asserted by both encoders,
and the Rust decoder checks its exact values. Tests reject invalid numeric/ID
records, duplicate IDs, oversized advertised counts, unknown tags, truncation and
trailing bytes. Geometry/hit tests cover ID reordering/renaming, holes, outer
boundaries, hidden positive-weight sectors, ignored/returning IDs and zero source
weights. No source mutation or synchronous OCaml layout callback is introduced.

Real hidden-window GPU readback passes fixed radii, per-slice rings, reordering
and equal-radius omission at harness scale factors 1, 1.25, 1.5 and 2. These are
actual rendered pixels under test scale overrides, not physical monitor moves,
foreground keyboard, IME or VoiceOver acceptance. The existing all-family pixel
cases pass in the same harness.

The gallery has independent Bonsai switches for fixed and per-slice radii. Its
[adjacent walkthrough](../../examples/gallery/charts_page.md#pie-radius-controls)
explains the actual state/effects, GPUIO constructors, ID mapping and adaptations.
One owner-authorized GPT-6.1 Sol documentation agent drafted that scoped section;
primary review checked it against source and interface contracts.

The full OCaml `@all @runtest @fmt` check and root foreground gallery walkthrough
pass. Native AX switch actions and keyboard selection exercise Fit/80-pixel and
per-slice modes, both theme transitions, raw Reasoning=44/Other=10 data rows,
unchanged sample publication and return to default radii. Selection remains the
original Reasoning ID/value. Final image/chart/canvas registrations and registered
source bytes reach zero, and the child closes normally. The variable-radius
screenshot was visually reviewed alongside the automated GPU pixel assertions.

All 435 protocol tests and 1,028 native unit tests pass (two existing native
ignores remain). The maximum allowed pie radius prepares bounded meshes at worker
scales 0.5/1/2/8. Strict native Clippy with both native test features and Rust
formatting pass. A fresh installed consumer passes the same full pie walkthrough,
catalog check and zero-resource cleanup without modifying an opam switch. Its
executable SHA-256 is `6b81d2148ae4ee562761778a56af5d598e7aa6af21972bb1472d3d270b4f615d`.
All local build/test processes exited and their test windows closed; VoiceOver
settings were not changed. OCH-41 and OCH-17 remain open. No Linux GUI or broader
release acceptance is inferred. Hosted run 37502930557 covers older `328267a`: Linux foundation passed and macOS
was still running its table retention check at this checkpoint. Current-source
Linux/hosted checks remain required.


## Reproduction and retained evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_options --test chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --locked -j2 chart_geometry
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section chart-pie --images scratch/agents/root-20261004-resumed/pie-gallery-images
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-pie --workspace scratch/agents/root-20261004-resumed/pie-radii-installed
python3 -m py_compile scripts/gallery_pie.py scripts/test_gallery.py
python3 scripts/audit_example_docs.py
git diff --check
```

The [raw logs and screenshots](pie-radii-och41-logs.tar.gz) have a
[verified manifest](pie-radii-och41-manifest.json). The first protocol compile
caught two test assertions that reused an Options value after moving it;
the new owned vector removes Copy, so those reused expectations now clone.
The failed compile and formatting corrections remain in the archive. No observed
runtime failure or acceptance gate was hidden by that test-source adjustment.
