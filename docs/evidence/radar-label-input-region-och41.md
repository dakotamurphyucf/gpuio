# Radar label InputRegion routing and pending clicks — OCH-41

2026-10-06, macOS 14.5 arm64, source base `cd9b3b3` plus this repair.
The foreground native chart harness now exercises an ordinary InputRegion inside
a retained radar label.

## Reproduced failures

With valid subscriptions and a visible, active native window, a down/up sequence
produced only `MouseUp`. The inner clipped-control wrapper consumed mouse-down
before the outer InputRegion received its bubble phase. InputRegion now receives
the event and applies its own configured native policy, like the other outer
gesture adapters. No protocol or OCaml API changes are needed.

After repairing routing, hiding and restoring the label's axis in one App update,
without a frame or input event in between, let an old mouse-up emit `Click`.
The region still held its pending down. Immediate visibility cleanup now clears
ineligible regions using their existing eligibility rules and state reset. The
same path handles chart data-browser visibility changes.

## Native regression

`chart_label_input_test.rs` runs from `native_chart_input`. It mounts a retained
InputRegion with text, waits for current geometry and confirms window activation
and label eligibility. Its subscriptions observe Click and use PreventAndStop for
bubble MouseDown/MouseUp. It verifies:

- A fresh native down/up produces exactly MouseDown, MouseUp and Click.
- A same-axis publication preserves the pending click.
- Axis removal/return before repaint retires the pending click even though the
  axis is eligible again when the old mouse-up arrives.
- Opening and closing the data browser before repaint also rejects that old
  click. A new down/up after repaint succeeds in both cases.
- The fixture removes its controls and restores the original chart data before
  the enclosing harness checks release and zero pending/retained renderer work.

Events are injected through GPUI's production dispatch in a foreground macOS
window. This is not physical mouse, radar-label AX, IME or VoiceOver evidence.
The separate existing InputRegion harness passes its native marked-text and
dispatch checks; those do not qualify IME inside radar labels.

## Validation

The focused and broader commands pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --test native_input_region --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --test chart_tree --test native_chart_view --test native_chart_input --test native_input_region --test native_menu_popup_queue --locked -j2
```

The broader run passes 1,024 unit tests (two existing skips), three chart-tree
tests and every selected native harness. Existing PointerArea, slider and AppKit
popup label regressions remain green. The ordinary InputRegion harness passes
nested capture/bubble ordering, window exit, clipping/modal/activation gates,
foreign capture, native editor marked-text retention and disposal.

Initial fixture rejection (invalid Click policy and subscription ordering) and
an ineffective first routing edit are retained separately from the two reproduced
product failures. No failed behavioral assertion was removed.

Strict Clippy with both native test features, rustfmt and whitespace checks pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

[Raw logs](radar-label-input-region-och41-logs.tar.gz) and their
[verified manifest](radar-label-input-region-och41-manifest.json) preserve the
failures and passing checks. All local harnesses exited normally after the fixes;
no VoiceOver settings changed.

OCH-41/OCH-17 remain open. Delayed non-menu accessible actions, further clipping/
resource children, theme/font/scale transitions and multiwindow label interaction
remain qualification work. Current-source hosted and independent consumer checks
are still required; CI run 37487278162 covers older `c373e3b`.
