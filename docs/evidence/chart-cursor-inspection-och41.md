# Cursor-following chart cards — OCH-41

2026-10-06, macOS 14.5 arm64, base `98d158e` plus this change.
`Chart_inspection.Placement.Cursor` follows native pointer motion without moving
crosshair/marker anchors or emitting OCaml hover observations. It updates within
the same mark, using the existing bounded gap/flipping/clipping rules. Keyboard
inspection and pointer departure/cancellation fall back to the mark anchor.
The [contract](../design/chart-cursor-inspection.md) preserves native ownership,
source identity and original-data access. Default Corner and Anchor behavior remains.

The placement appends wire tag 2; parent style advances to **-6**, with the existing
384-KiB envelope. Earlier schemas are rejected; matching packages are required.

## Evidence

- Public constructor/theme integration and independent OCaml/Rust placement tags
  0/1/2 pass. The full Rust protocol suite passes **446 tests**. The initial run
  caught an unchanged early style-version guard; it was corrected to -6 before
  this passing run. No rejection or size assertion was relaxed.
- Full native library tests pass **1,051**, with two existing skips. Card placement
  covers same-mark movement, Anchor fallback, invalid pointer fallback and bounds
  for all placement modes on small/large frames.
- Foreground native GPUI dispatch and actual GPU readback verify the card moves
  25 logical pixels within one pie wedge, without a selection event. Pointer
  departure clears position; Home returns to anchor-based keyboard inspection.
  The existing input/drag/commit/cancel/blur/reset/release and radar/AppKit suites
  pass afterward. These are not VoiceOver or OS IME checks.
- The first input run exposed a readiness-helper weakness: revision and palette
  could match an older prepared style. The helper now also requires the complete
  current configuration and matching frame. Production input correctly rejected
  that stale configuration; no input eligibility gate was bypassed.
- The public gallery passes all six inspection presets with actual pixels and
  original-value keyboard selection, source updates and original-data browsing.
  Its Cursor preset also moves the accessible card over the same pie wedge while
  leaving selection unset, then commits the original value by keyboard. Resource
  counts and source bytes return to zero after leaving the page.
- The mounted production chart suite and workspace/feature-enabled strict lint
  pass. Existing capped vendor warnings remain.

- Full Dune `@all @runtest @fmt` passes. A fresh staged-public-library consumer
  builds, passes its catalog checks, and passes the same complete inspection
  walkthrough, including actual same-wedge AX card movement and zero-resource
  cleanup. Both final gallery applications closed and were reaped.

Root binary SHA-256: `4632b118381dcccd245f962832e9964e0aba30338ae2afaa567b83bba4383d92`.
Installed binary SHA-256: `37d822e957e1b7510e95fdf2a1f516c85f5d41528a0edf6020910bfc6d36c565`.
[Archived logs and screenshots](chart-cursor-inspection-och41-logs.tar.gz) and the
[verified manifest](chart-cursor-inspection-och41-manifest.json) retain commands,
environment and binary identities. Archive creation verified that both binary
hashes still match the actual driver logs.

The [pure preset walkthrough](../../examples/charts/samples/inspection.md) and
[page guide](../../examples/gallery/charts_page.md) explain actual constructors,
Bonsai effects, native motion and ownership. GPT-6.1 Sol contributed the scoped
prose; parent reviewed it against source and validated 234 local links. Existing
inventory remains 421 source files / 262 reviewed groups / zero pending.

Rich custom rows, partial guide spans and annotations remain required catalog
work. OCH-41 and OCH-17 remain open. These tests establish no Linux desktop,
physical presentation/performance, VoiceOver, OS IME or whole-release acceptance.
No VoiceOver configuration changed. Current-source hosted checks remain required.


```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-inspection --images scratch/agents/root-20261004-resumed/cursor/gallery-images
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-inspection --workspace scratch/agents/root-20261004-resumed/cursor/installed
python3 scripts/audit_example_docs.py
git diff --check
```
