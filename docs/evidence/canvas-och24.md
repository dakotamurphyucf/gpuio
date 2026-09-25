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

Scoped Eio registration and integration of scene-handle ownership with views,
native painting/tessellation/cache budgets,
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

## Public immutable scene API

`Canvas_resource` and `Canvas_scene` now expose abstract, typed OCaml construction:
distinct resource/item IDs, phantom path/text/image kinds, paint/stroke values,
interaction policies, validated affine items, immutable scene snapshots and
application-bound native handles. Scene creation collects resources automatically,
checks exact identity conflicts and aggregate limits, and resolves theme colors.
Encoding rejects images belonging to another application. This is pure construction;
scoped native registration and the rendered widget remain pending.

Six expect tests in `test/canvas/scene_api_test.ml` pass, including exact agreement
with the independently constructed Rust scene fixture; invalid constructors and
unsupported transforms; owner/handle separation; canonical signed-zero identity;
theme resolution; 20,000 items sharing a 4,096-command path; and independent encoded,
resource, path-command, text and interaction limits. Resource bounds and canonical
bytes are cached once, while repeated use of the same immutable resource has a
constant-time identity fast path. Native label validation was aligned with Core's
ASCII whitespace rules and its vertical-tab rejection is tested.

Commands: isolated `dune runtest -j2 test/canvas`,
`cargo test -j2 -p gpuio-protocol --test canvas_scene`, protocol Clippy with warnings
denied and project format checks pass. The [design](../design/canvas.md) includes
a small construction example. No new native GUI behavior is claimed here.


## Scoped Eio publication

`Gpuio_eio.Canvas` adds scoped creation, stable borrowed handles, coalesced setters,
explicit scene-generation reset, error inspection and release. Initial creation
completes only after native publication. Cancellation suppresses late callbacks
and releases late allocations; native update rejection retains the prior accepted
scene and permits explicit recovery. The scheduler has one request in flight,
four staged uploads and a 256-entry/64 MiB conservative OCaml retention budget.

Eleven runtime expect tests cover 1,000 coalesced setters, reset ordering while
another reset is in flight, rejection/abort/retry, Begin admission failure,
cancellation at every upload boundary, initial failure/shutdown, foreign images
and scopes, a chunked 20,000-item scene, quota rollback, four-stage concurrent
progress, registration-count limits, cleanup priority, and reverting while an
older publication remains pending. Accounting returns to zero after disposal.
The logical charge includes scene representation and upload buffers; it is not an
RSS measurement.

The actual windowless `canvas_upload` check additionally passes public-API
publication, recovery from native resource-history rejection, 1,000 coalesced
resets checked against the native revision/generation, image reuse after asset
retirement, rejection of new image acquisitions, scope cancellation, 270 repeated
registrations/releases, reserved-lane progress beside 63 raw requests, and shutdown.
One local run completed in 3.904 seconds with zero UI commits/frames. This is a
transport/lifecycle workload, not a rendered-scene performance result.

Test iteration corrected two harness assumptions: retired asset generations
remain valid for existing leases (retirement is idempotent), and the last async
cleanup must drain before asserting an initially empty raw request lane. Stage
markers now identify the active integration check. The app has a 60-second
internal deadline and a 90-second external CI deadline. No window was opened.

Local isolated `dune build -j2`, full `dune runtest -j2`, and
`./scripts/gpuio check-fmt` pass. The existing macOS CI step runs this expanded
windowless integration check; hosted execution of the change is pending.

Native rendering, gesture/AX behavior, mesh/cache budgets, the interactive OCaml
example and Linux graphical evidence remain outstanding. Hosted M5 CI is still
pending; do not mark OCH-24 complete from this checkpoint.


## Mounted configuration and observation vocabulary

The pure `Gpuio.Canvas` interface now specifies viewport, bounded zoom policy,
selection/drag/pan controls, stable scene handles, theme-resolved selection color,
monotone commands and generation-stamped semantic observations. The matched
`Canvas_view_wire`/Rust `canvas_view` configuration has a bounded native decoder.
A foreign application owner becomes an unavailable source; it cannot leak its
native ID through generic extension properties.

Three OCaml expect tests and three Rust protocol tests pass. Both independently
construct the same 91-byte cross-language configuration fixture, including an
image-independent scene ID, UTF-8 label, nondefault viewport/limits/color and
command. Native checks reject every truncation, trailing bytes, oversized envelope
and label, invalid UTF-8, invalid geometry/zoom/color/command values. Typed event
conversion rejects malformed item IDs, transforms and publication identities;
pre-acquisition failure alone permits a zero revision/generation pair.

This checkpoint does not yet connect the configuration to `View.canvas`, native
painting, input, accessibility or command execution. The design records those
contracts and the ownership reason for a dedicated view configuration. OCH-24
remains in progress.


## Bounded native geometry and worker preparation

`canvas_mesh` implements cancellable local-space fills/strokes, including
rectangles, ellipses, quadratic/cubic curves and even-odd holes. Local anchors
preserve small translated shapes, and tessellating strokes before affine
transformation preserves their local-width contract. The native crate explicitly
depends on the already locked Lyon 1.0.19; both root and independent-consumer lock
changes add only that dependency edge, without upgrading packages.

Six mesh tests cover affine/reflected stroke area, subpixel shapes near the world
coordinate limit, holes/curves, ellipse accuracy, extreme flattening admission,
cancellation/invalid input and pre-growth vertex/index limits. Five scene-plan
tests cover normalized sharing, stroke/accuracy cache identity, unique-mesh and
expanded-output admission, partial-plan cleanup and retained-reader quota lifetime.
Four job tests cover 1,000 superseding updates, the 128-handle/two-worker bounds,
round-robin scheduling, stale completion/drop/shutdown fences and real off-thread
execution without UI/runtime access.

The local debug preparation workload of 20,000 translated rectangles uses one
shared mesh and accounts for 120,000 expanded draw vertices. It holds 804,784
charged bytes, prepares in approximately 23 ms (one recorded run 23.344875 ms),
and returns its accounting charge to zero on disposal. This is preparation plus
validation, not GPU rendering, frame latency or RSS. A shared-ellipse workload
exceeding the expanded draw budget is rejected and releases partial resources.

Local `cargo test --locked -j2 -p gpuio-native --lib` passes all 82 tests, including
24 canvas tests. Native all-target Clippy with `native-tests` and warnings denied
passes. Full isolated `dune build -j2` (including the independent extension
consumer backend) and `./scripts/gpuio check-fmt` also pass.
No graphical window was opened for these checks. Mounted host scheduling,
painting, font/image integration, input/accessibility and native application
acceptance remain outstanding.

## GPUI workers and mesh painting

`canvas_host` now schedules preparation on GPUI's background executor, refreshes
accepted-completion windows and drains worker-exit fences during application
shutdown. `canvas_paint` applies local affine geometry, world clips, viewport
mapping and shared per-frame expanded-vertex admission. These are native building
blocks; the public canvas widget and text/image/input/AX integration are pending.

The local macOS `native_canvas` executable passes actual GPU pixel readback with
`show: false` and `focus: false`, explicitly drawing its hidden window. Samples
verify rectangle/ellipse colors, a quadratic stroke, transformed world clipping,
scene revision replacement, pan/zoom and an actual resize from 240x240 to 320x280.
Device scale comes from the test window; no physical display-scale change is
claimed. The test also drops queued handles and checks that retained mesh and
scene charges return to zero after the final readers are released. The recorded
host metrics are `(completed=2, discarded=0, peak_workers=1, retained_bytes=0)`;
these are accounting values, not process RSS. Existing pure worker tests cover
supersession and the two-worker limit. The hidden window is closed before exit.

Commands through the isolated toolchain, with `GPUIO_JOBS=2`:

- `./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --lib`: 84 tests pass.
- `./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --features native-canvas-tests --all-targets -- -D warnings`: passes.
- `./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-canvas-tests --test native_canvas`: passes under a 90-second external deadline.
- `./scripts/gpuio exec dune build -j 2` and `./scripts/gpuio check-fmt`: pass, including the independent consumer backend build.

The CI definition builds/lints this feature on both required platforms and runs
the hidden-window GPU check on macOS. Hosted execution and Linux graphical
validation remain pending. This test does not claim public OCaml rendering,
keyboard/pointer interaction, accessibility or complete OCH-24 acceptance.

## Native interaction state and rendered position ownership

`canvas_state` now implements native selection, reverse-order transformed hit
regions, drag preview/completion/cancellation, pan/anchored zoom, scene-order
navigation, activation, keyboard-style movement and monotone commands. It keeps
completed positions separate from previews and reconciles them against immutable
scene publications. The mounted public view still needs to drive these operations
from actual input/lifecycle events and provide accessibility nodes.

Ten state tests pass, including source-transform acknowledgement without double
movement, same-generation publication retention, generation reset, item removal,
disabled/hidden/policy cancellation, world clips, affine translation bounds,
path control hulls beyond a hit region, zoom anchors/limits, input policy and
command failures/retries/deduplication. The 20,000-item workload retains at most
2,048 interactive entries/position overrides. Thirty-two state creation/disposal
cycles took approximately 47 ms in a recorded local debug run (47.460459 ms);
all scene accounting returns to zero after the final readers are dropped. This
measures native state construction/disposal, not a painted frame or process RSS.

The hidden-window GPU test now paints effective transforms from this state model
and switches its state snapshot when prepared geometry becomes ready. Direct state
calls move an ellipse, restore its original pixels on cancellation, complete a
drag, and preserve its moved pixels through a new scene publication and viewport
change. Scene/mesh accounting still returns to zero and the test closes its window.
These are actual GPU pixels driven by state calls, **not OS pointer/keyboard or
accessibility validation**. The same `native_canvas` command above passes under
the external deadline.

The full local native library suite passes 94 tests. Feature-enabled all-target
Clippy with warnings denied, full isolated Dune build (including the independent
consumer backend) and repository formatting also pass. OCH-24 remains in progress:
text/image painting, the mounted view/bridge/event pipeline, actual focus/input/AX
and the interactive public OCaml example are still required. Hosted platform gates
remain deferred until consolidated local milestone acceptance.

## Native text, images and deferred content work

`canvas_content` now shapes/caches native single-line text and paints raster/SVG
images using the existing scoped image leases and decoder/atlas scheduler. It
adds bounded text retention, variant counts and shared per-frame shaping,
glyph-work and image-request/draw admission. See the design for exact limits and
the distinction between retained accounting, native temporary work and GPUI caches.
Known-size SVGs use a direct variant request rather than an intrinsic decode first.

The local hidden-window GPU test passes white native text within a world clip,
absence of overflowing glyph pixels, clipped magenta PNM image pixels, two-color
SVG fill fitting, and subsequent viewport zoom/native resize. Original image
registrations are retired after the initial frame; the same scene still paints
and resamples its existing leases after publication and zoom. The test records
two text shapes and three image requests (one raster and two SVG sizes) before
extra workload, demonstrating cache reuse through ordinary frames and dragging.

Forty additional text color variants exercise the 32-new-shapes-per-frame budget.
A bounded per-frame trace observes a full 32-call frame with deferred work, later
completion of all 40 variants, and no frame exceeding that allowance. Requesting
513 variants reports `RenderLimit` while cache entries remain bounded. The trace
is necessary because GPUI may run multiple frames between test observations; the
test does not assume wall-clock scheduling determines a frame boundary. One
recorded pressure result held 106 text variants, two image variants, 106 cumulative
shaping calls and three image requests. That count is not a latency benchmark.

The same 513 color variants placed offscreen at a new font scale succeed without
consuming visible-line admission. They require one cold measurement, then no
additional shaping on repeated frames. Measured bounds have their own bounded
cache within the shared text accounting budget; full offscreen shaped lines are
not retained merely to support culling.

After disposal/shutdown the test checks zero retained text charge, zero mesh/scene
charge and zero retired encoded-image charge, then closes its hidden window. The
existing state/geometry pixel checks continue to pass. This is actual GPU painting
with direct state calls, not public OCaml widget or OS input/accessibility acceptance.

The native library suite passes 99 tests, including new shared text quota recovery,
physical font/SVG size admission, per-frame content limits and a direct SVG request
that queues only one variant, shares it, rejects raster misuse and releases pixels.
The native GPU command uses `native-canvas-tests --test native_canvas` under a
90-second external deadline. Hosted platform execution remains pending.

Canvas-feature all-target Clippy with warnings denied, the full isolated Dune
build including the independent consumer backend, and repository formatting pass
at this checkpoint. The final native test and all build/check processes exited.

## Public view and event bridge checkpoint

The typed `View.canvas`/Bonsai constructor now carries an application-owned
configuration through the reconciler and native tree. This checkpoint covers
the bridge and tree admission; the mounted native renderer and OS input/AX
integration still remain. No additional rendered-canvas capability is advertised.

Independent OCaml and Rust tests agree on the transaction bytes for kind 31 and
operation 36 and on event tag 40 with window/node/handler/source/scene identity.
They reject truncated transactions/events, trailing bytes, malformed observations,
missing sources and invalid publication pairs. The OCaml reconciler test verifies
callback-only replacement without bridge operations, handler rotation on config
change without remount, future revisions, foreign application ownership, typed
pre-acquisition failure and unmount suppression.

Eio scheduler tests exercise ordinary publication, pending-but-unsent uploads,
events before a Publish acknowledgement, failed publication, immediate reset
intent, another reset during publication, release, unknown sources and shutdown.
The earlier revision remains eligible during ordinary upload; it is rejected
after acknowledgement or immediately on reset intent. An event for the exact
in-flight Publish may arrive before its response; merely queued revisions cannot.

Native tree tests verify leaf/config validation, duplicate configuration rejection,
unchanged revision/retention after failed updates, all 128 permitted canvas mounts,
rejection of mount 129, quota reuse after removal, and zero configuration retention
after disposal. These are tree accounting checks, not native cache/RSS measurements.

Local macOS validation with the isolated wrapper and jobs=2 passes:

- `dune runtest -j 2 test/canvas test/runtime`.
- `cargo test --locked -j 2 -p gpuio-protocol --test canvas_view -p gpuio-native --test canvas_tree`
  (four protocol tests and two tree tests).
- `cargo test --locked -j 2 -p gpuio-native --lib` (99 tests).
- `cargo clippy --locked -j 2 -p gpuio-native --features native-canvas-tests --all-targets -- -D warnings`.
- Full `dune build -j 2`, including the independently packaged extension consumer.
- Repository `check-fmt` and `git diff --check`.

No GUI windows were opened by this checkpoint's checks. Native painting evidence
above remains from the earlier hidden GPU scenario; these bridge tests do not
extend that evidence to a public widget or keyboard/accessibility acceptance.
Hosted M5 validation remains pending.

## Mounted retained-tree renderer checkpoint

`native_canvas_view` uses the actual native `View`, session, retained-tree apply
path, canvas resource requests and event mailbox in a hidden, unfocused macOS
window. Unlike the earlier geometry/content harness, it reaches the renderer by
creating a `CanvasView` node and setting its typed configuration. It still does
not exercise the OCaml application or OS input/accessibility.

GPU readback verifies initial shape pixels and publication changes. A second
canvas is mounted and removed 32 times; each pair shares exactly 12 shape vertices
in one frame budget, including consecutive draws that verify budget reset.
One local debug run completed those lifecycle iterations in 391.86 ms; this is
not a frame latency, throughput or whole-process RSS guarantee.

The mounted checks also verify:

- Viewport commands produce scene-stamped asynchronous completion events.
- Hiding and showing an ancestor without an intermediate paint immediately
  discards jobs/prepared geometry/content while retaining the viewport and command
  watermark. Reshow prepares again without replaying the command.
- A valid scene with 4,097 unique rectangles exceeds the mesh preparation limit.
  The prior frame remains visible, exactly one `Render_limit` observation identifies
  the failed replacement revision, and a later valid publication recovers.
  A directed reporting check injects a separate old-frame content error and
  verifies that repeated draws do not make the two failure identities replay.
- A new scene generation restores the initial viewport, prepares at that zoom and
  actual device scale, and leaves the command watermark intact.
- A mounted lease still paints after registration release; after unmount, a new
  node cannot reacquire that identity and reports one pre-acquisition
  `Unavailable_scene`. Repeated paints do not repeat that failure.
- Unmount closes old states even if the test retains an earlier state reference;
  all leases/presentations are dropped and tree retention returns to zero.

After window removal and native worker shutdown, canvas scene, mesh and text
accounting are zero. The recorded worker metrics were `(39, 1, 2, 0)`:
completed jobs, discarded jobs, peak workers and remaining mesh bytes. The
two-worker limit is preserved. Accounting does not include unrelated GPUI/OS
resources or claim a process RSS ceiling.

The combined local command passes under a 90-second external deadline:

```
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native \
  --features native-canvas-tests --test native_canvas --test native_canvas_view
```

Both windows are hidden and removed by their respective harnesses, and both
processes exit. The full isolated Dune build also passes, including native FFI
and the independent extension consumer, as do canvas-feature all-target Clippy
with warnings denied, repository formatting and `git diff --check`.
CI now builds the new test on both
platforms and runs it beside the existing canvas GPU scenario on macOS; hosted
execution remains pending. Pointer/keyboard interaction, selection presentation,
accessible object semantics and the public OCaml diagram remain next work.
