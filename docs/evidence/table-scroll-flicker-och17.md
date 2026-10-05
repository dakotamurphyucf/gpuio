# Table scroll flicker — OCH-17

On 2026-10-05 the owner reported a few stable table rows while the other rows
flickered during the qualification workload. A slower local macOS replay
reproduces the symptom: after a viewport jump, rows 21–23 retain their text,
while the following visible rows have borders but empty cells. The next captured
image shows rows 24 onward filled in. This is an open visual rendering issue,
not intended animation or completed table acceptance.

## Reproduction and limits

Base source is clean `5c3956d2fb280aed6cf2d6d8c1b48c3541fafedc` on macOS 14.5,
Apple M1 Max. The diagnostic copies `examples/performance_table/main.ml`, adding
`Eio.Time.sleep clock 0.75` and a `diagnostic-scroll` log at the start of `move`.
It uses the same presentation-enabled executable dependencies as
`examples/performance_presented/table/dune`, built in a temporary sibling
directory with the repository release profile. That directory was removed after
copying the executable to scratch. No production source was changed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j 2 \
  examples/performance_presented/table_scroll_diagnostic/main.exe
python3 scripts/capture_window_startup.py \
  --output scratch/agents/root-20261004-resumed/table-paced-capture-001 \
  --title 'GPUIO · Paged table qualification' \
  --seconds 12 --max-frames 80 --interval 0.01 -- \
  /Users/dakotamurphy/gpuio/scratch/agents/root-20261004-resumed/table-paced.exe \
  --smoke --wall-clock
```

Diagnostic executable SHA-256:
`e63474bdc21adb6a7ff142849be053c8f4f94d39a970dfe2f226b4782e280beb`.
The capture collected 69 samples, then terminated and reaped its own child at
the declared deadline. No concurrent build or other owned test window ran.
The workload was deliberately interrupted; this is not a complete traversal,
resource-cleanup acceptance test or latency measurement.

Images 22 and 23 were visually inspected. Capture intervals were respectively
4.703–4.841 and 4.857–4.998 seconds after launch. They establish blank content
followed by populated content, **not the exact duration** of the blank display.
Sampled first-column text scans also identify the same three-row-only pattern
in images 28, 34, 40 and 46. The scan is a diagnostic aid, not a general GUI oracle.

The workload moves by roughly a viewport minus one row, with only two overscan
rows on each side. Native `Delegate::render_td` in `rust/native/src/table_view.rs`
returns an empty full-size element when a row has no materialized cell mapping.
The retained overlap and newly missing rows fit this native/Bonsai preparation
gap. Further tracing is needed before choosing a repair: the eventual fix must
preserve bounded row ownership, asynchronous delivery and correct logical row
identity. Increasing delays in a benchmark is not a production fix. Ordinary
wheel/trackpad behavior and arbitrary jumps still need explicit coverage.

## Separate current startup failure

Before the diagnostic, all four current presentation workloads built successfully
with `--profile release -j 2`. The unmodified table smoke at the same clean
revision completed its interactions and application cleanup, but the measurement
driver failed the unchanged 100 ms startup gate: first positive presentation
was approximately **147.364 ms** after first submission. It recorded 421 attempts,
418 presentations and three initial input-free zero timestamps, with no subsequent
loss and zero new frames during settled idle. No threshold is waived.

```sh
caffeinate -di python3 scripts/measure_table_history.py \
  --build-profile release --presentation --smoke \
  --executable _build/default/examples/performance_presented/table/main.exe \
  --output scratch/agents/root-20261004-resumed/current-easing-table-smoke-001
```

Unmodified executable SHA-256:
`d8d64a6960e27350e5a7ae4f741545fad0d09afb7e3043f8f6587bb72bfa033c`.
Its startup failure and the visual flicker are both open; this evidence does not
prove they have one underlying cause.

[Diagnostic archive](table-scroll-flicker-och17/diagnostics.tar.gz) includes the
paced source, build/driver logs, capture report, selected original PNGs and the
unmodified failed smoke's raw report/application log.
[Manifest](table-scroll-flicker-och17/manifest.json) hashes were verified against
the archive. Other original sampled PNGs remain in the local scratch workspace.
