# Exercise scoped resources before each closed-window sample

[gpuio_lifecycle_workload.ml](gpuio_lifecycle_workload.ml) implements the single
`run` function declared in [its interface](gpuio_lifecycle_workload.mli). This is
shared diagnostic code, not an executable. The [ordinary performance driver](../performance_lifecycle/README.md)
calls it with native entity auditing disabled; the [resource audit driver](../resource_audit/README.md)
selects a dedicated audit backend and enables that option. The backend choice is
static: a Boolean does not install a native extension into an ordinary application.

`App`, `Scope`, `Asset`, `Document` and `Canvas_resource` alias Eio APIs; `Scene`
is a pure `Gpuio.Canvas_scene` description. `B = Bonsai.Cont` builds the reactive
graph, `E = Bonsai.Effect` describes UI operations, and `V = Gpuio_bonsai.View`
builds view descriptions. `Probe` is qualification instrumentation, rather than
an application feature. Read the resource constructors, `component`, the UI bridge,
the cycle loop and its closed-window assertions in that order.

## Descriptions, registrations and borrowed views

`Resources.t` stores three owned registrations: an image, streamed document and
retained canvas. `scene cycle` constructs a validated rectangle with item ID `1`,
bounds `(16, 16, 200, 100)` and blue fill. This allocates no native canvas;
`Canvas_resource.create` registers the description later. `image_source cycle`
constructs a binary P6 PNM containing 256 × 256 RGB pixels. The first color byte is
`cycle mod 256`; the other bytes are 100 and 240. Sources differ across this finite
33-cycle full workload, avoiding an identity-cache shortcut around image decode.
An indefinitely extended workload would repeat that byte after 256 cycles.

`component` uses `let%arr` to read two `B.Expert.Var` values: optional resources and
an extension command `(sequence, request)`. `None` renders a preparing message.
`Some resources` creates configs borrowing their handles; rendering these views
does not transfer ownership or register duplicate resources. The document uses
Markdown with a 180-pixel viewport; the image requests 256 × 256; the canvas
requests 256 × 180 and sends sequenced `Reset_viewport` command `1L`.

Event handlers return effects. Image `Ready` asserts decoded dimensions and sets
`image_ready`; canvas `Command_completed 1L` sets `canvas_ready`; probe `Mounted`
sets `mounted`. Probe `Data` queues events for the worker. Failures raise rather
than pretending readiness. These refs and the queue communicate observations to
the single worker; they are not additional Bonsai state machines. An optional
audit instance reports failures; its actual entity/memory checks belong to the
separately linked native audit package.

## One worker opens and retires one window at a time

`run` selects one warmup plus three measured cycles for smoke, or three warmups
plus 30 measured cycles for full mode. `App.run ~exit_on_last_window:false` keeps
the process alive between closes. One application-scoped `Scope.start` owns the
worker. `perform` starts a short empty scoped task whose UI `on_result` maps a
supplied effect into an `Eio.Promise`; the worker awaits that promise. `sync`
wraps a UI thunk through this bridge. Do not await this adapter inside a UI
handler that must itself process the completion. It has no independent timeout
or closed-scope result; the explicit waits and external collector bound failure.

Each cycle creates fresh Vars, readiness refs and queue, then opens a 1200 × 800
window. `background` requests no focus; it does not guarantee visibility or a
render callback while occluded. After a window snapshot exists, its
`App.Window.scope` owns `Asset.register`, `Document.create` with an empty stream,
and `Canvas_resource.create`. Successful registrations are adopted together into
the resources Var, causing the reactive graph to expose the views.

The worker appends a canonical Markdown message containing Unicode and an OCaml
code fence, then calls `Document.finish`. A 20-second polling wait requires image
readiness, canvas acknowledgement, probe mount and `Document.is_published`, while
checking `Document.error`. It asserts exact retained source bytes and awaits a
native frame callback. Publication and that callback do not prove parser completion,
pixel correctness, GPU completion or physical presentation.

Even cycles additionally await the probe's delayed `Begun` event. Odd cycles may
close while that operation remains pending; the code does not assert that every
odd operation is still pending. This alternation exercises window-scope cancellation
alongside completed work without replacing ownership checks with timing guesses.

## What a checkpoint actually asserts

After emitting `exercised`, the worker closes the window and waits for acknowledged
retirement. `clean` requires zero windows, queued jobs/commands, pending requests,
assets/uploads/source bytes, documents/source bytes, charts/data bytes,
canvases/scene bytes and native command queue entries/bytes. The application scope
and main worker intentionally remain: their expected counts are one scope, one
task and zero cleanups. These are application diagnostics, not a census of all
native entities or allocated process memory.

It also asserts `Asset.is_released`, `Canvas_resource.is_released` and absence of
`Document.source`. It drops the model-held registrations by setting the Var to
`None`, waits two seconds and checks `clean` again. It neither forces GC nor purges
native caches. The emitted `checkpoint` pauses allocation until stdin contains
exactly `continue N` for that cycle, within 20 seconds. The collector samples the
waiting child's RSS before sending that acknowledgement. Native auditing, when
selected, adds its own matching-record requirements in the collector.

Trace: open → register in window scope → publish source and receive resource
observations → render callback → optionally await delayed probe → close → acknowledge
retirement → drop retained model handles → settled checkpoint → collector sample
and continuation. After the final cycle, `complete` precedes `App.shutdown`.

## Build, measure and adapt

From the repository root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_lifecycle/main.exe -j 2
python3 scripts/measure_resource_lifecycle.py --build-profile release --smoke --output scratch/lifecycle-smoke
python3 scripts/measure_resource_lifecycle.py --build-profile release --check-budgets --output scratch/lifecycle-full-1
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe -j 2
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --executable _build/default/examples/resource_audit/main.exe --smoke --output scratch/entity-smoke
```

Use fresh output directories and one owned GUI workload at a time. The executable
requires its collector's stdin protocol. Full RSS qualification requires three
full optimized runs and the predeclared final-ten growth gate; smoke is a protocol
check. The audit claim is no new live GPUI entity handles relative to its closed
warmup baseline, not zero global entities. Optional physical-memory and Metal
observations have separate contracts described in the driver READMEs. Source
review or Linux compilation establishes no new GUI acceptance. These commands
were reviewed, not executed for this walkthrough.

To adapt the workload, change resource fixtures and readiness predicates together,
preserve window ownership and exact checkpoint acknowledgements, and declare
measurement bounds before running. A different canvas command needs its matching
sequence check. Richer document fixtures require independent parser/render evidence;
new asynchronous work must be scoped and tested for retirement rather than merely
removed from the visible view.

The resource audit entry point can additionally pass `~presentation:true` to
`run`. This flag reaches each numbered `Gpuio_resource_audit.instance`; it does
not alter Bonsai state, render timing or the delayed performance probe. The
[native audit](../resource_audit/rust/src/presentation.md) starts its own immediate
measurement session and requires retirement before the collector permits another
window. The ordinary lifecycle entry point passes false.
