# Concurrent streaming and native typing — OCH-17

Status: all three full optimized runs pass the declared workload and budgets.
This is scoped performance evidence, not whole-milestone acceptance.

Source `f245b71d4b739cf807326fd3e42399b1e1ca049e`; executable SHA-256
`7244543c63a224a08b06e74dc9b7e99e8dc9b3ef57a4112e3eabaf76bcae86b3`.
Apple M1 Max, 32 GiB, macOS 14.5 arm64, built-in 120 Hz mode, one visible
1200×800 logical window at a time. No simultaneous local compilation or other
owned GUI. Read-only source review, CI queries, Linear updates and scratch notes
continued. Power/thermal and display metadata are retained per report; the mode
is not measured FPS.

Four independent Eio producers emit 2,400 fragments each at absolute 50 ms
deadlines, 128 UTF-8 bytes each, for 1,228,800 retained bytes in 600 history
blocks. A native multiline composer receives 1,200 physical key-code events at
10 Hz. External AX text and controller snapshots must match the canonical string;
every retained history block is validated and the four final blocks are checked
externally after timing. Source selection is restored without enabling/disabling
layouts. Owned resources retire after window close.

The initial smoke preserved all text but collected only one input-latency sample.
macOS direct text callbacks bypassed the original platform-key profiler scope.
The optional profiler adaptation measures direct invalidating text callbacks,
ignores no-ops and assigns nested callbacks to the existing keyboard scope. It
requests no redraw and does not alter text behavior. A repaired smoke collected
40 samples for 40 keys; the original incomplete measurement is not acceptance.

See the [predeclared plan](../design/performance-qualification.md) and
[driver README](../../examples/performance_streaming/README.md) for budgets,
pacing and segmentation.
These are CPU/native submission histograms, not hardware-event queue latency,
GPU completion or physical display presentation. RSS is not physical footprint.
This segmented plain-text history does not claim rich Markdown parsing performance.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_streaming/main.exe
python3 scripts/measure_streaming_typing.py --build-profile release --executable PRESERVED_EXECUTABLE --smoke --output FRESH_WARMUP_DIRECTORY
python3 scripts/measure_streaming_typing.py --build-profile release --executable PRESERVED_EXECUTABLE --check-budgets --timeout 300 --output FRESH_FULL_DIRECTORY
```

A separate four-second warm-up precedes each full run. Preserve all attempts and
raw histograms/update timings/key-dispatch records; never subtract percentiles.

Local checks: immutable model expect test, four collector rejection tests,
13 isolated pinned-GPUI profiler tests, 928 native tests (two existing platform
skips), strict native Clippy, default-feature native build and formatting pass.
The cumulative GPUI patch reconstructs all 156 files exactly. These do not close
remaining accessibility, GPU, distribution or whole-milestone requirements.

## Three full runs — 2026-10-05

Each observation checkout was clean at the recorded source. All three smoke
warm-ups and all three full attempts passed; none was discarded. The machine
was on battery with no recorded thermal/performance warning. Each run records
1,200 native input-frame samples and zero dropped timestamps. Independent text
callbacks can coalesce into one submitted frame; raw event-count buckets are
retained. Peak active history rows were 10 against the configured cap of 32.

| Run | Draw samples | Draw p95 / p99 ms | Input p95 / p99 ms | Peak RSS bytes | Wall seconds |
| --- | ---: | ---: | ---: | ---: | ---: |
| [001](streaming-typing-run-001-och17.json) | 5,426 | 4.628479 / 6.299647 | 9.101311 / 10.428415 | 128,499,712 | 123.474 |
| [002](streaming-typing-run-002-och17.json) | 6,075 | 4.792319 / 5.951487 | 8.937471 / 10.125311 | 134,463,488 | 123.466 |
| [003](streaming-typing-run-003-och17.json) | 5,069 | 4.747263 / 6.070271 | 7.102463 / 8.560639 | 138,018,816 | 123.417 |

Maximum draw buckets were 8.200191, 7.639039 and 8.658943 ms; maximum input
buckets were 11.747327, 11.485183 and 11.853823 ms. The slowest actual UI
update was 13.276 ms past its deadline, below the predeclared 100 ms guard.
The slowest key dispatch was 7.123 ms late, below its 50 ms guard. Actual
timestamps for every update and dispatched key are preserved in the raw reports.
Application user/system CPU seconds were 17.569032/2.661498,
20.174623/3.091600 and 19.814183/2.911190. The external driver's 5 ms polling
overhead is outside those application CPU/RSS totals.

Input selection was US before and during each run and restored successfully.
All owned registry/scope/queue teardown checks passed. The collector reaped its
children, all benchmark windows closed, and the batch-owned keep-awake process
was released. No native-entity or complete GPU-allocation census is inferred.

Local smoke logs, warm-up reports and the preserved binary remain under
`scratch/agents/root-20261004-resumed/`; they are historical artifacts, not build
inputs. Full report JSON above is versioned so the acceptance evidence does not
depend on scratch access.
