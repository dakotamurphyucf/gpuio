# Sankey presentation — OCH-41

Implementation based on `bd70820`, 2026-10-06, macOS 14.5 arm64 / M1 Max.
**Scoped local qualification passes.** This adds four validated public options:
node corner radius, ribbon opacity, minimum ribbon width and label gap. See the
[contract](../design/sankey-presentation.md) and [example companion](../../examples/charts/samples/sankey_presentation.md).

The existing defaults are preserved. Labels now anchor beyond the facing node
edge; typed alignment prevents a displaced anchor from changing sides. Positive
ribbons widen in the shared paint/hit geometry, clipping endpoints to the plot.
Zero flows remain absent; data/selection IDs, raw values and native ownership
remain unchanged. Options schema 4 appends four float64s; style -1/data 1 remain.

## Completed local evidence

- Two new native tests cover alpha multiplication, clamped corner radii,
  edge-relative labels, minimum thickness, tiny viewport bounds, zero-flow
  exclusion and hit/selection provenance. The complete native suite passes
  **1009 tests, with two existing skips**.
- Public expect tests cover defaults, constructor and decoded-record bounds,
  roundtrip configuration and independently specified wire bytes. The new default
  options fixture is 100 bytes; options-3 frames remain explicit rejection cases.
- The full root chart gallery passes all existing families/directions/inspection,
  source updates, original-data paging and zero-resource teardown. New presets
  change actual plot pixels: Rounded nodes 109, Muted ribbons 179825, Visible
  small flows 4428 and Spaced flow labels 2022 in this run. Every preset retains
  keyboard selection of raw value 100; publication advances it to 101.
- The spaced-label screenshot was visually inspected. Strict Clippy and rustfmt
  pass. The full protocol suite passes **425 tests**, and the complete OCaml
  `@all @runtest @fmt` check passes.

The first focused Dune command omitted the alias `@` and was corrected. The first
build then caught the new gallery mode missing from an exhaustive stack-mode
match; adding its explicit nonstacked case fixed it. No expect output was blindly
promoted. A fresh independently installed gallery also passes the full chart
walkthrough. It additionally selects the widened tiny ribbon and observes its
original value **0.01**, then verifies source updates, the five original-data rows,
zero registered resources and normal shutdown. Its binary SHA-256 is
`1c87ce35dcf289f2b8823019a6f3a11d65393cf7378369ab22b53e6ae2686aee`.

[Evidence archive](sankey-presentation-och41/evidence.tar.gz) and
[per-file checksums](sankey-presentation-och41/manifest.json) preserve source overlay,
raw checks, root screenshots, exact commands and installed binary identity.
The structural catalog audit passes; it is not behavior acceptance. Changed
Markdown links and `git diff --check` pass. Hosted checks for these new changes remain pending. Publishing `548bcde` queued
run [37447717604](https://github.com/dakotamurphyucf/gpuio/actions/runs/37447717604)
and the workflow's concurrency policy cancelled older run `37445142735` before
acceptance. The archived source overlay records the pre-publication checkpoint;
that older run must not be counted as completed qualification.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
python3 scripts/audit_component_catalog.py
python3 -m py_compile scripts/test_gallery.py
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib --features native-image-tests,native-canvas-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section charts --images scratch/agents/root-20261004-resumed/chart-sankey-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section charts --workspace scratch/agents/root-20261004-resumed/chart-sankey-consumer
```

This is scoped functionality evidence. Rich multiline node labels and ribbon
color gradients remain catalog work; whole-catalog, VoiceOver, performance,
Linux GUI and release acceptance remain open.
