# Current chart-gallery integration and callback review

2026-10-07, OCH-41, macOS 14.5 arm64 / Apple M1 Max, executable built from
`46b291d`. Documentation-only edits followed that commit during this run.

The complete public `charts` walkthrough passes with the new baseline source
schema and controls included. It exercises seven families and mixed layers,
all four Cartesian directions, keyboard selection, mark appearance, axes/grids,
pie/radar/Sankey presentation, retained rich radar labels and inspection content,
categorical point/band layouts, ordinal colors, stacking, original-data browsing,
source updates and branch lifetimes. The final background branch verifies
per-observation origins, Mean incompatibility/recovery, stable reorder and reset.
The driver finishes with zero registered images/charts/canvases, zero source bytes
and normal shutdown. The reordered interval capture was reviewed.

This is the repository executable. The separate
[bar-baseline checkpoint](bar-baselines-och41.md) qualifies its freshly installed
consumer for that branch; this run does not imply the complete current chart
sequence was rerun in that consumer. Earlier feature checkpoints retain their
own installed-consumer scope and source revisions.

The [chart source review](../catalog/charts-review.md#callback-and-geometry-boundary)
now compares pinned BarChart/Bar callbacks, Plot, Scale, Pie and Arc helpers with
current public contracts. Data-owned brushes and typed native ramps cover their
documented cases. Arbitrary live pixel-bound callbacks and raw plotting helpers
remain explicitly unsupported surfaces; they are not silently labeled equivalent.
The static extension SDK can implement a separate plot, but cannot inject a fill
callback into the built-in Chart worker or borrow its private store. This review
adds no runtime feature, accepted deferral or release waiver.

The root-module ledger now reflects the implemented chart extensions and current
view/options/style/data schemas **-2/9/-9/3**, with feature-specific evidence.
The catalog audit passes snapshot checksums and structural mappings for 146 root
modules into 43 families, 73 GPUIX style fields and 22 events. Structural coverage
does not establish complete configuration behavior or release acceptance.

```sh
python3 scripts/test_gallery.py --section charts --images scratch/agents/root-20261004-resumed/chart-catalog-current-images
python3 scripts/audit_component_catalog.py
git diff --check
```

[Logs/capture/metadata](chart-catalog-current-och41-logs.tar.gz) and the
[verified SHA-256 manifest](chart-catalog-current-och41-manifest.json) preserve
the exact commands, successful exits, source revision and executable hash.
No production code changed for this integration run. Current-source hosted/Linux
build/unit/consumer checks, full catalog acceptance, physical accessibility/IME,
presentation/resource/performance and distribution/API release gates remain open.
OCH-41 and OCH-17 remain In Progress; Linux desktop qualification stays OCH-47.
