# Pie captions and leader lines — OCH-41

2026-10-06, macOS 14.5 arm64, source base `fd2bc6f` plus this change.
The [contract](../design/pie-labels.md) adds Inside/Outside placement, bounded
label spacing and stable-ID caption/leader-color overrides. Native workers
measure outside captions using the captured font context. Two-sided spreading
and deterministic omission bound crowded layouts; literal oversized radii can
still enter caption space. Original weights, names, legend and selection values
remain independent. This is scoped chart evidence, not whole-catalog acceptance.

Options schema 9 uses 123 default bytes; style schema is -3. View/data remain
-1/1. Both packages must match. Paired tests assert the new placement/gap bytes
and the independent caption/color suffix `01070102cebb01070103`. They reject old
schemas, malformed/truncated/trailing frames, invalid numbers/colors/UTF-8,
duplicate IDs and oversized counts/text. Core tests also check theme resolution.
The style envelope admits 128 KiB, with independent 32 KiB text budgets for pie
and Sankey overrides. A combined full-budget configuration decodes and charges
all retained entries/text; bounded decoding precedes admission to the native tree.

Pure geometry tests verify leader anchors at each slice's actual outer radius,
source provenance after reorder, equal-radius omission, zero/tiny sweeps,
explicit hiding and invalid measurements. Dense 256-slice, 60-by-36 and 1-by-1
plots keep caption rectangles bounded and nonoverlapping, omitting rows when
needed while preserving wedges. This is not a guarantee that oversized wedges
or all leader segments cannot intersect.

The windowless platform-font harness runs real native shaping on a worker.
Unicode, proportional widths, stable-ID replacements, hidden captions and
cancellation pass. Prepared outside layouts pass at test scales 1/1.25/1.5/2.
These are worker preparation scales, not four physical monitors or four rendered
GPU captures. The existing Sankey font cases also pass.

The production hidden-window chart harness verifies actual green caption pixels
and independently red/blue leader pixels. It covers per-slice radius overrides,
font weight 400/700, a 120-pixel narrow view, inside/outside transitions, individual
and global hiding, and unchanged source data. Existing Sankey/radar/streaming
cases pass in the same harness. This is GPU rendering evidence, not physical
mouse, IME, VoiceOver or display-presentation timing acceptance.

The full OCaml build, expect suite and formatting checks pass. All 438 protocol
tests and 1,031 native unit tests pass (two existing native skips remain). Strict
native Clippy with both native test features and Rust formatting pass.

The public gallery walkthrough passes native AX switches, Home/Enter selection,
both theme transitions, radius controls, custom/hidden captions and spacing.
The original-data table still exposes Reasoning=44 and Other=10 after custom
captions hide Other's plotted text; selection remains Reasoning=44. Unchanged
sample publication and return to defaults pass. The outside-label screenshot was
visually reviewed. Final image/chart/canvas registrations and registered source
bytes return to zero; the app closes normally.

The [adjacent walkthrough](../../examples/gallery/charts_page.md#pie-captions-and-leader-lines)
explains the actual Bonsai toggles, effects, reactive derivation, GPUIO APIs,
stable IDs, native ownership and adaptation. An owner-authorized GPT-6.1 Sol
documentation agent drafted it; primary review checked it against implementation.
The example inventory remains 417 source files in 260 reviewed groups.

The fresh installed consumer passes the same complete walkthrough, catalog check
and zero-resource cleanup using an isolated prefix without changing an opam
switch. Its executable SHA-256 is
`43870731ac2196e6294c995f7863b12bee861e8ef7cac4b4f77626291fb63b6e`.
All local build/test processes exited and their test windows closed.
OCH-41/OCH-17 remain open. No Linux GUI or broader release acceptance is inferred.
Hosted run 37502930557 covers older `328267a`; current-source CI remains required.
No VoiceOver settings were changed.

## Reproduction and retained evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_options --test chart_style --test chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --lib chart_ --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_labels --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section chart-pie --images scratch/agents/root-20261004-resumed/pie-labels-gallery-images
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-pie --workspace scratch/agents/root-20261004-resumed/pie-labels-installed
python3 -m py_compile scripts/gallery_pie.py
python3 scripts/audit_example_docs.py
git diff --check
```

The [raw logs/screenshots](pie-labels-och41-logs.tar.gz) and
[verified manifest](pie-labels-och41-manifest.json) retain 20 files
(6,283,963 uncompressed bytes). Initial compilation exposed missing test-record fields and an
Arc/config comparison in the new fixture; both were corrected. Review also caught
an incorrect Python helper argument before the gallery run. These were harness
corrections, with no weakened assertions or runtime failure discarded.
