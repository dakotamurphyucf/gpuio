# Logical paged-table measurement

Read the [driver walkthrough](main.md) and [pure cache walkthrough](page_cache.md) for implementation and ownership.

This public-API workload keeps 100,000 stable logical row identities and 64
columns. Its application-owned cache retains at most four 128-row pages (512
payload rows), evicting payloads through `Table_data.set` without resetting source
lineage. Each loaded row has 64 deterministic UTF-8 strings. Logical identity and
coverage metadata remain O(rows); this is not constant total memory or a remote
I/O throughput test. The boundary-append `Table_paging` adapter is not used.

The 1168×720 table uses 32-pixel fixed rows, 64 pixels of overscan, at most 32
active rows and 2,048 active cells. All columns count toward the active-cell
budget, even though horizontal painting is virtualized. The workload visits every
row range forward and backward, verifies payload availability, preserves a cell
selection across eviction, visits all 64 column bands, and reveals a middle cell.
It then checks 60 seconds of settled idle and window-owned resource cleanup.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_table/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/performance_table
python3 scripts/test_measure_table_history.py
python3 scripts/measure_table_history.py --build-profile release --smoke --output scratch/table-smoke
python3 scripts/measure_table_history.py --build-profile release --timeout 1800 --check-budgets --output scratch/table-full-1
```

Smoke uses 1,030 rows (including a partial final page) and two seconds of idle.
Full acceptance needs explicit warm-up and three independent measured runs.
Run one owned GUI at a time and finish compilation first. `--background` requests
an unfocused window but does not establish OS occlusion or an inactive interval.
Reports separately retain asynchronous activation snapshots and observed changes
from idle start through final histogram retrieval. Those observations do not
prove OS visibility or rule out transitions coalesced before delivery.
The private native histogram probe and runner preserve exact binary hash,
checkout state, hardware/display/power metadata, CPU time, peak RSS, raw buckets
and failed reports. The checked-out revision is not embedded binary provenance;
retain the preceding build log and source revision.

The [predeclared table targets](../../docs/design/performance-qualification.md)
are draw p95 ≤16.7 ms, p99 ≤33.4 ms, at least 1,000 draw samples and peak process
RSS ≤1 GiB. Native draw/submission time is not GPU execution or physical
presentation. Initial O(rows) metadata construction precedes the histogram
interval but remains included in process CPU and peak RSS. Cache limits describe
payloads in the current application snapshot; prior snapshots and native retained
cells can survive until reconciliation/GC, so measured process memory remains a
separate required check.

The optional `--wall-clock` flag retains functional checks without mounting the
probe; it cannot supply frame-budget or zero-idle-redraw acceptance. The supplied
executable still links the profiled backend, so this flag alone is not an
ordinary-backend comparison.
