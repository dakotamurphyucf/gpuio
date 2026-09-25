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

See the registry and bridge checkpoints below for native resource ownership and
raw transport; the scoped registration adapter and widget are still pending.

Ergonomic public scene/item/resource constructors, OCaml application-owner checks,
scoped Eio registration, native painting/tessellation/cache budgets,
rendered images/text, accessible native selection/dragging/pan/zoom, an OCaml
diagram/plot example, measured rendered-scene/repeated-widget-disposal workloads
and the integrated chat showcase remain unimplemented. No native canvas
window, Linux canvas acceptance or hosted M5 CI is claimed. OCH-24 remains open.

## Native scene registry

`rust/native/src/canvas_store.rs` and the native `Session` now implement creation,
ordered upload staging, revision/generation-checked atomic publication, abort,
release, retained snapshots and terminal shutdown. Image acquisition uses the
same session's asset store. Existing snapshots retain asset leases after encoded
registration release; new scenes cannot acquire a retired asset. Resource history
rejects generation rollback and same-generation content changes even after removal;
explicit scene-generation reset permits a fresh identity namespace.

Local checks pass:

- `dune runtest -j2 test/canvas test/protocol`, including paired OCaml/Rust upload
  request frames with opaque binary chunks.
- `cargo test -j2 -p gpuio-native --lib`: all 67 tests, including nine canvas
  registry/session tests. Coverage includes partial uploads, rejected publication,
  abort, stale slot reuse, image retirement, negotiation, shutdown and retained
  snapshots after release.
- Full `cargo test -j2 -p gpuio-protocol --locked`, native/protocol Clippy
  `--all-targets -- -D warnings`, and project formatting checks.

A local 20,000-item retained-snapshot pressure test reached revision 28 with 28
held readers and 132,005,046 charged bytes before publication rejected the next
snapshot. Dropping old readers reclaimed quota and allowed publication to retry
using the already staged bytes. Final registration/reader disposal returned the
data accounting charge to zero. Separate history-count/history-byte pressure
tests recover through explicit reset. These are logical capacity/ownership
measurements, not RSS or GPUI rendering/cache measurements.

The initial registry checkpoint did not include bridge integration; the following
checkpoint adds that transport. The
[design](../design/canvas.md) distinguishes the 128 MiB retained/staged quota,
bounded temporary publication work, fixed slot metadata, and native rendering
budgets still to be implemented.

## OCaml/Rust bridge

Canvas requests and responses now cross the production bridge (message 13,
event 39, registration capability 1073741824). The raw Eio request lane owns
correlations, admission and terminal completions; scoped public registration
remains pending. Native mailbox tests prove responses survive a full input queue
and preserve correlations under command backpressure. Paired OCaml/Rust tests
cover request frames, response tags, binary chunks, invalid correlation, oversized
chunks, every request truncation and trailing data.

The local windowless `examples/canvas_upload/main.exe` passes through actual GPUI
startup and FFI: a 20,000-item scene larger than one bridge message, multi-chunk
upload, incomplete/rejected publication, revision-preserving retry, explicit reset,
stale slot reuse, 63 simultaneous request lanes plus local rejection, and clean
shutdown with zero UI commits/frames. The process returns successfully and opens
no test window. Its README gives the runnable command. The macOS CI workflow
includes this check; hosted execution remains pending.

Eight requests queued immediately before shutdown and one attempted after shutdown
each complete with Closed. This checks terminal runtime callbacks through actual
FFI, beyond the pure session shutdown test. Full local `dune build -j2`,
`dune runtest -j2`, `cargo test --workspace --locked -j2`, native/protocol Clippy
with `--features gpuio-native/native-tests --all-targets -- -D warnings`, and
format checks pass. The runtime check uses a 45-second external deadline and
exits normally; no GUI acceptance is inferred from its windowless execution.
