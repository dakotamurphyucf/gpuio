# Radar rating actions and public theme changes — OCH-41

2026-10-06, macOS 14.5 arm64, source base `997c9c0` plus these test additions.
The specialized rating path warranted a native check because it installs its
own semantic actions. Actual testing did not reproduce a production defect;
no runtime, API, wire, dependency or toolchain change was needed.

## Specialized native rating

The foreground chart fixture mounts an ordinary five-star rating control in a
retained label. Actual AppKit AXSlider increment is a positive control: it emits
one typed RatingRequested Increase, never a generic button Press. Another
increment is queued, confirmed not yet delivered, then the axis is removed and
returned in the same App update before repaint. The queued action is rejected.
A fresh native target works, while a retained retired AX object cannot activate
the replacement exposure.

GPUI pointer dispatch verifies a fresh first-star Toggle, preserves a pending
gesture across an ordinary same-axis publication, and rejects a pending gesture
across hide/return both before and after replacement paint. Fresh gestures recover
after each transition. The test resolves a new AX target again after those
deliberate retirements and verifies another successful increment.

The initial AX-only run already passed. Expanding it with pointer retirements
exposed a fixture mistake: its final positive assertion reused an AX object that
the new pointer cases had intentionally retired. AppKit correctly refused it.
The correction obtains the current target for that positive control; the stale
object rejection and all action-count assertions remain.

This is real AppKit semantic activation and GPUI-injected pointer input, not
physical-mouse or VoiceOver qualification. It covers the named rating behavior,
not a blanket acceptance claim for every specialized control or native action.

## Public OCaml gallery themes

The radar walkthrough now switches the application theme in both directions
while the custom-label native editor is focused. It verifies that the draft
remains `b`, focus stays on the same field, the label button remains enabled and
the Bonsai activation counter remains 2. The rest of the existing walkthrough
still covers typing/Backspace, source updates, button keyboard/AX effects,
ancestor disable, label hiding, original-data browsing, remount and teardown.
Registered image/chart/canvas counts and registered source bytes finish at zero.

The [adjacent example walkthrough](../../examples/gallery/charts_page.md#ordinary-views-as-radar-labels)
explains why a theme update restyles the retained Views without replacing the
native editor or resetting Bonsai state.

## Commands and evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-radar
python3 -m py_compile scripts/gallery_radar.py
git diff --check
```

The final native chart-input harness, strict Clippy, formatting, gallery build
and complete foreground public radar walkthrough pass. Existing clipping,
button/command, cross-chart/window, InputRegion and chart regressions remain
green. Final native chart worker reservations and workspace bytes are zero.
No new installed-consumer run is claimed for these test/driver changes.

[Raw logs](radar-label-rating-theme-och41-logs.tar.gz) and their
[verified manifest](radar-label-rating-theme-och41-manifest.json) preserve the
initial passing AX run, fixture failure and final successful validation. All
test/build processes exited and no VoiceOver settings changed.

OCH-41/OCH-17 remain open. These results extend the documented radar label
coverage; they do not establish all native widget combinations, VoiceOver, Linux
desktop behavior, physical presentation performance or broader catalog/release
acceptance. Hosted run 37502930557 covers production checkpoint `328267a` and
predates these additional tests. The required Linux nongraphical checks and
deferred OCH-47 desktop qualification remain separately tracked.
