# macOS cancelled-scroll conversion — OCH-41

2026-10-08, physical macOS 14.5 arm64, base `25ff4eb0` plus the accompanying
patch. This fixes a native input adapter defect; it does not claim to fix the
owner's separately reported loaded-list overlap or qualify smooth presentation.

## Defect and regression

The pinned GPUI macOS converter recognized scroll start and end but mapped
`NSEventPhaseCancelled` through its default `Moved` branch. The one-line repair
maps it to `TouchPhase::Cancelled`. The existing GPUIO carousel handler can then
discard the gesture without committing its candidate selection. Pinch phases,
wheel thresholds, quiet fallback and dependency revisions are unchanged.

The regression constructs actual CoreGraphics pixel-scroll events, obtains real
AppKit events with `eventWithCGEvent:`, and invokes the production converter.
Four phases × three deltas cover start/move/end/cancel, both axes and zero-delta
terminal events. On the original converter, the test fails with CG phase 8:
actual `Moved`, expected `Cancelled`. With the repair, all twelve combinations
pass. This runs without a window, global event injection or focus changes.

The isolated test workspace copies the vendor source and root lockfile, retains
root dependency patches and enables runtime shaders for Command Line Tools-only
machines. Its unused root benchmark feature is removed from the copied manifest
to avoid resolving Criterion dependencies absent from the application lockfile.
The final helper verifies unchanged copied source bytes and no new or changed
registry/git pins. It does not modify the repository manifest or lockfile.

## Validation

```sh
GPUIO_JOBS=2 python3 scripts/test_macos_scroll_phases.py --output scratch/scroll-phases
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib carousel_track
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section carousel-track --images scratch/scroll-gallery
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/scroll-consumer
python3 scripts/test_gallery.py --section carousel-track --executable scratch/scroll-consumer/consumer/_build/default/main.exe --images scratch/scroll-installed
```

The final adapter helper passes with no unexpected dependencies. All 44 carousel
model/routing tests pass, including cancellation and quiet-timer ownership. These
TestPlatform tests cover the layer after platform conversion, not physical input.
All 19 vendored files match reconstruction from the pinned archive and updated
patch. Vendor formatting, strict native Clippy, example/catalog audits, actionlint
and whitespace checks pass.

The freshly rebuilt gallery and freshly staged independent consumer both pass
the existing four axis/theme desktop navigation cases, native editing, clipped
neighbor pointer selection, reorder/resize retention, immediate loop navigation,
disabled controls and page remount. Each app closes normally. Local runs have a
180-second exception/cleanup bound. This rechecks ordinary desktop behavior;
that fixture does not generate cancelled gestures.

Repository executable SHA-256:
`83f4d9bf0ecc9e422d607f996772720fd42686af0050644f1be82fd7a07d893f`.
Fresh installed executable:
`f6fe066302bb168b8c9da7aaae26e30f06437384219ba56a3c90123b9e4849e7`.

The [40-file archive](macos-scroll-phases-och41/reports.tar.gz) retains the original
failing adapter test, final passing report/log, source/patch/helper snapshots,
native tests, reconstruction, builds and desktop reports. Every member matches
the [manifest](macos-scroll-phases-och41/manifest.json).

## Unsuccessful attempts and limits

Earlier global CoreGraphics cancellation injection attempts selected the next
card both before and after the mapping change. Temporary native traces showed
the start arriving but no terminal cancellation event; the quiet fallback could
therefore finish the gesture. Direct process posting did not deliver the control
sequence either. These attempts are inconclusive about physical cancellation,
not a failing-before/passing-after desktop result. Their prototype remains local;
no unreliable GUI cancellation gate was added. All temporary tracing was removed
before the final rebuild.

The first isolated build lacked runtime shaders, and an initial assertion tried
to compare a type without `PartialEq`; both were corrected before the actual
before-case. The first passing adapter execution failed the helper's dependency
check because Cargo resolved optional benchmark packages. The final narrowed
harness passes that check. An initial desktop wrapper lacked `scripts` on its
Python import path and exited before launching the app; the corrected wrapper
passes. Earlier attempts remain recorded, rather than relabeled as acceptance.

Hardware trackpad cancellation, WindowServer delivery, nested scroll ownership,
full gesture/edge handoff, VoiceOver and presentation/resource qualification
remain separate requirements. Covering a benchmark window can explain a frame
wait timeout; it does not by itself explain overlapping rows. Final-source hosted
checks remain pending. Linux desktop qualification stays in deferred OCH-47.
