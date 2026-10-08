# Current-source paged-table presentation qualification — OCH-17

On 2026-10-08, one smoke warm-up followed by all three planned full trials
passed the unchanged CPU, presentation, memory, coverage, idle and cleanup gates.
This qualifies this workload on the reference Mac; it does not complete OCH-17.

## Source and procedure

Executable source is `2562490ffee13059a6ca99ddc7108556f2a3972a`, SHA-256
`85619583aef0ebcc0b1644d004082c0cb4ea8624945c6579dbb38c41e080f557`.
The clean observation checkout was `de452dea3f73453716e801513f2ac645e449487f`;
intervening changes were documentation only. Temporary startup/idle diagnostics
were restored before the optimized rebuild. Both build logs and binary checks
are retained with the batch runner.

Reference: Apple M1 Max, 32 GiB, macOS 14.5 arm64, internal 3456×2234 display
reporting 1728×1117 logical pixels at 120 Hz. Runs used AC power; system queries
reported no recorded thermal/performance warning. Display mode is not measured
FPS. Window traces record 1200×801 content pixels and the unchanged table fixture.
There was one owned GUI and no simultaneous local compilation. Lightweight source
inspection, scratch notes, GitHub polling/downloads and Linear updates continued;
these are potential interference, not an isolated laboratory environment.

The predeclared batch used one warm-up and exactly three full trials, retained
every outcome, and stopped on child execution failure. The 3600-second process
timeout accommodates full traversal; frame waits and performance limits did not
change. Each full command was:

```sh
python3 scripts/measure_table_history.py --build-profile release --presentation \
  --check-budgets --timeout 3600 \
  --executable _build/default/examples/performance_presented/table/main.exe \
  --output scratch/agents/root-20261007-access-check/table-2562490f-001
```

Use suffixes `002` and `003` for the subsequent trials. The warm-up substitutes
`--smoke` for `--check-budgets`. The batch and all children exited zero; its owned
keep-awake process was reaped.

## Results

| Trial | Draws | Draw p95 / p99 ms | Presented frames | Submission→presentation p95 / p99 ms | Peak RSS MiB | Wall s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 001 | 26,370 | 15.426 / 16.179 | 26,369 | 17.056 / 19.661 | 489.641 | 784.359 |
| 002 | 26,341 | 15.819 / 16.703 | 26,340 | 17.105 / 20.546 | 473.969 | 794.935 |
| 003 | 26,228 | 15.811 / 16.646 | 26,227 | 17.056 / 20.283 | 471.359 | 790.307 |

Every full run covers 100,000 rows forward, backward and materialized, all 64
columns, selection retention and reveal. Peak active state is 26 rows / 1,664
cells; loaded payloads remain bounded at 512 rows, with 1,564 page loads and
1,560 evictions. All declared window-owned cleanup counters pass.

Each run retains one initial input-free zero-time drawable. First positive
presentation occurs at 32.490, 32.700 and 31.671 ms respectively, within the
previously declared 100 ms startup-transition gate. There are no subsequent zero
outcomes, missing callbacks, invalid clocks, admission losses or pending frames.
The bounded raw trace keeps 4,096 records; truncation is explicitly counted,
while cumulative histograms cover all outcomes. This is not a full frame trace.

All three settled idle intervals exceed 60 seconds and contain zero drawn,
admitted or presented frames. All sampled window observations are active/visible;
sampling does not establish continuous visibility between observations. Every
full report has `complete=true`, child exit zero and `budget_failures=[]`.

These measurements use Metal's reported presentation timestamp, distinct from
CPU drawing, callback arrival and photon latency. This workload contains no
typing or animation sample set and establishes no 120 FPS guarantee. List,
document, streaming, instrumentation-overhead and resource-retirement acceptance
remain separate. Historical table failures and older-source results remain in
[the table investigation](paged-table-performance-och17.md).

## Retained evidence

[Compact summary](presentation-table-current-och17/summary.json) includes original
report hashes, exact histogram counts/percentiles, coverage and idle results.
[Raw bundle](presentation-table-current-och17/reports.tar.gz) contains all four
reports, application/driver logs, declared runner and original/restored build logs.
All 19 archive members were read back and verified against the
[SHA-256 manifest](presentation-table-current-och17/manifest.json).
