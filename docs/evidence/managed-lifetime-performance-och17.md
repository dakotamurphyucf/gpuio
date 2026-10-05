# Managed lifetime allocation — OCH-17

**Superseded correctness finding, 2026-10-05:** the token-allocation optimization
below fails branch reactivation with constant configurations. Earlier release
`runtest` commands did not execute inline tests, which Dune disables by default
in that profile. Their suite-pass claims are withdrawn. See the
[gallery lifetime investigation](gallery-lifetime-repairs-och41.md) for actual
failing regressions and corrected test configuration. Recorded timing reports
remain historical measurements; they do not qualify a corrected implementation.

2026-10-05, macOS 14.5 arm64, M1 Max, 32 GiB, built-in 120 Hz display.
Implementation: `562c44ee2f16955413a4167eae41d4b82545d578`.
This follow-up identifies a concrete OCaml startup cost and removes it without
changing the renderer, vendored Bonsai or declared performance thresholds.
VoiceOver remains untouched and on hold.

## Cause and change

The earlier [startup investigation](presentation-startup-investigation-och17.md)
located a long gap between native frames. A temporary bounded trace around the
OCaml driver now measures a 103.687 ms Bonsai flush while initially materializing
the table, followed by a 42.868 ms flush after lifecycle activation. Reconciliation
takes 4.230 ms in the first step. These observations explain a substantial source
of delay; they do not attribute every workload's startup variation to this code.

`Managed_rows.assoc` used `Bonsai.Expert.thunk` to create each lifetime token.
In the pinned v0.17 implementation, that helper freezes its value through a
state model and an after-display activation action. A virtual table nests managed
associations through rows and columns, multiplying that unnecessary model/action
cost across active cells.

The wrapper now maps its association's stable key to allocate the token. Pinned
`Eval.Assoc` creates a fresh constant key node for each active map entry; data
updates preserve that node, while removal and reinsertion recreate the subtree.
The lifecycle and model-reset wrappers remain, including rejection of effects
from retired lifetimes. No user model, lifecycle hook or stale-effect check is
removed. This reasoning depends on the pinned association behavior and must be
reviewed when upgrading Bonsai.

With the same scratch instrumentation, the candidate's longest initial flush is
32.003 ms; its next-longest recorded flush is 0.753 ms. The native gap between
submission and the next draw falls from 98.130 ms to 33.770 ms in this pair.
These are diagnostic comparisons, not a statistical bound or release acceptance.

## Correctness and build checks

The new nested-association expect test runs with optimization both enabled and
disabled. It checks separate tokens for rows sharing the same column key,
preservation across data updates, preservation of a sibling when another row
retires, new tokens on row and column remount, and continued rejection of stale
effects. Existing activation/deactivation, model reset and 1,000-visit tests pass.
The virtual-list suite passes before and after the change; the full OCaml suite
and four optimized presentation executables pass after it.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest --profile release
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release \
  examples/performance_presented/list/main.exe \
  examples/performance_presented/table/main.exe \
  examples/performance_presented/document/main.exe \
  examples/performance_presented/streaming/main.exe
```

The two changed OCaml files pass the repository's pinned formatter. Tracked trace
overlays were restored byte-for-byte; normal executables were rebuilt. The restored
table executable matches the preserved candidate binary exactly, SHA-256
`08d05aac93a4d13334dff5b336cf883ef15317788b75d8c01d7fd8c0f2883b88`.
[Validation logs](managed-lifetime-performance-och17/validation.tar.gz).

## Fresh untraced smokes

All three run at clean `562c44e`, one visible application at a time and without
concurrent compilation. Each passes its unchanged smoke validation and closes.

| Workload | First positive presentation | Presented / attempts | Peak RSS bytes |
| --- | ---: | ---: | ---: |
| Table | 71.425 ms | 412 / 415 | 279,658,496 |
| Loaded list | 35.395 ms | 225 / 227 | 123,453,440 |
| Growing document | 63.159 ms | 191 / 194 | 124,305,408 |

The unpresented attempts are retained initial input-free zero timestamps under
the declared startup policy. There are no other losses, pending callbacks or raw
trace truncation. Table and list settled idle have zero draws and presentation
attempts; document interactions include clipboard restoration. All content,
boundedness and cleanup checks pass. These are smoke workloads, not full release
repetitions.

The table visits all 1,030 rows in both directions and all 64 columns, with at most
26 active rows / 1,664 cells and 512 cached rows. Its measured history decreases
from 17.239 to 8.096 seconds against the retained pre-change smoke, and peak RSS
decreases from 346,488,832 to 279,658,496 bytes. That single before/after pair
supports further qualification but is not a general performance guarantee.

```sh
python3 scripts/measure_table_history.py --build-profile release --presentation \
  --smoke --executable _build/default/examples/performance_presented/table/main.exe \
  --output scratch/agents/root-20261004-resumed/key-token-table-smoke-001
python3 scripts/measure_list_history.py --build-profile release --presentation \
  --smoke --executable _build/default/examples/performance_presented/list/main.exe \
  --output scratch/agents/root-20261004-resumed/key-token-list-smoke-001
python3 scripts/measure_document_growth.py --build-profile release --presentation \
  --smoke --executable _build/default/examples/performance_presented/document/main.exe \
  --output scratch/agents/root-20261004-resumed/key-token-document-smoke-001
```

[Raw smoke reports and driver logs](managed-lifetime-performance-och17/smokes.tar.gz).

## First full loaded-list repetition

The first full optimized list run after the change passes all declared budgets.
It launches at clean `562c44e`, using SHA-256
`584e314701399e2850123e191f637d69a2225f2e2f15c506ada31d2de08f4149`.
Only this evidence documentation changes while the binary runs; application
source and executable remain unchanged. No other owned GUI or compiler runs.

- All 10,000 rows are traversed in both directions, with at most 32 active rows;
  row growth, anchor and acknowledged cleanup checks pass.
- 27,655 positive presentations from 27,658 attempts; three initial input-free
  zeros, startup 88.085 ms, no other loss or pending callbacks.
- Submission-to-presentation p95/p99: 32.260/32.293 ms. Native CPU draw p95/p99:
  3.166/4.108 ms. Raw trace retains 4,096 records and explicitly counts 23,562
  truncated records; cumulative histograms include all outcomes.
- History takes 231.041 seconds; total process wall time 297.737 seconds;
  peak RSS 181,993,472 bytes.
- All 232 history and 60 idle observations, plus interval ends, are active and
  visible. The full 60-second settled idle records zero CPU draws and native
  attempts. The collector and application exit zero and their processes retire.

```sh
caffeinate -di python3 scripts/measure_list_history.py \
  --build-profile release --presentation --check-budgets \
  --executable _build/default/examples/performance_presented/list/main.exe \
  --output scratch/agents/root-20261004-resumed/key-token-list-full-001
```

`caffeinate` is scoped to the driver and exits with it; no persistent power
settings change. [Raw report and logs](managed-lifetime-performance-och17/list-full-001.tar.gz).
This is one passing repetition, not completion of the three-run requirement.
The earlier full-list startup failure remains recorded separately.

## First full paged-table repetition

The first full table presentation run after the change passes all declared
budgets. It launches at clean `0579ab8`, whose application sources are identical
to `562c44e`, using the table hash recorded above. A test-only lifecycle regression
is added during measurement; no compilation, application change or second owned
GUI overlaps the run.

| Measurement | Result |
| --- | --- |
| Coverage | All 100,000 rows, forward and backward; all 64 columns; selection and reveal |
| Active/cache bounds | 26 rows / 1,664 cells; 512 cached rows; 1,564 page loads / 1,560 evictions |
| Presentation accounting | 28,323 positive / 28,325 attempts; two initial input-free zeros; no other loss or pending callbacks |
| Startup transition | 58.394 ms |
| Submission-to-presentation p95 / p99 | 20.726 / 26.067 ms |
| Native CPU draw p95 / p99 | 14.057 / 14.483 ms |
| History / process wall time | 647.777 / 716.314 seconds |
| Process peak RSS | 516,063,232 bytes |
| Settled idle | Full 60 seconds; zero CPU draws and native attempts |

All 648 history and 60 idle observations plus interval ends are active and
visible. Raw trace retains 4,096 records and explicitly counts 24,229 truncated
records; cumulative histograms include all outcomes. Application and collector
exit zero, all cleanup counters pass, and the owned processes retire.

```sh
caffeinate -di python3 scripts/measure_table_history.py \
  --build-profile release --presentation --check-budgets --timeout 3600 \
  --executable _build/default/examples/performance_presented/table/main.exe \
  --output scratch/agents/root-20261004-resumed/key-token-table-full-001
```

[Raw report, logs and preserved binary hashes](managed-lifetime-performance-och17/table-full-001.tar.gz).
This is one passing table repetition; two more remain required.

After the benchmark exits, the added regression verifies hiding and reopening a
containing `match%sub` branch with an unchanged constant row map. Both optimizer
modes preserve model reset, reject retired effects while hidden and after return,
create a fresh lifetime, and accept its new effects. The entire virtual-list
suite passes with `GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest --profile release
test/virtual_list`; the pinned formatter also passes. No runtime correction was
needed. The pinned switch's Incremental bind recreates the branch scope, which
is a second relevant invariant to retain when upgrading Bonsai.

## Diagnostic provenance and limits

[Trace archive](managed-lifetime-performance-och17/ocaml-traces.tar.gz) contains
the exact temporary four-file overlay, build logs, reports and parsed events.
Baseline is `af96ab9`; candidate is `562c44e`, both with the same trace overlay.
Scratch builds use the release command above, copy the table binary outside
`_build`, then restore all tracked overlay files. Enable both
`GPUIO_SCRATCH_FRAME_TRACE=1` and `GPUIO_SCRATCH_OCAML_TRACE=1` when invoking the
ordinary table smoke driver with that scratch executable.

The OCaml trace retains its first 512 monotonic-clock markers and exports only on
shutdown. Two open nesting entries at that cap indicate truncated observation,
not unfinished runtime work. These traces are neither complete-workload profiles
nor acceptance binaries. The baseline diagnostic fails startup at 145.828 ms;
the candidate diagnostic passes the collector. Neither replaces untraced results.

[Artifact checksums](managed-lifetime-performance-och17/manifest.json).
Full repetitions, collector overhead/resources and independent hosted Metal
calibration remain open. Original failed list/table/document measurements remain
failed; the release startup bound remains 100 ms.
