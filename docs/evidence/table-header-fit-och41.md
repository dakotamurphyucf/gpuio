# Public rich-header button sizing — OCH-41

Local macOS arm64, 2026-10-07, based on `5ea879ee`. The Collections Result table's
**Inspect** button used the default 44-pixel height inside a shorter native header
slot, clipping its label and extending its logical bounds into the body. The
before-image and failed pixel/geometry assertions reproduce this example defect.

The public OCaml example now sets a 28-pixel height, small explicit padding,
18-pixel line height, scale-aware font and palette surface/foreground colors.
The change is entirely ordinary `View.button`/`Style` composition. Native table
geometry, selection ownership and the shared Rust implementation are unchanged.
The [adjacent walkthrough](../../examples/gallery/collections_page.md) explains
why arbitrary rich content must fit its native header slot.

## Actual window checks

The extended `table-presentation` walkthrough passes all six Dark/Light ×
Compact/Comfortable/Large combinations. It checks the full button rectangle stays
above the first body row and measures foreground glyph pixels inside the button,
with top/bottom margins so a clipped label cannot pass on AX bounds alone. The
existing row tint/hover/reset, untouched-sibling and fixed-cell-geometry checks
also pass. A Light/Large capture was visually inspected; the full label is visible.

The test clicks the header button through actual pointer input, changes the table
selection to reset the notice, then focuses the button and activates it with a
foreground Space key. Both actions produce the independent header notice. Native
Down-key selection, rich-header AX identity during styling, retained selection
and page departure/remount also pass. The owned application closes with exit 0;
no clipboard or VoiceOver settings are changed.

Final gallery SHA-256:
`aef84311d510261666e9692689c7c7f231e74b57543f5a4971c117b9d2220a37`.

## Commands and failed attempts

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec ocamlformat --check examples/gallery/collections_page.ml
python3 scripts/test_gallery.py --section table-presentation --images scratch/table-header-fit
python3 scripts/audit_example_docs.py
```

The build, formatting and actual window check pass. The documentation inventory
reports 429 sources in 266 reviewed groups, no pending groups; this is structural
coverage, not proof of native behavior. No new Rust-suite result is claimed for
this OCaml example change.

The first build used nonexistent `Padding_x`/`Padding_y` constructors and failed.
It was corrected to public `Padding`, `Padding_left` and `Padding_right`. A test
was inadvertently launched before that failed build result was inspected; its
hash identifies the old executable, and its reproduced clipping failure is not
reported as fixed-source validation. The corrected build completed before the
accepted run. All failed logs and owned-child cleanup outcomes are retained.

[Logs, source and selected captures](table-header-fit-och41/reports.tar.gz) have a
[SHA-256 manifest](table-header-fit-och41/manifest.json). This qualifies the example's
header fit and scoped input behavior, not every rich-header clipping scenario,
VoiceOver navigation, resource limits or consolidated OCH-41/OCH-17 acceptance.
