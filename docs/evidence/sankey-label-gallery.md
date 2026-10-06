# Measured Sankey labels in mounted views — OCH-41

2026-10-06, macOS 14.5 arm64, source base `66dad21` plus the source hashes in the
[manifest](sankey-label-gallery-manifest.json). This extends the
[worker measurement checkpoint](sankey-label-worker.md) with actual mounted text
pixels and public OCaml gallery behavior. It is scoped local evidence, not release,
Linux desktop, VoiceOver, performance or arbitrary dense-graph acceptance.

The public **Flow labels** mode uses three columns, nodes stored out of topology
order and main/tiny/zero incoming edges. Pure sample functions provide options and
ID-keyed captions; Bonsai controls placement, long Unicode captions, inherited
font weight, width and visibility. Source data, native ownership and selection
retain the existing contracts. The [sample walkthrough](../../examples/charts/samples/sankey_presentation.md)
and [page walkthrough](../../examples/gallery/charts_page.md) explain the code.

## Rendering regression found through screenshot review

The first root walkthrough passed semantic/layout checks, but visual review found
“Recorded activity” painted only “Recorded” and “Delivered locally” only “Delivered”.
The fixed-height caption used `text_ellipsis`, which does not disable wrapping in
the pinned GPUI. Accessibility retained the full string, hiding this failure from
text-presence assertions. The correction uses `truncate` on the caption and its
container so each prepared line explicitly stays one line.

A hidden-window GPU regression changes the red test caption to “IN X”. Before the
fix, its colored span was only 20 physical pixels at scale 2, corresponding to
“IN”; the regression failed. After the fix, the full-fit outside 200-pixel case
passes at normal and bold weight. Narrow/inside captions may legitimately
ellipsize; their checks require actual colored text, not the full-word width.
An intermediate assertion incorrectly required full text there and failed; that
log is preserved. Both before/after tests start with the full-fit outside case.

## Actual native and public checks

`native_chart_view` now checks three colored label regions using GPU readback,
left/right/above placement, source-index-independent node hit provenance,
Inside/Outside, 120/200 logical pixel widths, global hiding/restoration and rapidly
superseded font weights ending at 700. This executed at the local display scale 2;
it is not four-scale rendered-label qualification. The earlier worker harness's
four preparation scales remain a separate measurement claim. Existing native
chart teardown, two-window stream and final metrics also pass: `(73, 2, 2, 0, 0)`.

The root public walkthrough passes rich/long/hidden captions, inside/outside,
420-pixel width, bold style, keyboard selection Input → Processing 100 → 101 on
update, seven-row original data and page-scope retirement. Screenshots confirm
complete multiword captions after the fix. A fresh staged installation and
independent consumer build pass. Its first walkthrough failed because the new
page-scrolling helper was used on a fixed header theme button. Correcting the
driver to address that header directly, without rebuilding the installed binary,
passes Dark → Light → Dark plus the full sequence. Both failures and successes
are retained in the [raw evidence archive](sankey-label-gallery-logs.tar.gz).

The rerun reports zero image/chart/canvas registrations, zero registered source
bytes and normal application shutdown. Tests close/reap their windows. Background
GPU readback is distinct from the public foreground keyboard checks. No OS IME,
clipboard or assistive-technology behavior is implied.

Commands (repository environment, two build jobs):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-canvas-tests --test native_chart_view
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-labels --images scratch/agents/root-20261004-resumed/sankey-label-gallery-fixed-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-labels --workspace scratch/agents/root-20261004-resumed/gallery-installed-measured-labels
python3 scripts/test_gallery.py --executable scratch/agents/root-20261004-resumed/gallery-installed-measured-labels/consumer/_build/default/main.exe --section chart-labels --images scratch/agents/root-20261004-resumed/sankey-label-installed-images
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native --features native-canvas-tests --lib --tests --no-deps -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j 2
```

Native test, gallery builds/root/fresh-installed rerun and strict Clippy pass.
Full `@all @runtest @fmt` and `cargo fmt --all -- --check` pass (exit 0). The initial fresh
consumer command exits 1 for the documented driver error; its rerun exits 0.
Paired protocol/library/worker checks remain in the preceding worker checkpoint.
Current-source hosted Linux checks and broader OCH-41/OCH-17 requirements remain
open; the public API retains options schema 6 and style/data schemas -2/1.
