# Collector overhead attempts — OCH-17

The comparison is **incomplete**, not an overhead result. These attempts use clean
`9e6fea4c46e368a748c51e2b87b9d704af2f83b8` on the physical M1 Max/macOS 14.5.
Both optimized executables use the same OCaml list workload. Cargo feature graphs
verify that the ordinary backend excludes the profiler/presentation collector,
while the instrumented backend enables both. Binary hashes:

- Ordinary: `d18f909e4749abf04093f2260c49cba3deed81b6bb846c72a960e0750ea7a23c`.
- Instrumented: `fd3d5dabeb1d73eade70207db5a48d69120a8b7370f450d3ec20593e802843c2`.

The predeclared plan was one smoke warmup per variant, then A1/B1/B2/A2/A3/B3,
where A uses the ordinary backend's `--wall-clock` mode and B uses
`--presentation --check-budgets`. Full trials retain the identical 10,000-row
contents, complete forward/backward traversal, row growth, sixty-second idle and
cleanup requirements. This would compare total qualification instrumentation,
including CPU profiling, presentation collection, observation and report export;
it would not isolate callback cost. Wall-clock mode cannot qualify frame timings
or zero-redraw idle. No new overhead threshold was chosen.

## Retained attempts

The first batch's ordinary smoke passed. Its instrumented smoke exited 2 waiting
for `row 0 rendered frame`; the failure snapshot shows an inactive window, an
empty native command queue and one pending request. This is consistent with the
separately reproduced visibility-sensitive frame wait, but does not prove this
window was covered. The batch stopped before any full run.

A distinct batch added the same startup precondition to **both** variants: once
the owned window exists, request AXFrontmost/AXRaise once, verify the Boolean
foreground state, and finish all AX queries before the history begin record.
The helper fails if setup crosses that boundary. It never reactivates or queries
the window during measurement. Both smoke warmups then passed; A1 completed its
full ordinary workload and cleanup. B1 completed history, then failed the original
idle assertion before normal cleanup:

| B1 interval | Elapsed seconds | CPU draw samples | CPU input-bearing samples | Presented / zero outcomes |
| --- | ---: | ---: | ---: | ---: |
| History | 231.322 | 27,656 | 0 | 27,655 / 1 |
| Idle | 60.005 | 908 | 900 | 1,384 / 1 |

The 232 history and 60 idle observations all report active/visible. The idle
interval therefore was not untouched: the native profiler recorded input-bearing
frames. Neither comparison driver posts input during measurement. The record does
not identify who or what supplied those events. This differs from the earlier
fourteen-draw idle failure, which had no recorded input. CPU draw and presentation
counts differ because presentation can also reuse an existing scene.

B1 exited 2 after 297.049 seconds; the batch stopped, leaving B2/A2/A3/B3 unrun.
All children and owned keep-awake processes were reaped. Process reaping does not
replace the missing application cleanup marker. No concurrent compiler or second
owned GUI ran during either batch.

Do not compute a repeat overhead estimate from one passing A and an incomplete B,
or relabel these failures as passes. A new comparison needs a controlled desktop
interval; the original budgets and failed evidence remain intact.

The [archive](collector-overhead-och17/attempts.tar.gz) retains all thirty files:
both plans, exact commands/helpers, build and feature logs, every raw application
log and report. All members were read back and hash-verified against the
[manifest](collector-overhead-och17/attempts-manifest.json).
The [summary](collector-overhead-och17/summary.json) records exact results,
binary/report hashes and partial interval counts without claiming acceptance.

The [opt-in foreground input follow-up](foreground-input-diagnostic-och17.md)
adds event-kind and lost-entry counts for diagnosis. Its quiet and injected-input
smokes validate instrumentation only; they do not complete this comparison.

The subsequent [full-workload diagnostic](foreground-input-diagnostic-och17.md#full-workload-diagnostic-follow-up--2026-10-08)
completes 10,000 rows and sixty-second idle with zero idle draws, inputs and lost
journal entries. History journal entries are overwritten and reported separately.
The earlier input-heavy result is therefore not inevitable, but its producer is
still unidentified. Diagnostic instrumentation remains enabled, so this is not
an additional paired trial or completed overhead estimate.
