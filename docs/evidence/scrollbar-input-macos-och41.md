# Scrollbar input and animated narrowing — OCH-41

2026-10-08, macOS 14.5 arm64, Apple M1 Max, based on `c2ac9562`.
The ordinary-viewport gallery walkthrough exposed a native animated-width defect.
This is scoped evidence; OCH-41/OCH-17 remain open.

## Native repair

Turning off Gradient thumbs changes the example's 18-pixel track to the default
16-pixel track while retaining finite width animation. The renderer resolved a
16-pixel interaction envelope, then replaced the track width with the sampled
18-pixel width from its previous accepted paint. Geometry validation correctly
rejects a track wider than its envelope. The adapter consequently suspended
ranges, discarded that frame and blurred focus; a real-window failure capture
shows both bars missing despite the fully visible overflowing viewport.

`scrollbar_widget.rs` now includes the sampled track width in that frame's
interaction envelope. As the animation settles, the envelope contracts to the
configured state maximum. Normal hover/pressed refinements retain their common
maximum; no new scroll handle, offset, timer or callback crosses the bridge.
No protocol, OCaml API, dependency or upstream patch changes are involved.

The added `narrowing_animated_appearance_keeps_ranges_and_the_scroll_owner`
TestPlatform regression paints 18-pixel tracks with nonzero offsets and focused
horizontal range, then requests default 16-pixel tracks over 120ms. Before the
repair it fails the retained-focus assertion. Afterward, both ranges remain
painted and accessible, offsets/focus survive and the geometry settles to 16px.
The full native suite passes 1,190 tests with two existing platform skips.

## Physical walkthrough scope

The focused `--section scrollbars` driver checks Light/Dark ×
Comfortable/Large/Compact for this ordinary viewport:

- Matching-axis arrow keys, Home/End and AX Increment/Decrement on both ranges;
  vertical Page Down/Up. Numeric range observations verify movement/endpoints.
- Track click and captured vertical thumb drag retain the existing button focus.
  Escape cancels capture; subsequent held-pointer movement no longer changes
  the offset. Cancellation preserves movement already applied.
- Both nonzero offsets survive thumb styling, animation and visibility-mode
  updates, axis removal/restoration and custom metadata removal/restoration.
- Adding six rows increases the vertical maximum while preserving both offsets;
  resetting rows and page retirement also succeed.

This uses guarded foreground keys, OS pointer events and native accessibility
range actions. The driver changes no clipboard or system preferences. Its
settled samples do not qualify exact animation/presentation timing, idle CPU,
retained memory, VoiceOver or hardware trackpads. It covers the ordinary owner;
managed list/tree/table ownership has separate tests. It does not explain the
reported loaded-list row overlap.

The initial fixture tried to query a horizontal bar clipped by the outer page.
It now reveals the viewport bottom first. Subsequent attempts retained the real
styling failure even with the viewport revealed and action settlement; all logs
are preserved. Earlier attribution of that later failure solely to page clipping
was an unconfirmed hypothesis, superseded by the native regression and repair.

## Reproduction

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @test/gallery/runtest @fmt
python3 scripts/test_gallery.py --section scrollbars --images scratch/scrollbars-root
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/scrollbars-consumer
python3 scripts/test_gallery.py --section scrollbars --executable scratch/scrollbars-consumer/consumer/_build/default/main.exe --images scratch/scrollbars-installed
```

Native walkthroughs use 240-second exception watchdogs through normal harness
cleanup. The new four-minute CI step awaits updated-source hosted execution;
Foundation 37789987337 predates this repair. Linux nongraphical requirements and
all other release gates remain unchanged.

## Results and retained artifacts

Final repository `scrollbar-input-native-005` and fresh installed
`scrollbar-input-installed-001` each pass all six cases and finish with
`GPUIO_GALLERY_AX_OK`. Both processes close and are reaped normally. The fresh
consumer stages public libraries without modifying a switch and passes both
catalog schema checks. Root and installed binary hashes are recorded in the
[manifest](scrollbar-input-macos-och41/manifest.json).

The [verified archive](scrollbar-input-macos-och41/reports.tar.gz) retains
38 artifacts (12,725,916 bytes): all physical attempts, the failure capture,
final captures/reports, native before/after logs, root/consumer builds and strict
Clippy. The first two native-regression attempts failed to compile the fixture
(private axis helper, then a borrowed-context cleanup); the corrected third
attempt is the actual pre-repair behavioral failure. No expectations were
promoted. Initial physical attempts are not counted as passing acceptance.

The failure capture and final Light/Comfortable capture were inspected. Native
regression/full suite, gallery expect tests/build/format, fresh consumer,
`cargo clippy -p gpuio-native --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings`,
Python syntax/focused Ruff, unchanged legacy lint (244), docs inventory
(432 sources/268 groups), catalog audit, actionlint and whitespace checks pass.
