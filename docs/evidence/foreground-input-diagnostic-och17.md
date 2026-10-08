# Foreground input diagnosis — OCH-17

This adds opt-in instrumentation to investigate unexpected input in the
[incomplete overhead comparison](collector-overhead-och17.md). It does not fix or
qualify that workload. Sources are based on `9dbbcc2b`; exact changed sources,
commands/helper and logs are in the [verified archive](foreground-input-diagnostic-och17/reports.tar.gz),
with its [manifest](foreground-input-diagnostic-och17/manifest.json) and
[summary](foreground-input-diagnostic-och17/summary.json).

`GPUIO_DIAGNOSE_FOREGROUND_INPUT` enables a cursor on the existing bounded GPUI
foreground journal. At the measurement cutoff the probe drains it once and
reports input-kind counts, invalidation flags and lost entries. It adds no
sampling timer or input payload logging. The journal covers the foreground
thread, not a particular window, and cannot identify the event producer.
See the [implementation walkthrough](../../examples/performance_probe/rust/src/presentation.md#opt-in-foreground-input-diagnosis).

The release report validator rejects every enabled diagnostic report, including
zero-input reports. Default reports have a null field; historical reports with
the field absent remain compatible. No performance budget changes.

## Validation on the physical Mac

The optimized list executable SHA-256 is
`92fe5c3cd058d2ee70e2f93b6508210a68f46d7500fc753d8b11f03115414ada`.
Both short runs use the same binary and enable the diagnostic only in the child
environment. They activate the owned window during startup and reap it afterward.

- Quiet smoke: normal exit, both phase summaries contain zero inputs and zero
  lost entries.
- Deliberate-input smoke: four guarded OS mouse movements during idle; the
  summary contains exactly four `mouse_move` events, zero marked invalidating
  and zero lost entries. The unchanged idle check fails with one CPU draw and
  exit 2. This verifies event detection, not performance acceptance or the cause
  of the earlier 900 input-bearing frames.
- The initial quiet attempt failed before measurement during window discovery.
  Its log is retained. The helper now waits up to five seconds for startup
  readiness; it does not retry measurement assertions.

Exact build/check commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/performance_presented/backend/Cargo.toml --locked -j2 -p gpuio-performance-probe --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --manifest-path examples/performance_presented/backend/Cargo.toml --locked -j2 -p gpuio-performance-probe --lib --tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 --profile release examples/performance_presented/list/main.exe
python3 -m unittest discover -s scripts -p test_presentation_report.py
python3 scripts/audit_example_docs.py
git diff --check
```

Four Rust tests, strict probe Clippy, optimized build, thirteen Python tests and
the documentation inventory pass. Existing upstream Cocoa deprecation and
`block` future-compatibility warnings remain. This is macOS diagnostic evidence,
not Linux GUI, repeated full performance or reported row-overlap acceptance.

The owner confirmed switching away from or covering the previously queried
timed-out benchmark window. This supports a visibility contribution to that
frame wait; it does not explain the separately reported visual overlap or prove
the cause of older idle redraws. No long benchmark was rerun for this checkpoint.
