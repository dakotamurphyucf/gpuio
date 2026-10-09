# Canvas activation investigation — OCH-17

Status: unresolved native acceptance failure; diagnostics and deterministic
routing coverage added locally. This is not a canvas repair or macOS AX pass.

Hosted macOS run [36791905054](https://github.com/dakotamurphyucf/gpuio/actions/runs/36791905054),
revision `e54d2795df761e05067b5359a9b074c8590cb770`, failed the public canvas test
waiting for `Activated: Swift` after one AXPress on `Activate Swift`. The prior
keyboard move reached OCaml: the dump showed Swift at x=181 and `Moved: Swift`.
The available log does not identify where the activation was lost.

The move handler publishes a replacement scene. Publication immediately changes
the source lease; native presentation and the accessibility tree subsequently
catch up. Input is intentionally rejected while the presentation uses an older
snapshot, and an old accessibility callback remains invalid after installation
of the replacement. An enabled AX node observed before publication is therefore
not proof that its eventual callback will still be valid. This is a candidate
explanation, not a demonstrated cause of this hosted failure. Preserve these
fences; replaying AXPress could duplicate a real application action.

## Diagnostics for the next native run

`scripts/test_canvas.py` now starts only its child with `GPUIO_TRACE_CANVAS=1`
and `--trace-events`. Its existing one-press scenario, deadlines and cleanup are
unchanged. The example's trace flag logs received public canvas events without
enabling the automated self-test or its automatic shutdown. Native traces are
disabled by default and do not include scene text, labels or asset data.

- `GPUIO_CANVAS_AX_TREE`: the native snapshot revision/generation and the current
  input rejection reason when constructing semantic objects.
- `GPUIO_CANVAS_AX_ROUTE`: activation/selection intent, node/item identity,
  captured/native/published scene epochs, and callback rejection reason (or
  `None` when admitted).
- `GPUIO_CANVAS_EVENT_QUEUE`: event kind, node/tree/scene revisions and whether
  the native mailbox accepted the event. Acceptance does not prove OCaml receipt.
- `CANVAS_PUBLIC_EVENT`: the example's accepted public event, logged before its
  application handler updates state.

Compare these boundaries before changing behavior. No route trace after AXPress
requires investigation of AX delivery/tree handlers. A rejected route supplies
its fence. An admitted route with a queued activation but no public event moves
the investigation to bridge delivery and OCaml generation/handler admission.
Tracing is diagnostic overhead; do not use an enabled run for latency budgets.

## Deterministic scope

`host::canvas_view::accessibility::route_test` runs on GPUI TestPlatform and opens
no desktop window. It uses actual session scene publication, native canvas state,
focus eligibility and the event mailbox. It asserts one activation for the
current snapshot, none while publication is pending, none from the prior snapshot
after replacement, and one from the new route. Inactive windows, unprepared,
disabled, hidden and closed placements and replaced callback tokens must reject
without emitting an activation. A new callback after token replacement works.

The fixture deliberately installs a synchronously prepared replacement to control
that boundary. It does not test asynchronous worker completion, OS accessibility
delivery, physical presentation or OCaml receipt. The production input policy is
unchanged: a single rejection function now supplies both Boolean admission and
diagnostics. `scripts/gpuio test` includes these feature-gated library tests with
`--lib`, so enabling GPUI test support does not launch GUI integration binaries
on displayless Linux jobs.

Real macOS reproduction and resolution remain required. The latest separate
native image run could not complete under the current desktop-service
restrictions; no additional GUI run was attempted for this investigation.

Local validation on macOS arm64, 2026-10-01 UTC, `83eb87e` plus uncommitted
worktree changes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 \
  -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_TRACE_CANVAS=1 GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib \
  host::canvas_view::accessibility::route_test -- --nocapture
```

All exited successfully: 425 native library tests passed, two ignored; strict
Clippy and Dune build/tests/format passed. The focused trace-enabled run passed
and emitted distinct pending-publication/stale-snapshot rejection records and
successful activation queue records. It did not construct an OS semantic tree.
Python syntax, shell syntax and `git diff --check` also passed. These local
changes have not been committed, pushed or exercised by hosted Linux/macOS CI.
