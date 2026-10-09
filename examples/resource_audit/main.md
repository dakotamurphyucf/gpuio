# How `main.ml` selects the resource qualification workload

[README](README.md) · [Source](main.ml)
· [OCaml audit instance](ocaml/gpuio_resource_audit.md)

This small entry point parses flags and delegates to `Gpuio_lifecycle_workload.run`.
It creates no Bonsai graph, window, collector, or memory measurement itself.
`--smoke` selects one warmup plus three measured cycles; full mode selects three
warmups plus 30 measured cycles. `--background` requests background opening.
`--metal-memory` requires device samples. `--presentation` requires immediate
active collector retirement records. `native_entities:true` is always enabled
for this executable because its Dune stanza selects the separate audit backend.

The [shared workload](../lifecycle_workload/gpuio_lifecycle_workload.ml) owns
`App.run`, the Eio task, sequential 1200 × 800 windows, and window-scoped image,
streamed Markdown, canvas, and asynchronous probe resources. Its `let%arr` binds
external Bonsai Vars to derive views; callbacks return effects that record
readiness. Native decode/preparation/layout remain native. Some cycles finish
probe work, while others close with delayed work potentially pending.

Each window gets a numbered audit instance. Closing cancels resource scopes;
the workload waits for empty public registration/request queues, drops model-held
registrations, settles two seconds, and writes its application checkpoint.
Separately, Rust's app-owned audit checks closed native entities after one second.
The external collector validates both records and sends `continue <cycle>` on
stdin. Without it, a direct launch waits for that acknowledgement and times out;
this is not a standalone interactive demo. Requested Metal records add a third
required checkpoint before continuation; presentation retirement adds another.
See [the native implementation](rust/src/presentation.md).

From the repository root in the [repository environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --executable _build/default/examples/resource_audit/main.exe --smoke --output scratch/entity-smoke-review
```

Use a new output directory. The collector uses the built binary, records its hash,
environment and log/report, owns timeout/child cleanup, and supplies the handshake.
Default collector timeout is 600 seconds. Native graphical setup is required;
background frames may be deferred. Full `--check-budgets` requires release build
and full cycles; smoke cannot establish full qualification. The README contains
full and optional macOS Metal/physical-memory commands.

Public source/queue charges, GPUI live entity handles, sampled Metal allocations,
process RSS, and physical footprint are different observations. None is a complete
GPU/Objective-C/Arc ownership census or physical presentation measurement. This
backend enables leak-detection instrumentation, so its timings are not ordinary
responsiveness evidence. Do not add it to a production backend to collect metrics
silently; keep qualification separate and claims tied to actual runs/platforms.
