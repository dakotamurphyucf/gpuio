# Native presentation workloads

These qualification executables enable the optional Metal presentation collector.
Dune copies the exact OCaml sources from the existing list, table, document and
streaming workloads; only the statically composed backend changes. This is not a
new installed application API. Ordinary applications and CPU-only benchmarks keep
their existing feature selection.

Build before measuring, with no other compiler or GUI test running:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release \
  examples/performance_presented/list/main.exe \
  examples/performance_presented/table/main.exe \
  examples/performance_presented/document/main.exe \
  examples/performance_presented/streaming/main.exe
python3 scripts/measure_list_history.py --build-profile release --presentation --smoke \
  --executable _build/default/examples/performance_presented/list/main.exe \
  --output scratch/presented-list-smoke
python3 scripts/measure_streaming_typing.py --build-profile release --presentation --smoke \
  --executable _build/default/examples/performance_presented/streaming/main.exe \
  --output scratch/presented-typing-smoke
```

Table and document use `scripts/measure_table_history.py` and
`scripts/measure_document_growth.py` with the corresponding executable. Choose a
new output directory for each run. Keep the foreground window visible; typing and
document interaction tests require native keyboard focus. Every driver retains
raw logs, hashes, environment information, failures and cleanup evidence.
VoiceOver is neither needed nor operated by these workloads.

`--presentation` requires one native report per CPU phase. Omission rejects any
unexpected native reports to avoid silently mislabelling a presentation-enabled
run as CPU-only. Begin/Finish retain the existing v3 wire protocol. The optional
probe stops native admission, captures the CPU cutoff, settles callbacks for at
most two seconds, then exports JSON before acknowledging Finished. No export or
file I/O occurs inside a Metal callback. Dropping the probe cancels its task and
session. The raw trace is bounded at 4,096 records; histograms remain cumulative.

For full release runs, replace `--smoke` with `--check-budgets` and collect three
independent runs. The [declared qualification contract](../../docs/design/metal-presentation-qualification.md)
defines loss/accounting checks, the bounded input-free startup transition,
sample floors, timing budgets and limitations. Every skipped frame remains counted;
no later or input-bearing skips are accepted.
Existing workload content/CPU/resource gates remain in force. OS-reported Metal
presentation is separate from GPU execution duration and hardware/photon latency.
Read-only one-second visibility/activation samples do not prove continuous
visibility. Full acceptance also requires overhead and resource qualification;
a passing development smoke is not that evidence.

Regenerate the backend with:

```sh
python3 scripts/compose_backend.py examples/performance_presented/native.json \
  examples/performance_presented/backend
```

The component manifest's optional `features` array selects the package's
`presentation-diagnostics` feature. Features are sorted and unioned when component
and document-profile factories share a package. Cargo still validates that named
features exist. No global/default feature or unrelated toolchain is changed.
