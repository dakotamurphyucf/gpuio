# Growing-document qualification driver

OCH-17 workload with three passing optimized runs at a recorded checkpoint.
See the [qualification evidence](../../docs/evidence/growing-document-och17.md)
for exact sources, raw reports and remaining accessibility/presentation limits.

The normal foreground 1200×800 window grows three retained Markdown sources to
8, 8 and 4 MiB. Each append contributes exactly 128 KiB of deterministic UTF-8.
The native view uses a 700-pixel viewport. Source fallback above the rich-parser
budget is required and identified in every growth observation. Per-source
admission remains 8 MiB: additional appends must fail without changing the source.

Each numbered checkpoint blocks the producer while the macOS harness operates
only on the owned application PID. Eight native selection expansions, each checked
through Copy against the canonical source prefix, and page
navigation accompany each append. At each final size, all source pages are traversed forward and backward,
checking their byte intervals and text against independently generated canonical
content. Native end navigation plus selection/copy of the final line, and
select-all/copy are checked within the last page; Copy source separately checks
the complete source. This preserves the
accepted page-local selection contract for huge documents. Clipboard change counters prevent a prior correct copy from
masking a later failed action. Original eagerly readable clipboard items/types
are restored by the existing preservation scope; unavailable or oversized
original representations abort before mutation. The current source view does not
expose AX selection ranges. These clipboard checks establish native selection
behavior, not selection-range accessibility acceptance; that remains an open
OCH-17 accessibility requirement.

After reaching 20 MiB aggregate retention, the driver resets the first source to
small rich content, verifies the static profile's code action, removes that
profile and verifies the built-in action returns. It copies the reset source
again and closes the window. Application resources, work queues and window scopes
must retire. This does not supply a native-entity or GPU allocation census.

Measurements distinguish append-to-publication, publication-to-AX-readiness acknowledgement,
native submitted-frame histograms and cumulative document-worker elapsed counters
(queue/configuration/parse/highlight/search). Worker durations are not CPU time;
the AX observation includes collector polling and acknowledgement latency and is
not a parser-duration measurement. Native source-page metadata is derived from
the installed snapshot; its total byte count and page contents must match before
the harness acknowledges readiness. The public `on_preview` callback remains
limited to Markdown/HTML Flow; this viewport workload uses the supported native
accessibility surface. Frame callbacks are not physical presentation. Counter snapshots
include discarded work and absolute resource peaks. The collector retains every
raw histogram, interaction record and failure report. No forced GC or cache purge
is used. The harness's memory/CPU is outside the owned application's wait4 totals.

Build and smoke-test locally:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/performance_document/main.exe
python3 scripts/measure_document_growth.py --build-profile dev --smoke \
  --output scratch/document-smoke-001
```

For acceptance, build an optimized paired executable, preserve/hash it, run an
explicit smoke warm-up and then three full runs in fresh output directories:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j2 examples/performance_document/main.exe
python3 scripts/measure_document_growth.py --build-profile release --check-budgets \
  --output scratch/document-full-001
```

At least 1,000 native draw samples are required, with p95 ≤16.7 ms, p99 ≤33.4 ms,
peak RSS ≤1.5 GiB and no dropped native input timestamps. The smoke workload uses
2, 2 and 1 chunks; it is behavior preflight only. Keep all failed attempts and
record concurrent activity. No local compilation or second GUI during measured
runs. See [the qualification plan](../../docs/design/performance-qualification.md).
