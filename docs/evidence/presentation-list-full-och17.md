# Full presented-list qualification — OCH-17

First optimized run at clean `394039d`, macOS14.5 arm64, Apple M1 Max.
**Failed**: the initial transition takes102.723 ms, exceeding the predeclared
100 ms bound. This is not waived or counted as a passing repetition. The
[original artifacts](presentation-list-full-och17/run-001-artifacts.tar.gz) include
all raw records, failure traceback and report. A separate
[diagnostic summary](presentation-list-full-och17/run-001-summary.json) retains
the failure and distinguishes offline CPU-budget replay from whole-run acceptance.

The full10,000-row forward/backward traversal, first-row growth/anchor behavior,
bounded active rows and final resource retirement complete. The child exits0 and
is reaped; the collecting script exits1. No other local compiler or GUI test ran
concurrently. Release-notice manifests were inspected read-only during the run;
their three attribution records were refreshed near completion, without touching
application/build inputs. The recorded dirty state is the state at startup.

| Observation | Result |
| --- | --- |
| History duration | 222.114 seconds |
| Native attempts / presentations | 26,590 / 26,587 |
| Zero-time outcomes | Three contiguous initial frames, none with input |
| Other losses, invalid clocks, pending callbacks | Zero |
| Raw trace | First4,096 records,22,494 explicitly counted truncations; cumulative histograms cover all outcomes |
| Submission-to-presentation p95 / p99 | 32.260 /32.293 ms |
| CPU draw p95 / p99 | 2.972 /3.838 ms |
| Peak process RSS | 183,844,864 bytes |
| Settled60-second idle | Zero CPU draws and zero native attempts/presentations |

All223 history and60 idle observations report active/visible; this is sampled
visibility, not continuous proof. The CPU/content/resource budgets pass on replay,
and presented-frame p95/p99 lie within their targets. The startup bound still
fails, so neither those partial results nor the small2.723 ms excess establish
acceptance. The source of the extra delay remains unverified. Do not simply rerun
until a passing result appears or raise the limit to fit this observation.

The [optimized build logs](presentation-list-full-och17/release-build-logs.tar.gz)
record all four instrumented executables built in release mode at `05ceb3a`, then
a successful cached recheck after the documentation-only `394039d` commit. The
full report identifies the exact executable hash. Original startup failures and
the earlier native/standalone isolation remain in [idle evidence](idle-presentation-och17.md).

```sh
python3 scripts/measure_list_history.py --build-profile release --presentation \
  --check-budgets --executable _build/default/examples/performance_presented/list/main.exe \
  --output scratch/agents/root-20261004-resumed/presentation-list-full-001
```

Remaining work includes diagnosing the failed transition, repeated full workload
acceptance, presentation-collector overhead and resource qualification. VoiceOver
was not operated or configured. OCH-17 remains open.
