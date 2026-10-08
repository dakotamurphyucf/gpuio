# Preview list qualification and instrumentation comparison

2026-10-08, Apple M1 Max / 32 GiB / macOS 14.5 arm64, internal display reporting
120 Hz. **Both warmups and all six predeclared full trials pass.** The three
instrumented trials meet the existing list CPU, presentation, memory, idle and
cleanup budgets. This is workload evidence, not a universal frame-rate promise.

The ordinary and instrumented optimized binaries were built from clean
`31bffeee8752a965fd53d3e565c26c95295766ca` using:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j2 \
  examples/performance_plain/main.exe examples/performance_presented/list/main.exe
```

Cargo feature graphs verify ordinary A excludes CPU/presentation profiling and
instrumented B enables both. Both use the same OCaml workload. Binary SHA-256:

- A: `d6f2e2eb7ef46c16997ae4741f05b08f37730d32575d5ddce5a7d48ce6cc6ae2`.
- B: `9afbcbc18b8913f9f5343287a208f1b2a4802b73a1822e9dbab64e4f7d746f16`.

The fixed order was A/B smoke warmups, then **A1/B1/B2/A2/A3/B3**, stopping on
any failure. No trial was replaced. The foreground helper requests activation
once and completes all accessibility setup before timing, then performs no AX
queries or input injection during measurement. Extra input/startup diagnostics
are disabled. No concurrent GPUIO compiler or second owned GUI ran. Lightweight
source/documentation work and hosted CI polling continued; this is a desktop
measurement, not laboratory isolation.

Documentation commits and CI-only probe/classifier edits occurred after building;
each report retains its actual observed revision/dirty state. The binaries and
application/runtime sources stayed unchanged. Their hashes were checked before
each launch. The new CI classifier does not participate in these workload checks.

## Three full instrumented results

Every full trial traverses all 10,000 variable-height records forward and backward,
checks growth of the first row and its anchor, and retains at most 32 active rows.
All three instrumented trials then record **zero draws during 60 seconds of idle**.
Application registrations, pending requests and native command queues retire;
all children and the batch-owned keep-awake process are reaped.

| Trial | CPU draw p95 / p99 | Submission→presentation p95 / p99 | Peak RSS |
| --- | --- | --- | ---: |
| B1 | 3.404 / 4.227 ms | 27.361 / 31.867 ms | 178,192,384 B |
| B2 | 3.316 / 4.190 ms | 25.543 / 31.752 ms | 181,174,272 B |
| B3 | 3.277 / 4.159 ms | 31.785 / 32.145 ms | 174,555,136 B |

The trials contain 28,335 / 28,349 / 28,492 draws. Each has one initial, input-free
zero-time presentation within the existing startup allowance and valid positive
timestamps thereafter. Histograms retain all samples; raw traces retain their
declared first 4,096 records. There are no lost/invalid/pending samples or histogram
overflow. All periodic history observations report active/visible. These sampled
observations do not prove continuous visibility or hardware/photon latency.

## Ordinary versus instrumented cost

| Metric, median of three full trials | Ordinary A | Instrumented B |
| --- | ---: | ---: |
| History traversal | 240.055 s | 237.187 s |
| Whole process elapsed | 304.518 s | 303.084 s |
| Whole process CPU | 104.446 s | 121.145 s |
| Peak RSS | 159,694,848 B | 178,192,384 B |

Instrumentation adds approximately **16.0% median CPU** and **18,497,536 bytes
(17.6 MiB) median peak RSS** in this comparison. Elapsed times are similar. They
include refresh-paced frame waits, so the slightly lower instrumented median is
not a throughput improvement. CPU ranges overlap: A is 93.803–115.504 seconds;
B is 110.383–122.443 seconds. Three trials quantify this observed cost without
establishing statistical confidence or a universal overhead constant.

This measures total qualification instrumentation: CPU histograms, presentation
callbacks, observations and report export. It does not isolate callback cost.
Ordinary release applications exclude these diagnostic features. A's wall-clock
mode cannot independently certify frame latency or zero redraws; B supplies those
measurements. Both variants verify traversal, growth and application cleanup.

The [summary](preview-list-comparison-och17/summary.json) retains every trial and
min/median/max comparison. The [archive](preview-list-comparison-och17/reports.tar.gz)
contains all 35 build/feature/driver/log/report files; every member was read back
and verified against the [manifest](preview-list-comparison-och17/manifest.json).
All eight logs were independently replayed through the current workload and
presentation validators; full B trials have no budget failures.

Earlier interrupted/input-heavy/idle failures remain in the
[previous attempts](collector-overhead-och17.md). These new results do not establish
their original triggers. The traversal uses immediate row jumps and does not
simulate smooth wheel/trackpad scrolling. The separate normal-scroll assessment,
streaming/resource change-impact review, and remaining preview requirements must
not be inferred from this benchmark alone.
