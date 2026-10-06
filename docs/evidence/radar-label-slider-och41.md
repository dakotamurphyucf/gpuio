# Radar label slider input and retirement — OCH-41

2026-10-06, macOS 14.5 arm64, source base `7e3eaa8` plus this repair.
The ordinary View label adapter now has a native slider regression covering
track input and visibility changes between frames.

## Reproduced defects and repair

A visible slider track failed to start a drag: the inner clipped-control wrapper
consumed mouse-down before the slider's outer native Region received it. Like
PointerArea, Slider now reaches its own capture and propagation policy. Its
native handler prevents the enclosing chart from taking the same gesture.

After that routing repair, removing the label's axis left native pointer capture
active until another event or frame. Immediate visibility cleanup now calls the
same eligibility/cancellation policy already used by retained-tree slider
updates. This preserves the existing cancellation reasons and event path; no
new protocol or OCaml API is introduced.

## Behavioral regression

`chart_label_slider_test.rs` composes a real Slider with ordinary text inside a
radar label. It waits for current geometry, asserts visibility, and injects GPUI
mouse events into the native window. It checks:

- Clicking the middle of the track starts a Single-thumb drag and previews 50
  in a domain of 0–100.
- A same-axis publication preserves pointer capture and the complete slider
  snapshot, without extra slider events.
- Axis removal releases capture and ends dragging before yielding or drawing
  another frame, with exactly one `Cancelled Hidden` event.
- Returning the axis cannot turn an old mouse-up into a commit. Fresh down/up
  works normally, and the fixture removes its retained controls afterward.

The hidden-window dispatch regression passes. It does not establish physical
mouse, radar-label AX, IME or VoiceOver acceptance. Initial fixture mistakes
(detached text, slot allocation order and missing geometry readiness) are retained
separately from the two actual product failures.

## Validation

The focused command passes with `GPUIO_RADAR_SLIDER_OK`:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
```

The broader native suite passes **1,024 unit tests** (two existing skips), three
chart-tree tests, and all selected native harnesses: chart view, chart input,
AppKit popup queue and sliders. The ordinary slider harness also passes GPU
appearance at four scales, minimize/restore, multiwindow ownership, three
1,024-owner workload/disposal cycles and native accessibility actions. These
ordinary-slider checks are separate from radar-label accessibility qualification.
The harness stopped advancing frames until its own window was brought to the
foreground; it then completed normally. No assertion was relaxed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --test chart_tree --test native_chart_view --test native_chart_input --test native_slider --test native_menu_popup_queue --locked -j2
```

Strict Clippy with both native test features, rustfmt and whitespace checks pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

[Raw logs](radar-label-slider-och41-logs.tar.gz) and their
[verified manifest](radar-label-slider-och41-manifest.json) preserve the initial
fixture failures, reproduced routing/retirement failures and passing checks.

OCH-41/OCH-17 remain open.
Other native label wrappers, delayed non-menu actions, clipping/resource children,
theme/font/scale and multiwindow label interaction still need qualification.
Linux/hosted coverage for this repair remains pending; run 37487278162 tests the
older `c373e3b` revision.
