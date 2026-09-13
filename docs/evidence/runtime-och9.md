# OCH-9 local runtime measurements

2026-09-13, Apple M1 Max, macOS 14.5, stock OCaml 5.3.0, development build.
Commands: `./scripts/gpuio build examples/runtime/bench.exe`, then
`_build/default/examples/runtime/bench.exe N [--clock] [--hz=5]`.
Each window renders 50 static rows; clock mode adds a header using ordinary
`Bonsai.Clock.approx_now` every second. Warm-up includes initial accepted commits,
a requested render, and 250 ms settling. Each sample measures about three seconds.

| Windows | Mode | Target Hz | CPU seconds | CPU % of one core | Allocated bytes | UI turns | Clock ticks | Native commits | Render observations |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | Static | 60.0 | 0.043199 | 1.439 | 784464 | 180 | 180 | 0 | 0 |
| 4 | Static | 60.0 | 0.045440 | 1.514 | 1831104 | 180 | 181 | 0 | 0 |
| 1 | One-second clock | 60.0 | 0.048152 | 1.605 | 834792 | 187 | 180 | 3 | 3 |
| 4 | One-second clock | 60.0 | 0.059633 | 1.987 | 2077872 | 197 | 181 | 12 | 12 |
| 1 | Static | 5.0 | 0.025382 | 0.846 | 66416 | 15 | 15 | 0 | 0 |

Approximate static OCaml allocation rates: 261 KB/s for one window at 60 Hz,
610 KB/s for four windows at 60 Hz, and 22 KB/s for one window at 5 Hz.
CPU is whole-process `Sys.time`, including Rust/native work; allocation counts
are OCaml GC allocation counters. Native observations can be coalesced and do
not measure physical presentation. Tick counts are actual, not assumed from
cadence. OS scheduling and other development activity affected these short runs.
These samples are not battery, energy or release-build benchmarks.

The 60 Hz starting default offers roughly 16.7 ms timer granularity; 5 Hz gives
200 ms granularity. Input and task wakeups remain independent at either cadence.
The native self-test deliberately uses 5 Hz to make that distinction observable;
its log reports frame-response, task-completion delivery and ordinary Bonsai timer
lateness. The deterministic scope test proves completion wakes without any timer.

The result justifies retaining the accepted configurable 60 Hz starting target,
with a measurable idle cost and no static native commits/redraw observations.
Larger application graphs and direct `Clock.now` consumers need application-level
profiling. No battery guarantee or universal latency budget is inferred.

## Latency and dependency observations

At 5 Hz, the final native self-test measured 0.049 ms task-result delivery and
3.074 ms requested-frame response. Three ordinary 50 ms Bonsai sleeps completed
5.77, 5.85 and 93.68 ms after their requested interval. These are short samples,
not percentiles or an OS input-to-photon measurement. Additional input/task wakes
can advance clocks between periodic ticks.

`examples/runtime/bench_input.exe` injects 100 generation/revision-checked Press
events directly into the OCaml driver and measures dispatch through reconciliation
and simulated acceptance while cycling one or four window drivers. With one
window: median 0.003 ms, p95 0.004 ms, max 0.965 ms. With four: median 0.004 ms,
p95 0.006 ms, max 0.039 ms. This isolates OCaml input processing; it excludes OS
input, the wake pipe, native transport and painting. Actual native keyboard/mouse
behavior is covered by the OCH-8 native suite, without claiming those timings
measure physical input-to-presentation latency.

`nm _build/default/examples/runtime/main.exe` contains no `camlAsync` symbols.
The native runner links Eio, Bonsai and the existing pure/native libraries; it
adds no Async package dependency. Both native shutdown scenarios return cleanly.
