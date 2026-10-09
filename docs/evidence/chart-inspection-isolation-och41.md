# Retained inspection isolation across charts and windows

OCH-41, 2026-10-06. Local macOS 14.5 arm64 / Apple M1 Max qualification after
`4fba4cc`. This adds a native fixture to `native_chart_inspection`; production
rendering, protocol and OCaml APIs are unchanged. It does not complete the catalog
or qualify every widget/IME/VoiceOver combination.

Two native windows each contain a left Card chart and a right Overlay chart. Their
local tree IDs are identical across windows. The two left charts share one source;
the right charts share another. Each child has an ordinary button with a distinct
semantic label within its window.

Actual AppKit button activation verifies exact `(window, node)` callback routing.
Hiding one chart's card denies only its children; showing it while hiding its peer
does not clear the peer's hidden state. Old accessibility objects remain retired,
while other charts and the other window still accept their current objects.

Removing the left target from its shared source hides it in both windows. An
ordinary publication of the right source preserves its controls. Returning the
left target and making a fresh preview restores new actionable semantic objects.
Unmounting a chart and closing its window do not disturb the surviving window.
A real GPUI pointer gesture on the surviving right button completes while the
other source is released; native Space dispatch also activates it afterward.
Releasing its own source denies the button and retires its accessibility object.

Both temporary windows close and both temporary sources release, with chart
source accounting restored to the pre-fixture baseline. The original test window
is reactivated before continuing its normal cleanup. The complete inspection
suite passes, including prior button/clipping, Input/Textarea AppKit composition,
aggregate publication and command cases; final metrics are `(66,12,2,0,0)`.

The first run found a fixture precondition: native chart keyboard input correctly
requires an active window. Its synthetic keys sent to inactive windows did not
select targets. The corrected fixture activates each window before previewing,
then tests independence after switching away. No production input-policy change
was needed.

Commands, all successful on the corrected source:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_inspection --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-tests,native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

The [raw logs and source hashes](chart-inspection-isolation-och41-logs.tar.gz) and
[verified manifest](chart-inspection-isolation-och41-manifest.json) retain the first
fixture failure and successful corrected run. All local commands finished and
windows closed. No OCaml/example source changed; preceding Dune and hosted results
are historical. CI37549499328 targets older `54173d2` and does not cover these local
additions. Rich-row helpers, public-gallery integration, current-source hosted
checks and broader OCH-41/OCH-17 release requirements remain open.
