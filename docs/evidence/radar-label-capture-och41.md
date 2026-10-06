# Radar label pointer routing and idle retirement — OCH-41

2026-10-06, macOS 14.5 arm64, source base `c373e3b` plus this repair.
The [ordinary View adapter](radar-label-content-och41.md) supports native child
controls; the new regression found two bugs in its PointerArea lifecycle.

First, the clipping/interaction wrapper stopped mouse-down propagation on the
inner View before the PointerArea's outer native Region could receive it. The
regression failed at `label pointer region starts capture`. PointerArea now keeps
its own capture and propagation policy; generic button/editor handling remains
on its existing path. The chart's own drag handler already refuses to capture
when a child owns native pointer capture.

With routing repaired, the same test failed at `axis removal must cancel capture
before paint`. A source notification hid the label and updated focus, but did not
run the existing capture/drag/tooltip synchronizer. Capture survived until another
input event or frame. Chart visibility transitions now run that synchronizer after
releasing retained-tree/state borrows. Ordinary updates that leave eligibility
unchanged do not invoke it or cancel the label's gesture.

## Native regression

`chart_label_capture_test.rs` mounts an actual PointerArea as radar content and
injects mouse events through GPUI's production dispatch in the hidden GPU window.
It checks real capture ownership and asynchronous PointerEvent phases, not just
the visibility predicate. Source completion and its notification run synchronously;
cancellation is inspected before yielding or drawing a replacement frame.

The harness passes:

- A child down starts native capture; ordinary same-axis value updates preserve it.
- Axis removal, generation reset and a change to Pie each release capture and emit
  exactly one `Cancelled Hidden` after `Started`.
- Returning the radar axis does not let the old mouse-up emit `Released`; a fresh
  down/up produces a normal `Started` / `Released` pair.
- Hiding label options, replacing the source and releasing the replacement each
  retire a held child gesture before another paint.
- Cleanup removes the child and releases registrations. The enclosing production
  chart and two-window streaming harness also pass with zero pending/retained work.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
```

The initial routing failure and subsequent retirement failure are retained; neither
assertion was removed. The final passing marker is `GPUIO_RADAR_CAPTURE_RETIRE_OK`.
This is real native dispatch/capture in a hidden window. It is not physical-mouse,
foreground keyboard, popup, IME or VoiceOver qualification.

The broader native suite with `native-canvas-tests,native-image-tests` passes
**1,024 tests**, with two existing skips; all three chart-tree tests pass. Strict
Clippy includes dependencies and both test features. The rebuilt public gallery
also passes its complete foreground radar walkthrough:
actual editor typing/Backspace, focus across source publication, accessible/Space
button activation, disabling, hide/browser draft retention, empty remount and
Bonsai counter preservation. Resource retirement converges to zero within the
existing bounded check and the child exits normally. This walkthrough exercises
the new native code but does not add a physical-pointer capture claim.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --test chart_tree --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-radar
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

[Raw failing/passing logs](radar-label-capture-och41-logs.tar.gz) and their
[verified manifest](radar-label-capture-och41-manifest.json) preserve the checks.
Current-source Linux/hosted coverage remains due; run 37487278162
covers older `c373e3b`. [Subsequent AppKit/browser evidence](radar-label-popup-och41.md) covers
browser-triggered capture cleanup and queued/live popup retirement. Delayed
non-menu accessible actions, clipping/resource children and multiwindow input
isolation remain separate label qualification. OCH-41/OCH-17 remain open.
