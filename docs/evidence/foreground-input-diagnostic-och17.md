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

## Full-workload diagnostic follow-up — 2026-10-08

One predeclared diagnostic reuses the exact executable above, with the journal
enabled and the unchanged 10,000-row traversal, row growth and sixty-second idle.
It completes normally in 302.902 seconds, including the application's cleanup
marker and zero windows, pending requests and native command queue. The child and
owned keep-awake process are reaped. No compiler or second owned GUI runs alongside
it, and the helper injects no input. It requests activation once and checks foreground state during startup, then
releases its AX reference and makes no later AX queries. Exact preflight-to-capture ordering was not separately timestamped;
this is diagnostic evidence, not an unperturbed acceptance trial.

| Phase | CPU interval seconds | Draw samples | Input-bearing samples | Journal inputs / lost entries |
| --- | ---: | ---: | ---: | ---: |
| Full history | 237.833 | 28,452 | 0 | 0 / 101,433 |
| Settled idle | 60.008 | 0 | 0 | 0 / 0 |

All 238 history and 60 idle window observations are active/visible; neither
observation list is truncated. The bounded foreground journal overwrites older
history entries, so its zero retained input count is not a complete history
event census. The independent CPU histogram records zero input-bearing samples.
The idle collector starts fresh and loses no entries: that full interval has
zero input events and zero draws.

This demonstrates that the full fixture can settle without drawing; the earlier
900-input/908-draw idle result is not inevitable on every execution. It does not
identify that event producer, explain the separate fourteen-draw/no-input failure,
resolve the historical startup outlier or establish the cause of reported row
overlap. No retry-until-pass loop or threshold change was used. The next paired
overhead comparison still needs controlled desktop conditions, final optimized
paired builds and diagnostics disabled, with all original workload/budget checks.
This journal-enabled result must not count as one of those acceptance runs.

The [five-file archive](foreground-input-diagnostic-och17/full-reports.tar.gz)
retains the predeclared plan, exact helper/command and binary hash, raw application
log, console log and final summary. Every member was read back and checked against
its [manifest](foreground-input-diagnostic-och17/full-manifest.json). No production
source or performance validator changes accompany this follow-up.
