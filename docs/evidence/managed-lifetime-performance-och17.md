# Managed lifetime allocation — OCH-17

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
