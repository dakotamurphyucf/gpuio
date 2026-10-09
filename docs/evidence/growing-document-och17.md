# Growing-document qualification — OCH-17

Status: three full optimized runs pass the declared document workload and budgets.
This is not full accessibility, physical-presentation or milestone acceptance.

The workload and targets are declared in the
[performance plan](../design/performance-qualification.md). Three deterministic
sources grow to 8, 8 and 4 MiB, respecting the accepted 8 MiB per-source cap.
Markdown above 64 KiB deliberately uses source fallback. See the
[driver contract](../../examples/performance_document/README.md) for phase,
copying, page traversal and measurement boundaries.

## Accessibility repair and preflight

On the reference Apple M1 Max / macOS 14.5 arm64 desktop, the first viewport
preflight incorrectly requested the Flow-only preview callback. Removing that
unsupported request preserved the production API contract. The next two runs
found visible source-page bounds and fallback notices absent from the native AX
tree. A screenshot and bounded AX capture confirmed this independently of
document-worker readiness. Identified Label nodes now expose both notices.
Production-tree Markdown and HTML regressions verify exact labels and their
update after a native accessibility page-navigation action.

The fourth preflight reached the first installed append but found that the source
view's AX node exposes value/focus without selection ranges. This remains an
open accessibility requirement. The benchmark now checks physical selection
through native Copy, change counters and independently generated canonical
bytes. This does not repair or qualify selection-range accessibility.

The fifth preflight passed on the development build: five 128 KiB appends across
three sources, eight expanding selections per append, complete forward/backward
source-page traversal, page-local selection copy, final-line navigation/copy,
whole-source copying, reset, profile removal and all owned application counters
retired. The original clipboard was restored. Its 244 draw samples and 640 KiB
aggregate source size are smoke coverage, not the full performance workload.

The executable was built from the uncommitted document-workload/AX repair on
`b972c6b9cbf74c80a337786dc8a4e761ea0ed4ef`; SHA-256
`b96e450dc26886362043b11a03ec2baff4fe21c5dddeaadf15c223c3ff0b1929`.
One owned visible window ran at a time, with no concurrent compilation during
the passing physical preflight. The machine was on battery, with no recorded
thermal warning. Local raw logs and failure artifacts are retained under
`scratch/agents/root-20261004-resumed/document-smoke-001` through `005`; they are
not build inputs or published performance evidence.

## Local checks

At this repair checkpoint:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --features native-image-tests,native-canvas-tests,performance-diagnostics --locked -j2`:
  928 pass, two existing skips.
- Native strict Clippy with those features and `--lib --tests -- -D warnings`:
  pass; Rust and Dune formatting checks pass.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/performance_document/main.exe @examples/performance_probe/ocaml/runtest`:
  pass, including the paired probe's OCaml command/event fixtures.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/performance/backend/Cargo.toml --locked -p gpuio-performance-probe --lib -j2`:
  the Rust command-boundary test passes.
- `python3 -m unittest discover -s scripts -p test_measure_document_growth.py`:
  five pass, checking both language fingerprints against the declared schema and
  rejecting missing/reordered growth, stale source totals, incorrect
  selection/page/full-source copies, incomplete traversal, missing fallback and
  regressed preparation counters.
- `python3 scripts/measure_document_growth.py --build-profile dev --smoke --output scratch/agents/root-20261004-resumed/document-smoke-005`:
  pass, owned child reaped and clipboard restored.

The probe schema is version 2 on both OCaml/Rust sides. Its new preparation
snapshot reads existing application-wide worker counters without scheduling
work or requesting redraw. Stage durations are cumulative worker elapsed times,
not CPU or GPU time. Readiness acknowledgement includes the collector's AX and
IPC latency. Neither establishes physical display presentation.

## Three full optimized runs — 2026-10-05

All three runs use source `84827c0db4dcaa33e431f458d2d96e5c3cb726d5`,
with a clean observation checkout and preserved executable SHA-256
`ef6703f5abcbcc9b74e4a8e57b7bae39fbea1a338d7241287b90e151229bddd6`.
The paired OCaml/Rust release build and OCaml probe tests passed:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j2 examples/performance_document/main.exe @examples/performance_probe/ocaml/runtest
```

A separate smoke warm-up preceded **each** full measurement. Each full invocation
used the preserved executable with `--build-profile release --check-budgets
--timeout 1200`, through `scripts/measure_document_growth.py`, and a fresh output
directory. All three warm-ups and all three full attempts passed; no full-run
outlier was discarded. Earlier failed development preflights remain described above.

| Run | Draw samples | p95 ms | p99 ms | Maximum bucket ms | Peak process RSS bytes | Wall seconds |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| [001](document-growth-run-001-och17.json) | 6,476 | 2.383871 | 3.340287 | 5.922815 | 236,273,664 | 148.316 |
| [002](document-growth-run-002-och17.json) | 6,488 | 2.469887 | 3.637247 | 5.910527 | 237,518,848 | 147.853 |
| [003](document-growth-run-003-och17.json) | 6,479 | 2.502655 | 3.493887 | 6.258687 | 228,491,264 | 148.948 |

Every full run verifies 160 appends, exactly 20,971,520 retained source bytes,
three live documents, and rejection without mutation of an extra byte beyond
each 8 MiB source. It visits all 187, 187 and 94 pages forward and backward,
checking the installed byte bounds and canonical content. The 1,280 expanding
selection copies, final-line/page/full-source copies, reset and profile removal
all pass. There are zero dropped native input timestamps and all final owned
application/resource/work-queue counters retire; clipboard restoration succeeds.
This does not replace the independent concurrent-typing workload.

Append→publication p95 is 12.130 / 11.267 / 11.862 ms.
Publication→AX-ready acknowledgement includes the external collector's 100 ms
polling cadence and IPC; its roughly 108 ms p95 is **not** document preparation
time. The raw reports retain every append observation and stage counter. Large
Markdown source fallback is explicitly observed at every append. Growth processes
165 jobs and 614,465,702 cumulative source bytes in each run; that byte counter is
repeated job volume, not live retained memory. Tiny cumulative parse values do
not mean 20 MiB of rich Markdown was parsed. Worker peaks are one concurrent job
and 24,368,497 reserved bytes; these are worker accounting, not process/GPU memory.

Reference hardware is the Apple M1 Max / 32 GiB / macOS 14.5 arm64 machine in the
plan. The built-in display reports 1728×1117 logical pixels and 120 Hz mode. The
app uses a normal visible 1200×800 window; mode metadata is not measured FPS. All
three start on battery, with no recorded thermal/performance warning. One owned
GUI ran at a time and no local compilation ran during the batch. Concurrent work
was read-only source/CI inspection, scratch notes, a small Git push and Linear
updates. The owned display-idle assertion and all GUI/collector children were
released at completion. Ordinary unrelated desktop processes were not stopped.

Native draw/submission histograms do not measure GPU completion or physical
presentation. RSS does not establish Metal/IOSurface allocation or physical
footprint. Full AX selection-range/VoiceOver qualification and the remaining
release workloads remain open.
