# Retained canvas implementation evidence

OCH-24 is in progress. This ledger distinguishes the implemented pure geometry
from the scene registry, widget and platform acceptance still required.

## Geometry and paths

Local macOS arm64, isolated stock OCaml 5.3/Core 0.17 and pinned Rust toolchain:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 test/canvas test/protocol`
  passes. Public abstract constructors reject non-finite/out-of-range coordinates,
  invalid rectangles, singular transforms, degenerate polygons, invalid path
  topology and excessive command counts. Calculated transforms/points preserve
  the admission invariant.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --locked`
  passes the protocol suite, including seven new canvas tests. Corresponding
  OCaml and Rust cases cover local-first composition, reflections, rotation,
  shear, 1,764 inverse round trips, ellipse/concave-polygon boundaries,
  world-clip intersection and empty clips. Open and closed path eligibility is
  checked separately from command/coordinate admission.
- `test/fixtures/canvas-v1-path.hex` agrees with independent OCaml/Rust path
  construction for all five command tags; the OCaml check also decodes the exact
  buffer and requires full consumption. The native bounded scene decoder remains
  future work; this fixture alone does not prove hostile-input admission.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-protocol --all-targets -- -D warnings`
  and `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` pass.

Tests exposed and fixed a translated inverse's floating-point boundary miss and
hits admitted by clips meeting only at an edge. Inverse point mapping now solves
after subtracting translation; containment has an explicit small numerical
tolerance. Clips with zero-area intersection never hit.

Source: `lib/core/canvas_geometry.{ml,mli}`, `lib/core/canvas_path.{ml,mli}`,
`lib/protocol/canvas_wire.ml`, `rust/protocol/src/canvas.rs`, `test/canvas/`.
The [design](../design/canvas.md) records coordinate and path contracts.

## Scene wire and admission

The version-1 scene wire schema, Rust bounded decoder and domain admission now
pass the same OCaml/Rust protocol and Clippy checks. `canvas-v1-scene.hex` covers
all drawing/resource/hit-region kinds; both languages construct and decode the
fixture independently. Every truncated prefix and a trailing byte are rejected
by the native decoder. Tests reject hostile outer/nested counts, aggregate path/
text/interactive budgets, duplicate IDs, stale or wrong-kind references, invalid
geometry and unsupported text/image transforms. Pure topmost hit tests cover
reorder, transformed regions, clipping and decorative items not intercepting input.

`cargo test -j2 -p gpuio-protocol --test canvas_scene -- --nocapture` passes five
tests. One local debug-profile measurement decoded and validated 20,000 items
sharing a 4,096-command path: 1,589,440 encoded bytes in 45.409 ms. This measures
codec/domain admission only; it is neither frame timing nor rendered-scene/RSS
evidence. Native scheduling, cache and disposal workloads still require tests.

Source: `lib/protocol/canvas_scene_wire.ml`, `rust/protocol/src/canvas_scene.rs`,
`rust/protocol/src/decode/canvas.rs`, `rust/protocol/tests/canvas_scene.rs`,
`test/canvas/scene_codec_test.ml`.

## Remaining acceptance

Ergonomic public scene/item/resource constructors, application-bound native leases,
bounded uploads and atomic publication, native painting/tessellation/cache
budgets, images/text, accessible native selection/dragging/pan/zoom, an OCaml
diagram/plot example, measured large-scene/repeated-disposal workloads and
integrated chat showcase are not implemented by this checkpoint. No native canvas
window, Linux canvas acceptance or hosted M5 CI is claimed. OCH-24 remains open.
