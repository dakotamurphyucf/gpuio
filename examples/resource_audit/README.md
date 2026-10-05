# Native entity retention qualification

This separate statically composed backend enables GPUI `leak-detection` only for
this audit. Do not use its timings as ordinary responsiveness evidence. It uses
the same OCaml lifecycle workload as `performance_lifecycle`: images, Markdown,
canvas and asynchronous extension work, alternating settled and interrupted
operations. No production API or pinned GPUI source patch is needed.

The qualification component registers one numbered window at a time. An
application-owned subscription waits one second after close, requires no open
native windows, takes a baseline after the final warmup, and checks each later
closed window for new live GPUI entity handles. The baseline holds IDs, not
entities. The app-owned task holds no strong window/entity handles. It reports
through bounded qualification stdout records, never a retired extension event
route or synchronous OCaml callback. The final checkpoint also requires an audit
completion marker; the subscription is cancelled then and completed task storage
is released with the application. Panic containment occurs inside the global
lease so an assertion failure cannot corrupt global storage.

The collector withholds the next numbered continuation until both application
resource retirement and the matching native audit pass. Missing, failed,
reordered or duplicate records reject the run. Native failure remains visible in
the captured panic report and process record. The deliberate retained-entity
unit fixture must fail; releasing it must make the same baseline check pass.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/resource_audit/rust/Cargo.toml --locked --lib
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe
python3 scripts/test_measure_resource_lifecycle.py
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --executable _build/default/examples/resource_audit/main.exe --smoke --output scratch/entity-smoke
python3 scripts/measure_resource_lifecycle.py --build-profile release --native-entities --executable _build/default/examples/resource_audit/main.exe --check-budgets --output scratch/entity-full
```

Full qualification is three warmups plus 30 measured closes, repeated three
times with separate smoke warmups. A local four-close smoke and the shared ordinary-backend smoke pass; full
repeated runs are pending. Linux pure/build
coverage does not establish real desktop acceptance. The claim is specifically
**no new live GPUI entity handles relative to the closed warmup baseline**;
it is not zero global entities, nor all Arc/Objective-C/GPU resource ownership.
Keep the separate [physical-memory audit](../../docs/evidence/physical-memory-och17.md).
