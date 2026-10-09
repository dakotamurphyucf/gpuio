# Radar label popup and browser retirement — OCH-41

2026-10-06, macOS 14.5 arm64, source base `5f8890a` plus this change.
The previous [capture repair](radar-label-capture-och41.md) synchronized pointer
owners on source visibility changes. The new AppKit regression found that open
menus and the data-browser entry paths still required lifecycle synchronization.

An ineligible retained menu now closes its native popup lease or drawn menu path
without restoring focus into its unavailable owner. The existing input-visibility
synchronizer performs this check alongside focus, pointer capture, drag and tooltip
retirement. No new serialized messages, OCaml callbacks or chart schema are needed.

Chart keyboard and original-data button routes now run that synchronizer after
releasing the chart-state borrow. Focus callbacks can therefore run safely. The
keyboard route only does this when the visibility identity actually changes;
ordinary mark navigation does not scan menu owners on every key.

## Reproduced failures and corrected fixture errors

The first fixture attempted to acquire chart data before its initial publication
and received `NotReady`. It now treats that initial state as base revision zero.
With a valid fixture, hiding a live axis left its popup lease active, reproducing
`hidden radar label must retire its popup lease before paint`.

After the source-driven repair passed, the expanded fixture initially attempted
to retrieve a chart control handle through the manager's scope-handle API; that
lookup correctly returned None. The fixture now gives the chart focus through
native mouse dispatch. Its actual D-key route then reproduced the same popup
retirement failure when opening original data. The keyboard/button retirement
repair fixes that path without removing the failing assertion.

Failure cleanup explicitly closes a stranded popup after recording the failing
state, so a failed assertion cannot leave AppKit's nested tracking loop running.
The recorded state still fails the regression; cleanup is not acceptance.

## Native evidence

`menu_popup_chart_test.rs` extends the real `native_menu_popup_queue` harness,
wrapping its retained AppKit menu in a radar-axis View. The source store,
notifications, chart preparation, menu commands and native tracking adapter are
production code.

- Queue Show, remove the axis and return it in one App update. The old popup lease
  retires before yielding/painting and its queued runner never enters AppKit.
- Open original data through native chart D-key dispatch while a popup is queued.
  The hidden label's lease retires before paint. Escape restores label eligibility.
- Start a native PointerArea gesture inside that label, then press D. Native
  capture releases with exactly `Started` / `Cancelled Hidden` before repaint.
- Let a current popup actually enter AppKit tracking, then remove its live source
  axis. Its owner lease retires in the source notification turn; AppKit cancellation
  completes through the existing run-loop adapter. The native entry counter proves
  this exercises live tracking rather than only skipping a queued Show.
- Restore the original menu, release the chart registration and preserve the
  existing queued-close, observer/config replacement, hidden/disabled,
  owner-remount, positive-recovery and window-close checks.

These are real AppKit tracking and GPUI input-dispatch checks in a visible window.
Injected mouse/key events are not physical-device or VoiceOver acceptance.
The new fixture drives the keyboard browser route; the public gallery separately
checks the accessible View data/Back to chart button route.

## Validation

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_menu_popup_queue --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --test chart_tree --test native_chart_view --test native_chart_input --test native_menu_popup_queue --locked -j2
```

The full run passes **1,024 unit tests**, two existing skips, and all three
chart-tree tests. The native chart input, GPU/lifecycle and popup executables all
pass. This includes the existing marker-pixel, stale data-route, quota-recovery
and two-window streaming checks; those checks retain their original scope.
Strict Clippy (including dependencies and both native test features), Rust format
and whitespace checks pass. The final strengthened popup run also asserts that
axis eligibility has already returned before the old queued runner is rejected,
and that an old mouse-up emits no pointer event after browser close.

The rebuilt public gallery passes the full foreground radar walkthrough: editor
typing/Backspace, publication focus/draft retention, accessible/Space activation,
disabled input, View data/Back to chart, empty native remount with retained Bonsai
counter, zero final source resources and normal child exit. This checks the repaired
button route as well as the prior keyboard route.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-radar
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

[Raw logs, including original failures](radar-label-popup-och41-logs.tar.gz) and a
[verified manifest](radar-label-popup-och41-manifest.json) retain the evidence.

Current-source Linux/hosted qualification remains required. Run 37487278162
covers older `c373e3b` and must not be counted as checking this repair. Delayed
non-menu accessibility actions across hide/reset/return, clipping/resource children,
theme/scale and multi-chart/window input isolation remain outstanding label work.
The broader catalog and OCH-17 release gates remain open.
