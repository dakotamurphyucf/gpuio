# Radar label clipping and native focus — OCH-41

2026-10-06, macOS 14.5 arm64, source base `48cb2ca` plus this repair.
The foreground chart regression reproduced two defects when a native window
narrowed without an OCaml tree or chart-source update: a retired AppKit button
object could activate a newly visible label, and a fully clipped button retained
native keyboard focus. The containing InputRegion remained partly visible, so
whole-slot visibility alone could not establish child eligibility.

## Repair and ownership

The host now schedules complete-paint cleanup when the measured clipping mask
changes. Shared visibility cleanup retires ineligible input/action lifetimes,
and the existing focus finisher releases an ineligible focused child. This runs
after all owner-specific finishers, allowing a carousel to restore its own
viewport focus before the shared root fallback is considered. The first broader
test run caught three carousel regressions from doing this too early; those
failures and the corrected run are preserved.

This connects native resize/scroll clipping to the existing lifetime contract.
Source-driven hiding still performs immediate cleanup before another paint.
No public API, wire, compiler or vendor change is involved. The hook runs on
visibility changes; it does not introduce permanent idle rendering.

## Native evidence

The foreground `native_chart_input` fixture now checks:

- A 240 px window narrowed to 80 px, with the composite partly visible but its
  absolutely positioned child completely clipped. The child loses native focus.
- Expansion back to 240 px without a tree/source update. A retained old AppKit
  AXButton cannot activate the replacement exposure; a fresh target succeeds.
- Zero-height content and combined natural bounds exceeding the chart geometry
  limit. Individually valid padding fields produce the latter case. Both gate
  the child, retire its AX action and release focus; normal dimensions recover.
- A bounded idle observation requiring five consecutive unchanged render-count
  samples, 20 ms apart, after recovery. Cleanup settles without a redraw loop.

AX actions use actual AppKit objects, and resizing/focus use a real native window.
Related pointer regressions inject GPUI events. These are not physical-mouse,
radar IME or VoiceOver acceptance. No VoiceOver settings changed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --test chart_tree --test native_chart_view --test native_chart_input --test native_input_region --test native_menu_popup_queue --test native_table_host --test native_carousel --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-radar
git diff --check
```

The final selected suite passes 1,024 unit tests (two existing skips), three
chart-tree tests and all selected native harnesses. This includes the carousel
focus regressions, existing chart label gestures/actions/popups, native InputRegion,
AppKit table keys/marked-text/AX behavior and sparse 100,000-row table checks.
Strict Clippy, formatting and the rebuilt root gallery radar walkthrough pass.
The gallery finishes with zero registered source bytes. The earlier fresh local
consumer evidence belongs to `48cb2ca`; it is not a fresh install of this repair.

[Raw logs](radar-label-clipping-och41-logs.tar.gz) and their
[verified manifest](radar-label-clipping-och41-manifest.json) retain the initial
AX and focus failures, the owner-order regression and final passing checks.
All local test/build processes exited. No acceptance assertion was removed or
threshold relaxed.

OCH-41/OCH-17 remain open. Resource/icon children, cardinal/diagonal placement,
font/theme/scale changes, two-chart/two-window label isolation and specialized
widget actions still require qualification, alongside broader catalog and release
requirements. Hosted run 37487278162 covers older `c373e3b`; current-source CI is
still required.
