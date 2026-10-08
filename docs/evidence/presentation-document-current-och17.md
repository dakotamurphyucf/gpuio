# Current-source growing-document qualification — OCH-17

One smoke warm-up and all three predeclared full trials pass on 2026-10-08,
using the unchanged [performance](../design/performance-qualification.md) and
[presentation](../design/metal-presentation-qualification.md) contracts.
This is scoped document acceptance on the reference Mac, not release completion.

## Provenance and exercise

Executable source: `2562490ffee13059a6ca99ddc7108556f2a3972a`; SHA-256:
`a1fedc1659c24e0ab2ee211624444a6e4df8c611ca9e331c234fbb6b582b15c3`.
All runs observed clean checkout `5f75aec1767852f87c914b5490534d93b238fc54`;
the intervening changes were documentation only. Original/restored optimized
build logs are included. No temporary startup or idle instrumentation remained.

Environment: Apple M1 Max, 32 GiB, macOS 14.5 arm64, internal display reporting
1728×1117 logical / 3456×2234 physical pixels and a 120 Hz mode, on AC power.
Raw reports include display, power and thermal query output. No simultaneous local
compiler or second owned GUI ran. Source reads, scratch preparation and hosted-job
polls continued; this was not a fully isolated machine.

Each trial grows three independent sources to 8, 8 and 4 MiB through 160 appends
and verifies 166 native interaction checkpoints. Navigation/selection and real
Copy checks cover source prefixes, every page forward/backward, current-page
selection and the final source line. The test verifies rejection above the 8 MiB
per-source limit, 20 MiB aggregate retention, source reset, profile removal and
owned cleanup. The driver restores and verifies the original clipboard on every
trial, including the warm-up. Every child and the batch's keep-awake process
exited and were reaped.

Large Markdown uses the existing source fallback above the rich-view limit;
the driver explicitly verifies the ResourceLimit fallback notice. These results
do not claim rich Markdown parsing of 20 MiB or support for one 20 MiB document.
Raw preparation counters distinguish queueing, parsing, highlighting and repeated
source preparation from retained source bytes. Append publication and subsequent
AX-ready acknowledgement timings are also retained separately.

```sh
python3 scripts/measure_document_growth.py --build-profile release --presentation \
  --check-budgets --timeout 1200 \
  --executable _build/default/examples/performance_presented/document/main.exe \
  --output scratch/agents/root-20261007-access-check/document-2562490f-001
```

Trials `002` and `003` use their corresponding output directories. The warm-up
uses `--smoke` instead of `--check-budgets`. No failed trials were replaced.

## Results

| Trial | Draws | Draw p95 / p99 ms | Presented frames | Submission→presentation p95 / p99 ms | Peak RSS MiB | Wall s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 001 | 6,465 | 4.448 / 6.242 | 4,855 | 17.973 / 22.233 | 215.625 | 149.008 |
| 002 | 6,475 | 4.518 / 6.402 | 4,863 | 17.875 / 22.872 | 205.703 | 149.143 |
| 003 | 6,471 | 4.432 / 6.214 | 4,863 | 17.678 / 21.725 | 202.375 | 148.773 |

Each trial also pairs 3,055 native input samples with presentation samples.
Input→presentation p95/p99 are respectively 27.214/28.279, 27.247/28.377 and
27.083/27.902 ms. These document interactions are not a substitute for the
separate streaming/typing workload. Input timing excludes the preceding OS queue,
keyboard hardware and photon latency.

Every trial retains two initial input-free zero-time callbacks, with first positive
presentation at 44.249, 46.660 and 51.321 ms, within the existing 100 ms transition
bound. No later zeros, missing callbacks, invalid clocks, saturation, pending
frames or histogram overflow occur. Raw traces are bounded at 4,096 records;
761/769/769 further records are counted as truncated while cumulative histograms
retain all outcomes. All 146 periodic observations and the final observation are
active/visible in each run; that does not prove visibility between samples.

All full reports have `complete=true`, child/driver exit zero, empty
`budget_failures` and `clipboard_restored=true`. The acceptance scope is this
defined workload and source on this Mac. Collector overhead, active-collector
retirement, the outstanding list findings and broader release requirements remain.

[Summary](presentation-document-current-och17/summary.json) retains exact values
and original report hashes. [Raw bundle](presentation-document-current-och17/reports.tar.gz)
includes four reports, application/driver logs, batch plan and build logs. All
19 members were read back and checked against the
[SHA-256 manifest](presentation-document-current-och17/manifest.json). Historical
results remain in [the earlier document evidence](growing-document-och17.md).
