# Full presented-list qualification — OCH-17

The [current-source repeat batch](#current-source-repeat-batch--2026-10-07)
has two full passes and one failed idle check. Three-run acceptance remains open.
The original startup failure below is retained unchanged.

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

## Current-source repeat batch — 2026-10-07

A new optimized batch on clean `2562490ffee13059a6ca99ddc7108556f2a3972a`
completed its declared one warm-up and three full trials. **Two full trials
passed; the third failed the zero-redraw idle requirement.** No replacement
trial was launched. This does not establish three-run acceptance.

The executable SHA-256 is
`fd3d5dabeb1d73eade70207db5a48d69120a8b7370f450d3ec20593e802843c2`.
It was rebuilt alongside the other three presentation workloads with
`GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j2`.
The reference remains macOS 14.5 arm64 / Apple M1 Max / 32 GiB, with the internal
display reporting a 120 Hz mode. Mode metadata does not establish measured FPS.
No simultaneous local compiler or second owned GUI test ran. Read-only source,
report and hosted-CI inspection continued; application sources stayed unchanged.

| Trial | Result | CPU draw p95 / p99 | Submission-to-presentation p95 / p99 | First presentation | Peak RSS bytes |
| --- | --- | --- | --- | --- | ---: |
| 001 | Passed | 2.621 / 3.052 ms | 32.063 / 32.227 ms | 27.324 ms | 181,764,096 |
| 002 | Passed | 2.703 / 3.183 ms | 31.818 / 32.162 ms | 38.718 ms | 181,223,424 |
| 003 | Failed idle assertion | Not accepted as a complete run | Not accepted as a complete run | Not accepted as a complete run | 189,120,512 |

Trials 001/002 each cover all 10,000 rows forward and backward, verify row growth
and anchor behavior, and retain at most 32 active rows. They have respectively
27,762/28,595 draws and 27,761/28,594 presentations: one bounded, input-free
initial zero-time outcome each. Their declared CPU, startup, presentation, RSS,
settled 60-second idle and owned-resource cleanup checks all pass. Histograms
cover all samples; each raw frame trace is explicitly capped at 4,096 records.
Every sampled visibility/activation observation is true, which does not prove
continuous visibility between observations.

Trial 003 completes both traversals and the growth check, then records **14 CPU
draws and submissions during 60.011 seconds of idle**. The child exits 2 at the
unchanged zero-draw assertion, before normal cleanup/completion records; the
driver and batch exit 1. Its idle Metal report has seven presented and seven
zero-time outcomes, no pending callbacks, and no input samples. All 14 records
have `new_scene=true`, `animating=false` and `active=true`. They occur as seven
pairs over approximately 41.65 seconds from the first submission. All sampled
window observations remain active and visible. These observations do not identify
the redraw trigger or prove that no pointer/desktop interaction occurred.
The owner was asked about interaction during this interval; its cause remains
unverified at this checkpoint. No threshold, settling interval or workload was
changed to accept this failure. The child and batch-owned keep-awake process
were reaped; application-level cleanup is not claimed for this failed run.

The [complete batch archive](presentation-list-current-och17/reports.tar.gz)
contains the warm-up, all three original reports/application logs/driver logs,
the declared runner, build log and separately labeled offline failure inspection.
All 18 archived files were checked against the
[SHA-256 manifest](presentation-list-current-och17/manifest.json).
The [compact summary](presentation-list-current-och17/summary.json) preserves
each outcome and binds it to the original report hash; it does not fill missing
failed-run fields with passing values.

This batch follows intervening native cache/lifetime repairs and the separately
[diagnosed visibility-sensitive frame wait](list-scroll-diagnostics-och17.md).
It does not prove the cause of the historical 102.723 ms startup failure or
resolve the owner's visual-jitter report. Investigate the new idle failure before
claiming repeated list qualification. Other workloads, collector overhead,
resource validation and broader release requirements remain open.
