# Initial retained chart inspection rendering — OCH-41

2026-10-06, macOS 14.5 arm64, base `e8688b6` plus this change.
The [content contract](../design/chart-inspection-content.md) now has experimental
`View.chart ~inspection_content` attachment and native Card/Overlay rendering.
OCH-41 remains open: this is initial ordinary-button interaction qualification,
not acceptance of every arbitrary child widget or the full public gallery.
The view/options/style/data schemas remain **-2/9/-7/1**.

## Implementation

OCaml emits target-keyed wrappers after the independent radar-label wrappers.
Order, text and Card/Overlay changes preserve child identity. Normal child updates
use ordinary reconciliation; no synchronous callback enters OCaml from native
measurement, paint, hover or focus handling.

Native resolution compares the displayed selection against the immutable prepared
snapshot and current resource lease. Exact IDs can retain focused content while
new geometry for a reordered publication is pending. Removed targets retire
immediately; aggregate targets remain tied to their exact publication. Card uses
the configured backing/placement and Overlay uses plot bounds without that backing.
Both render ordinary retained Views with the existing control clipping and visibility
gates. The native source summary remains the container's accessible label.

Pointer entry or child focus holds the current target. Explicit chart keyboard
navigation supersedes pointer retention. Tab may enter an uncommitted preview
without committing a selection. Hidden children retain their nodes but lose input
eligibility through the existing revocable native lifetimes.

## Executed evidence

- OCaml reconciliation expect: reorder and Card/Overlay changes do not recreate
  controls; callbacks retain their targets. Text-only changes do not submit chart
  config; removal retires the callback.
- Actual foreground `native_chart_input` regression: native green button and
  magenta Card backing pixels, Overlay pixels without that backing, ordinary
  pointer clicks, pointer entry, Tab into committed and uncommitted previews,
  focused pointer-leave retention, stable-ID source reorder while focus stays in
  the child, immediate source retirement and rejection of a gesture spanning
  removal/return before repaint. Full existing chart/radar input checks also pass.
  Final metrics are `(117, 7, 2, 0, 0)`; resource cleanup counters are zero.
- The new regression initially failed because its fixture skipped native allocation
  slots. Reusing the retired slots with their next generation corrected the fixture.
  A subsequent test exposed a production defect: Home/End navigation could not
  replace a card held by the pointer. Clearing pointer retention for explicit
  chart commands fixes it; the same regression then passes.
- Native feature library tests: **1,053 passed**, two existing ignored. The new
  target-presence test checks all chart families using validated source fixtures,
  missing values and zero-weight unplotted targets.
- Strict feature-enabled Clippy and full Dune `@all @runtest @fmt` pass.
  The documentation inventory remains 421 sources / 262 reviewed groups /
  0 pending. No hosted result covers this change yet.

The foreground harness dispatches real GPUI native input and reads actual GPU
pixels. It is not a physical keyboard, IME, VoiceOver or arbitrary-child AX test.
No fresh-installed public example is claimed at this checkpoint. Native editor
state, aggregate interaction, queued AX/command/popup retirement, nested clipping,
multiple charts/windows, convenience rows and gallery integration remain required.

## Commands and logs

Use `GPUIO_JOBS=2` with the repository-local environment:

```sh
./scripts/gpuio exec dune build -j2 @test/chart/runtest
./scripts/gpuio exec cargo check -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
./scripts/gpuio exec dune build -j2 @all @runtest @fmt
```

The [log archive](chart-inspection-renderer-och41-logs.tar.gz) and
[manifest](chart-inspection-renderer-och41-manifest.json) preserve exact results,
including failing regression runs, and final source hashes.
