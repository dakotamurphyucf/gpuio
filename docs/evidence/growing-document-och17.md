# Growing-document qualification — OCH-17

Status: local behavior preflight passes; optimized performance acceptance is
pending. This is not full accessibility or physical-presentation acceptance.

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
