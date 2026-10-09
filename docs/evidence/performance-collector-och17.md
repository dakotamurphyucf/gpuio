# Optional native performance collector — OCH-17

Implemented after `ce8bb76` on macOS 14.5 arm64. The `performance-diagnostics`
Cargo feature exposes `gpuio_native::performance`; default builds exclude it.
The module samples the pinned GPUI's cumulative window histograms on demand. It
adds no host loop, timer, redraw, file output, global trace activation or synchronous
OCaml callback. Native snapshots own bucket data and a window ID, not a window
handle. Capture overhead remains to be measured.

`Snapshot::since` subtracts each cumulative bucket count and the dropped-input
counter. It rejects different windows, reversed time and any regressed bucket,
even when total samples increased. Intervals expose raw ascending buckets,
nearest-rank percentiles, counts and elapsed time. Empty distributions yield no
percentile. Upstream three-significant-digit HDR upper bounds retain quantization;
durations are nanoseconds, while inputs-per-frame buckets count events. Native
input→frame and dirty→submission are not hardware latency or physical presentation.

Local validation, all exit zero:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 \
  -p gpuio-native --features performance-diagnostics,native-image-tests,native-canvas-tests \
  --lib performance::tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 \
  -p gpuio-native --features performance-diagnostics,native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked --offline -j2 \
  -p gpuio-native --features performance-diagnostics,native-image-tests,native-canvas-tests \
  --lib --tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo check --locked --offline -j2 -p gpuio-native
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
```

The focused suite passes five tests. Four cover interval mathematics/error cases;
the fifth captures a real GPUI Window using TestPlatform, observes no new samples
from repeated captures, records exactly one explicitly drawn frame, and closes
the window with snapshots still owned. The full feature-enabled library suite
reports **925 passed, two existing macOS private-bus skips**. No desktop window
was opened. Strict lint, default-feature compilation, formatting and workflow
YAML parsing pass. CI adds required no-display collector tests/lint on both OSes;
that new step has not run on the hosted source yet.

Logs: `performance-collector-00{1,2}.log`, `performance-native-suite-001.log`,
`performance-clippy-001.log`, `performance-default-check-001.log` and
`performance-format-check-001.log` under the local session directory
`scratch/agents/root-20261004-resumed/`.

## Dependency and notice evidence

No GPUI source/pin changes. Enabling its existing optional profiler adds locked
`hdrhistogram` 7.6.0. Its full MIT and Apache-2.0 license texts are collected.
The first supplemental collection correctly rejected the changed first-party
native Cargo manifest hash; after reviewing the added feature and unchanged
Apache workspace inheritance, only that package's declaration hash was updated.
The root license bytes and all third-party declarations remain unchanged.

Fresh profiled native collection, including `third_party/notice-sources.json`,
reports 514 packages, 868 collected-file hashes verified, and the same 27 existing
missing-text packages. Earlier no-supplemental and stale-declaration attempts are
retained separately. This is collection, not distribution/license approval.
Reproduction:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec python3 scripts/collect_rust_notices.py \
  --target aarch64-apple-darwin --root gpuio-native \
  --features gpuio-native/performance-diagnostics \
  --supplemental third_party/notice-sources.json \
  --output scratch/fresh-profiled-native-notices
```

See [the qualification plan](../design/performance-qualification.md). Optimized
workload drivers, actual input/presentation evidence, sampling-overhead comparison,
resource/idle budgets and release acceptance remain unfinished. Passing collector
math or TestPlatform checks cannot establish those outcomes.

## Loaded-history driver and functional smoke — 2026-10-04

The private static probe and [public-list workload](../../examples/performance/README.md)
now connect snapshot intervals to bounded OCaml event pages. Begin waits two
seconds without notifying the view; Finish captures before its own redraw, and
raw histogram retrieval is outside the measured interval. Unmount cancels the
settle task. The normal backend is unchanged.

Local debug smoke `performance-smoke-007` traversed all 96 rows both ways, kept
at most 32 active computations, grew row zero to 820 logical pixels (observed
through external macOS AX), retrieved 307 native draw samples, recorded zero
draws in two seconds of idle and retired all window-owned resources. Earlier
failures are retained: absent explicit colors produced a black-looking test
window; twelve added lines could legitimately preserve the last visible row;
the first Rust inline-record sexp had an extra list level. The corrected fixture
uses explicit colors/sizing and forty lines, and independent OCaml codec examples
check the event shapes. AX inspection must descend into the list's outline.

The paired Dune release build completed with an optimized native archive in
`performance-release-build-001.log`. The first release smoke passes the strict
Python report validator (`performance-release-smoke-001/report.json`): peak RSS
123,322,368 bytes, approximately 10 seconds total including settles and idle.
This is functional smoke, not the 10,000-row performance budget result.

Checks pass: the OCaml codec expect test, native bounded-command test, strict
probe Clippy, six report-parser tests and ten shared child-lifecycle/report tests.
The report validator rejects missing/truncated/duplicate pages, inconsistent
counts, incomplete traversal, failed growth anchoring, idle work and incomplete
resource cleanup. The required CI includes the portable report tests. Exact
commands are in the workload README; raw local artifacts are under
`scratch/agents/root-20261004-resumed/`.

Full optimized runs, repeated-run/overhead comparison and the remaining workloads
are still required. Snapshot timestamps are native submission measurements, not
physical presentation or GPU execution timings.

### First full optimized loaded-list run

Run `performance-release-full-001` passes its predeclared workload budgets on
Apple M1 Max, 32 GiB, macOS 14.5 arm64, AC power. Source at process launch was
clean `c0d2eaf321f3d07af0231278bf8ed42b56a8152f`; executable SHA-256
`2112317d1393bf4e2a520590d712d05c477222eb125d5ec9930728839aed87d3`.
No other GPUIO GUI or compilation ran during measurement. The report retains
display/power metadata; ordinary owner desktop activity was not controlled.

| Observation | Result |
| --- | --- |
| Coverage | 10,000 rows forward and backward; growth/anchor checks pass |
| Peak active row computations | 32 |
| Native draw samples | 26,881 |
| Native draw p95 / p99 | 3.125247 ms / 3.743743 ms |
| Peak process RSS | 184,483,840 bytes |
| History interval / whole process | 224.475 s / 290.102 s |
| Settled idle | 60.006 s, zero native draws |
| Dropped input timestamps | 0; this workload does not synthesize typing |
| Window-owned resources / pending work at close | Zero |

History snapshot capture took 0.130 ms before and 0.548 ms after. These capture
costs are reported separately; they do not measure the profiler's per-frame
overhead. Exact command: `python3 scripts/measure_list_history.py --output
scratch/agents/root-20261004-resumed/performance-release-full-001 --build-profile
release --check-budgets`. Raw buckets, process usage and cleanup counters remain
in that directory. This is one passing workload run, not the required repeated
matrix, unprofiled comparison, physical-presentation or full release acceptance.

## Repeated loaded-list baseline — 2026-10-04

All three independent full runs of the preserved optimized `c0d2eaf` executable
pass the predeclared loaded-list budgets and functional/resource checks. The
96-row smoke warm-up also passed before runs 002/003. No local GPUIO compilation
or second GUI ran during these measurements; ordinary desktop activity was not
experimentally controlled. Small source/documentation edits and portable tests
continued outside the measured application.

| Run | Draw samples | Draw p95 | Draw p99 | Peak RSS | History duration | Draws during ≥60s idle |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 001 | 26,881 | 3.125247 ms | 3.743743 ms | 184,483,840 B | 224.474568 s | 0 |
| 002 | 27,256 | 2.848767 ms | 3.495935 ms | 177,602,560 B | 227.254535 s | 0 |
| 003 | 26,313 | 2.949119 ms | 3.756031 ms | 177,127,424 B | 219.402511 s | 0 |

Every run visits all 10,000 records in both directions, materializes every key,
retains at most 32 active rows, preserves the first-row anchor during growth,
reports no dropped input timestamps, and retires the specified window-owned
resources and queued work. These runs do not generate typing. They requested a
focused window; the protocol did not record native activation throughout the idle
interval, so this does not complete the separate focused/unfocused idle matrix.

The executable SHA-256 remains
`2112317d1393bf4e2a520590d712d05c477222eb125d5ec9930728839aed87d3` and its
source/build provenance remains clean `c0d2eaf321f3d07af0231278bf8ed42b56a8152f`.
The runner's `revision`/`dirty` fields describe the checkout at launch, not an
embedded build identity: runs 002/003 report `c28cb8e` with false/true dirty state
respectively while using this exact preserved binary. Later benchmark harness
edits did not alter the running executable. Keep that distinction when comparing
reports. Raw reports/logs are `performance-release-full-00{1,2,3}/`; the binary is
`performance-c0d2eaf-release.exe` in the session scratch directory.

Repetition now establishes this baseline's consistency; it does not complete
profiler-overhead, table/document/streaming/resource-cycle, physical presentation
or release qualification. The [ordinary-backend comparison](../../examples/performance_plain/README.md)
uses identical OCaml work in explicit wall-clock mode without fabricated frame
histograms. Its implementation/build/measurement evidence is tracked separately.

## Paired profiler comparison implementation

The shared OCaml workload now supports explicit `--wall-clock` measurement. Both
backends run identical list traversal/growth/frame acknowledgments and cleanup;
Eio's monotonic clock records phase intervals. Native histogram mode remains
separate. Wall-clock reports cannot pass frame-budget checks or claim zero idle
draws, and seven portable report tests reject mixed modes, truncated phases,
incomplete traversal, too-short idle and retained resources.

A fresh paired release build and formatting check pass. The first combined build
reported only a missing blank line in the new Dune file; after reviewing/fixing
that whitespace, the exact command passes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 --profile release \
  examples/performance/main.exe examples/performance_plain/main.exe @fmt
python3 scripts/test_measure_list_history.py
```

The profiled executable SHA-256 is
`3bcbb83e85acf2a8884d54e6ba32129a12635f1ef6002037a4f47ced0f4adabc`;
the ordinary-backend executable is
`d0c56c8f6884458e15d46a1ac6206dcf1d3981008f15a3cda63455dfecb571b8`.
Locked Cargo feature graphs show the optional `gpui/profiler` feature only in the
composed backend. An independent symbol check finds 45 HDR histogram / 132 journal
symbols in that executable and zero of either in the ordinary executable.
A broader initial assertion that *all* symbols containing `profiler` disappear
was incorrect: GPUI includes task-timing types/helpers in that module even with
the optional feature disabled. This comparison concerns the optional feature.

Both wall-clock smoke runs pass full 96-row coverage, growth and cleanup. The
histogram smoke rerun also passes zero draws during its two-second idle. Its first
attempt recorded one idle draw and correctly failed; the raw failed report remains
retained. No source change preceded the passing rerun, and activation/desktop
interference was not recorded, so the cause remains unproven. Neither run replaces
the three full baseline runs or establishes the focused/unfocused idle matrix.

Artifacts: `performance-paired-build-00{1,2}.log`,
`performance-{default,profiled}-features-001.txt`,
`performance-paired-symbols-00{1,2}.json`,
`performance-paired-histogram-smoke-00{1,2}/`,
`performance-{profiled,plain}-wall-smoke-001/` in the session scratch directory.
## Completed ordinary-backend comparison — 2026-10-04

All six full wall-clock runs pass traversal, growth, the 60-second idle duration
and resource cleanup. Order was profiled1 / ordinary1 / ordinary2 / profiled2 /
profiled3 / ordinary3 after the paired smoke warm-ups. Both preserved binaries
come from `28d56ab5270a4411eebe06c0c13c4c7f896ab236`, with the hashes above.
Checkout revision/dirty metadata varies as independent source/documentation work
continued; no executable changed, local compilation or second owned GUI overlapped.
The parser refactoring during the last run reproduces every preceding report
exactly from its raw log. Ordinary desktop activity was not controlled.

| Build / run | History elapsed | Whole-process CPU (user + system) | Peak RSS |
| --- | ---: | ---: | ---: |
| Profiled 1 | 216.453105 s | 102.313716 s | 188,530,688 B |
| Ordinary 1 | 225.006434 s | 102.469383 s | 174,096,384 B |
| Ordinary 2 | 228.260017 s | 107.712212 s | 181,469,184 B |
| Profiled 2 | 225.086427 s | 105.469419 s | 184,369,152 B |
| Profiled 3 | 229.105415 s | 105.135304 s | 178,536,448 B |
| Ordinary 3 | 227.320620 s | 99.062042 s | 182,386,688 B |

Profiled versus ordinary medians are: history 225.086427 versus 227.320620 seconds
(−0.98%); whole-process CPU 105.135304 versus 102.469383 seconds (+2.60%); peak RSS
184,369,152 versus 181,469,184 bytes (+1.60%, 2,899,968 bytes). CPU/RSS include
startup, idle and teardown, while history elapsed excludes those phases.

These observations suggest modest optional-profiler cost in this loaded-list
workload. Overlapping ranges and three observations per build do not establish a
statistical bound, a speedup, or a guarantee for other workloads. No retrospective
pass threshold is imposed on this comparison. Wall-clock mode does not capture
histograms or establish zero idle draws. The native snapshot capture costs above
are separate from the optional feature's runtime overhead.

Raw logs/reports: `performance-{profiled,plain}-wall-full-00{1,2,3}/`; aggregate
`performance-overhead-summary-001.json`; exact order/process log
`performance-overhead-runs-001.log`, all in the session scratch directory. Both
preserved executable hashes were rechecked after completion. This completes the
initial loaded-list feature comparison, not the remaining workload matrix or
physical/GPU qualification.
