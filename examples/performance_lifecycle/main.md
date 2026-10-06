# Select the ordinary resource lifecycle workload

[main.ml](main.ml) is a ten-line launch adapter. `flag name` checks whether an exact
argument occurs in `Sys.get_argv ()`; the top-level call invokes
`Gpuio_lifecycle_workload.run` with `--smoke` and `--background` Booleans, and fixes
`native_entities:false` and `metal_memory:false`. There is no Bonsai graph, native
resource registration or measurement implementation in this file.

The executable links the shared workload and performance backend through
[Dune](dune). Read the [shared interface](../lifecycle_workload/gpuio_lifecycle_workload.mli)
and [implementation walkthrough](../lifecycle_workload/gpuio_lifecycle_workload.md)
for the owning computation. Native entity/Metal auditing requires the separate
statically composed resource-audit executable; passing collector flags does not
turn this adapter into that executable.

The shared code opens one window per cycle and owns image, streamed document and
canvas registrations in that window's scope. `B = Bonsai.Cont` and reactive Vars
hold optional resource handles and a sequenced probe command; `let%arr` derives
borrowed view configs when they change. `E = Bonsai.Effect` describes callbacks and
`V = Gpuio_bonsai.View` creates descriptions. Constructing an effect does not run
it. A native image `Ready` event executes its effect to set a readiness ref; the
worker then proceeds once document publication, canvas acknowledgement and probe
mount also hold. This is a worker-driven resource workload, not an interaction
state machine in the thin launcher.

The unique 256 × 256 PNM fixture exercises decode; document bytes and canvas
commands are checked before an acknowledged frame. Even cycles await the probe's
delayed operation; odd cycles may close with it pending. Close cancels the window
scope and waits for diagnostics to show acknowledged retirement, then drops
model-held handles and settles for two seconds. No forced GC or cache purge makes
the baseline artificially smaller. One root scope and one worker remain expected.

Each numbered checkpoint waits for `continue N` on stdin. The
[collector](../../scripts/measure_resource_lifecycle.py) samples the child's RSS
while allocation is paused, then acknowledges it. Smoke selects one warmup plus
three measured cycles; full mode selects three warmups plus 30 measured cycles.
The final-ten baseline growth gate is an increase of at most 64 MiB relative to
the first baseline of that final-ten window, not a requirement that process memory
becomes zero.
Application retirement diagnostics are not a census of all native entity handles.

From the repository root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_lifecycle/main.exe -j 2
python3 scripts/measure_resource_lifecycle.py --build-profile release --smoke --output scratch/lifecycle-smoke
python3 scripts/measure_resource_lifecycle.py --build-profile release --check-budgets --output scratch/lifecycle-full-1
```

Use fresh output directories and the collector, rather than launching alone and
waiting at the first checkpoint. `--physical-memory` and `--check-closed-surfaces`
are collector options for separate macOS evidence, not launcher switches or a
replacement for RSS/latency evidence. [README](README.md) explains their bounds
and the three full optimized runs required for qualification. Background mode
requests no focus; it does not prove exposure or foreground input behavior.
Commands were reviewed, not run for this guide; there is no `--self-test` mode.

Adapt resources and their readiness/retirement predicates in the shared workload,
then update collector expectations. Keep this launcher explicit about which
backend audits are enabled so ordinary diagnostics cannot imply stronger ownership
or Metal-memory acceptance than they actually measure.
