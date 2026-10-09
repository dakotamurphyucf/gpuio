# Paged-table qualification driver

[main.ml](main.ml) has no separate interface. Read the [README](README.md),
[pure cache](page_cache.md) and
[qualification contract](../../docs/design/performance-qualification.md). This executable
automates native traversal/measurement; it is not an interactive application template.

`App`/`Scope` alias Gpuio_eio services, `B` Bonsai.Cont, `E` Bonsai.Effect, `V`
Gpuio_bonsai.View, `W` its Table adapter, `T`/`D` core Table/Table_data, `Column` Table_column
and `Cache` Page_cache. The graph hosts reactive table/source/command computations. Expert.Var
exposes current source and probe-command values; `let%arr` derives cell/view descriptions when
these values change. Effect construction does not execute a scroll or mutate a Var.

`config` declares 64 width-128 columns, column 0 left-pinned, row height 32, overscan 64 and
budgets 32 active rows/2,048 cells. `component` renders a 1168×720 managed table; absent cached
payload displays Loading. `render_row_presentation` uses after_display to record materialized
row numbers, and another after_display stores latest adapter Output. These hooks are
bookkeeping, not proof of physical display or a synchronous native-layout callback. Probe
extension events queue native measurement replies; failure raises, and the queue asserts one
outstanding reply.

`run` uses 100,000 logical rows/60-second idle, or --smoke uses 1,030/two seconds. App owns
window/scope; --background requests no initial focus. Worker state holds cache, observations,
coverage sets, peaks and probe sequence. `perform` schedules a no-op scoped task whose UI-domain
completion executes an effect and resolves an Eio promise, separating worker orchestration from
Bonsai mutation.

For a move, `preload` computes a new deterministic cache snapshot, then its effect updates
source; reactive derivation publishes payloads. Controller.scroll_to executes against a captured
row reference. Native viewport observations update adapter Output; worker waits for target
anchor/materialization, requests a frame acknowledgement, waits for matching viewport and checks
budgets/payload availability. A frame acknowledgement alone does not ensure an asynchronous
viewport sample or physical presentation. Worker waits have explicit timeouts; no worker mutates
Bonsai directly.

The history phase selects cell (0,0), traverses every row range forward/backward, checks
coverage and selection preservation across eviction, visits all column bands, then reveals
middle-row column 63. The probe Begin/Finish/Buckets protocol validates five metric counts and
strictly ordered bounded histogram pages, preserving raw buckets. --wall-clock omits probe
mounting and measures elapsed phase time; it cannot establish frame budgets or zero-idle-redraw
acceptance and still links the profiled backend.

After a two-second settling period, idle captures asynchronous activation state/changes, waits
and asserts zero newly drawn application frames when probe results exist. Background/active
snapshots are not OS occlusion evidence. Cleanup closes window, waits for zero windows, emits
diagnostics/completion and shuts down App. Failure can raise before this normal path; the runner
must retain failed reports. Cache generation is synthetic, not disk/network throughput;
draw/submission timing is not GPU execution or physical presentation.

## Commands and limits

From repository root, build before an owned measurement run:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_table/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/performance_table
python3 scripts/measure_table_history.py --build-profile release --smoke --output scratch/table-smoke
```

These commands were not executed for this documentation change. The runner records metadata/raw
results; read README for full warm-up, three measured runs and budget checks. There is no
--self-test flag; --smoke is a smaller automated workload. Adapt functional orchestration
separately from instrumentation, retain exact binary/build provenance and distinguish bounded
active/payload rows from total logical metadata/process RSS. Reusing this driver’s loops does
not by itself qualify a platform or establish a passed measurement.
