# Hover paint after layout changes — OCH-41 / OCH-17

Local macOS arm64, based on `0670136e`, 2026-10-07. This checkpoint follows an
actual-window table-styling walkthrough. It does not complete either ticket or
establish a fix for the separately reported loaded-list scrolling appearance.

## Reproduced defect and repair

After the gallery changed theme/scale, a row retained native hover paint after
the pointer moved outside the table. The control bounds were stable before and
after the move; waiting longer did not repair it. In the Light/Compact case, the
first row remained `(228,232,236)` while unhovered siblings were `(242,244,247)`.
A later style update caused the expected repaint. Failed captures are retained.

A small production-GPUI/TestPlatform case reproduced the issue without Bonsai or
the table adapter. A widget's first paint or a programmatic layout change can put
it under a stationary pointer. GPUI computes paint styles using the current
hitbox, but the stored hover transition state was updated only by mouse events.
It could therefore paint hovered while remembering false. On the next pointer
exit, false equalled that stored false, so no redraw was requested and the
previous hover paint remained visible.

`Interactivity::paint_mouse_listeners` now remembers the current painted hover
state separately from the stored hover state used during layout. Later pointer
movement requests a redraw if either state differs from the current hit test.
This preserves existing hover-driven layout updates while clearing stale paint.
It adds no polling, timers, OCaml input callbacks or unconditional redraws. The public API/protocol is unchanged.

Two before-fix native tests fail at the observed color after pointer exit: initial
hover paint and a widget moved under a stationary pointer. Four repaired tests
cover those cases for both element and group hover. A fifth covers hover-driven
width changes for both kinds. They inspect actual inherited paint color and deliver native input events, rather than asserting the private
state field changed. They are not physical GPU or screen-reader tests.

## Public gallery walkthrough

`scripts/test_gallery.py --section table-presentation` exercises the public
Collections Result table with rich headers. It compares actual window pixels for
8% data-dependent accent tint, 16% hover tint, untouched sibling rows and reset,
in Dark/Light at Compact/Comfortable/Large scales. It also checks fixed cell
geometry, header AX identity during style changes, actual pointer and foreground
Down-key selection, the independent Inspect action, selection retention and page
retirement/remount. It does not use the clipboard or change VoiceOver settings.

The early harness labelled themes incorrectly because the gallery button shows
its current theme; clicking it toggles to the other theme. The corrected helper
verifies the rendered palette. Another early assertion compared a header AX
object across scale/visibility transitions. The final paint-only identity check
captures its baseline after layout settles. AX objects can change across those
transitions; this is not evidence that the native widget owner was destroyed.
No cross-scale AX identity or full retained-editor behavior is claimed here.

The failing pixel assertion was preserved, rather than weakening its color bound.
It exposed the native hover defect described above. Exact source hashes, command
results, before/after logs and selected window images accompany this checkpoint.

## Validation and boundaries

The first candidate macOS gallery walkthrough passed all six theme/scale cases and
pointer/Down-key/action/retirement checks, then closed with exit zero. Its binary
SHA-256 is recorded in the retained log. The earlier Light/Compact hover mismatch
was absent on that renderer with the same pixel assertions. Final revised-renderer
results follow below.

The first repair passed **1,183 native tests, two existing skips**, strict lint,
formatting, the workspace suite and the visible table check. Review then added a
hover-layout-width guard, which failed: synchronizing the stored state during
paint could suppress the next layout update. That candidate was not accepted.
The revised implementation retains painted and layout state separately. On that
final source, all **five focused regressions**, **1,184 native tests (two existing
skips)**, strict workspace/all-target Clippy, workspace formatting and the gallery
build passed. The actual macOS six-case table walkthrough passed again with exit
zero, including the unchanged pixel assertions and foreground input checks.
Final gallery SHA-256: `2e17a92e8e00f82d3f64d6862d07c945b2febdf724ea67b5c65081ba5a9c9821`.
The earlier complete workspace pass applies to the first candidate; it is not
reported as a final-source workspace result. Required final-source hosted checks
remain open.
The catalog inventory and example audit also passed (429 sources, 266 reviewed
groups, zero pending); these structural checks do not establish physical coverage.
Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --workspace --all-targets --locked -j2 --features gpuio-native/native-canvas-tests,gpuio-native/native-image-tests,gpuio-native/presentation-diagnostics -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section table-presentation --images scratch/table-presentation
python3 scripts/audit_component_catalog.py
python3 scripts/audit_example_docs.py
```

GPUI reconstruction compares all 159 files, excluding generated Cargo.lock,
against the pinned upstream archive plus the checked patch. Patch SHA-256:
`60653a7d80d8b893bea050f7504ca8762db81153cb615d0fba7307cbdebedb4d`.
No upstream unit-suite or Linux graphical result is implied by these local checks.
Current-source hosted Linux/macOS checks, VoiceOver, wider header/row state/clip
coverage and consolidated resource/performance/release requirements remain open.

[Evidence archive](hover-layout-och41/reports.tar.gz) contains logs, source,
reconstruction hashes and selected before/after images;
[the manifest](hover-layout-och41/manifest.json) verifies every member.
