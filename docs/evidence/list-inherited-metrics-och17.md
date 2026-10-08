# List scrolling after inherited text changes — OCH-17

On macOS 14.5 arm64, the production host under GPUI TestPlatform reproduced a
scroll jump after an ancestor changed line height. A 24-pixel wheel event moved
a retained row by **44 pixels**. This is a deterministic layout/scroll regression,
not physical desktop presentation evidence or a diagnosis of the owner's earlier
benchmark overlap report. That benchmark uses constant text metrics.

## Cause and repair

GPUI measures visible rows during layout but retains cached heights for leading
and trailing overscan. The GPUIO list adapter invalidated rows on data changes;
it did not observe inherited text metrics. Changing an ancestor's line height
from 20 to 40 pixels updated visible rows while leaving a preceding cached row
at 20 pixels. The next wheel event crossed that old height. Remeasuring the newly
visible row then added an unintended 20-pixel displacement.

The list now uses the existing `measured_list_layout` observer also used by
pickers and palettes. Each retained list owns one metrics cache. The observer
invalidates measurements before list layout when inherited text metrics, rem
size or display scale change, preserving the logical row/pixel anchor. Text
color, background and decoration are excluded from the metrics comparison.
There is no new per-frame OCaml callback or protocol change. This does not turn
ordinary row updates into whole-list invalidation.

## Validation

At base `cdd25879` plus the new regression, the corrected initial fixture fails
with `left: 44px; right: 24px`. An earlier fixture attempt incorrectly queried a
row before the current anchor through `bounds_for_item`, which deliberately
returns `None` there; it was corrected before the actual behavioral failure was
captured. After the production repair, the unchanged wheel assertion passes.

The final expanded regression covers line heights 40, 12 and 28 pixels, checks
the unchanged row-50/5-pixel anchor, exact 24-pixel wheel displacement, invalidation
of a previously measured distant row and preservation of that measurement under
a paint-only color change. It uses production transactions, list layout and
wheel routing with TestPlatform; it does not open an OS window.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests --lib inherited_line_height_preserves_pixel_scrolling_across_warm_rows
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native \
  --features native-image-tests --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
```

The final native suite passes **1,186 tests with two existing skips**. This feature
selection is recorded explicitly; counts from differently selected prior suites
are not interchangeable. Strict Clippy and formatting also pass. Before/after and final-suite logs are retained in the
[evidence archive](list-inherited-metrics-och17/reports.tar.gz), with hashes and
source identity in its [manifest](list-inherited-metrics-och17/manifest.json).

Physical scrolling, display transitions, VoiceOver, full performance budgets and
final-source hosted acceptance remain separate requirements. The earlier
[loaded-list overlap investigation](list-scroll-diagnostics-och17.md) stays open;
this repair must not be presented as confirmation that the reported transient
has been fixed.

## Real-window follow-up — 2026-10-08

At `fd2a8112` plus the new `list_metrics_window_test.rs` fixture, the actual
macOS 14.5 arm64 window also passes all three line-height cases (40, 12 and 28).
Each change preserves logical row 50 and its five-pixel offset. After a 24-pixel
wheel event, freshly painted row bounds move exactly 24 pixels and match the
native list's reported rectangle. The fixture clears its paint probes before
each event and waits for platform frames; it cannot pass using old probe values.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-tests --test native_list --no-run
target/debug/deps/native_list-9a70830820ad6a82 --inherited-metrics
```

The exact local executable SHA-256 is
`3a16037e7e4b8d0deb96d425bfdcdb5fbd0405816d929110f7a3b52687c54cb9`.
The build passes; the test exits zero and closes its owned window normally under
a 90-second process-group timeout guard. Its portable invocation is
`cargo test --locked -p gpuio-native --features native-tests --test native_list
-- --inherited-metrics` through the isolated wrapper; Foundation now runs that
focused check alongside the existing full native-list and public-paging tests.

[Native build/run archive](list-inherited-metrics-och17/native-reports.tar.gz)
and [manifest](list-inherited-metrics-och17/native-manifest.json) retain the
commands, exact binary identity, source hashes and output marker.
This exercises the real platform renderer. The fixture dispatches input directly
at GPUI's `PlatformInput::ScrollWheel` boundary; it does **not** qualify OS event
synthesis, physical trackpad momentum, presentation latency, VoiceOver or the
original constant-metrics benchmark overlap report. No production code changed
in this follow-up.
