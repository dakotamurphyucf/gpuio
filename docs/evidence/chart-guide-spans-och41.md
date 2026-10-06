# Independent chart inspection spans — OCH-41

2026-10-06, macOS 14.5 arm64, source base `17cd961` plus this change.
[Validated span values](../design/chart-guide-spans.md) provide independent vertical
and horizontal guide intervals, in logical pixels or fractions of the physical
plot extent. Defaults preserve full guides. Layout clips overhang and draws no
empty intersection, without changing data, selection, card placement or marker
anchors. Fractional spans follow native resize.

Span wire tags are Full=0, Pixels=1 and Fraction=2. Two fields follow crosshair
color, vertical then horizontal. Parent style is **-7**; older schemas are rejected
and matching packages are required. The maximal admitted style still fits the
384-KiB style and existing parent transaction envelopes.

## Local evidence

- Focused OCaml expect tests pass validated bounds, nonfinite rejection, public
  style resolution and independent fixed span bytes shared with Rust fixtures.
- All **448 protocol tests** pass, including independent span fields, malformed
  values on either axis, unknown/truncated tags, old-version rejection and the
  maximal style/parent message.
- Native library tests pass **1,052**, with two existing ignored tests. The span
  resolver checks clipping, zero/outside intervals and resize-relative values.
- The foreground native chart test passes **72 actual GPU readback cases**:
  vertical/horizontal/both, solid/dashed, two sizes and six span settings. Both-axis
  cases intentionally use different units/intervals. Independently calculated
  expected rectangles bound every guide-colored pixel; solid interiors must be
  completely painted. Empty guides emit none. Original source revision and selected
  slice ID remain unchanged. The existing cursor, pointer, keyboard, lifetime,
  radar and AppKit checks pass afterward. This is not OS IME or VoiceOver evidence.
- The mounted chart suite and strict workspace/feature-enabled Clippy pass locally.
  The separate hosted pie-leader failure on an older revision remains open; local
  success does not erase it. See [terminal hosted evidence](hosted-presentation-calibration-och17.md#hosted-run-37523471664).
- The public root gallery passes seven presets, including actual Partial guides
  pixels, original-value keyboard selection, source update, original-data browsing,
  cursor-card movement and resource cleanup. Chart/source bytes and registration
  counts return to zero on page exit. Visual review confirms the middle-half
  vertical and 120-pixel horizontal guide in the actual screenshot.

Full Dune `@all @runtest @fmt` passes. The fresh installed-library consumer builds
and passes the same seven-preset native walkthrough with zero-resource cleanup.
Both applications close and are reaped. The example inventory stays at 421 sources
in 262 reviewed groups; scoped GPT-6.1 Sol documentation updates were reviewed
against the final API and sample code. Structural audit and Markdown links pass.

Root gallery SHA-256: `f874e4f1ee92d54c33f84756fa282d9e5fe3fff0e1ac972a88046c2bf212fd6e`.
Installed gallery SHA-256: `beecaa709224cad474a3e9cc8e37188950542d64996f9513b727f67a5f8f99ac`.
[Logs, commands and screenshots](chart-guide-spans-och41-logs.tar.gz) and the
[verified manifest](chart-guide-spans-och41-manifest.json) retain 16 files
(4,616,887 uncompressed bytes). Archive creation verified both current binary
hashes against the actual driver logs. Current-source hosted checks remain required.

Broader rich inspection rows/annotations and catalog/release work remain.
OCH-41 and OCH-17 stay open; no Linux GUI, physical presentation/performance,
VoiceOver, OS IME or whole-release acceptance is inferred from these checks.

## Commands

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-inspection --images scratch/agents/root-20261004-resumed/guide-spans/gallery-images
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-inspection --workspace scratch/agents/root-20261004-resumed/guide-spans/installed
python3 scripts/audit_example_docs.py
git diff --check
```
