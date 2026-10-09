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

## Bounded command preparation — local correction

Later on 2026-10-05, a correction based on `4ccd87f` prepares destination cell
descriptions in the same Bonsai publication as a programmatic scroll command.
It retains native layout ownership, row/cell caps and pins. Preparation stays
until a current native observation arrives; delayed older effects cannot undo
it. Superseding an undisplayed batch cancels its preparation. The public contract
and remaining geometry limitations are in [data tables](../design/data-tables.md).
No Rust synchronous callback, expanded budget or benchmark pacing change is
required by the production correction.

Both the existing dev virtual-list suite and the suite with the new regression
pass. The final release build also passes `@test/virtual_list/runtest` and builds
the normal and paced presentation executables. The regression inspects actual
published rows alongside commands, pinned-row priority, retention after local
acknowledgment, stale effects, native demand replacement, end jumps, superseding
batches, final batch destination and query reset. Final `@fmt` passes. An extra
direct invocation initially used the wrong working directory for ppx_expect;
its failed log is retained. Repeating from `_build/default/test/virtual_list`
passes with `-matching 'scroll prepares bounded destination cells' -strict
-show-counts` (one test). This does not replace the full suites above.

The same 12-second paced capture, using `table-paced-prepared.exe`, collects
70 samples. Its initial loading image is blank; all 69 subsequent sampled
first-column regions have text in every inspected row. Original images show the
table advancing through rows 168–188 without the previous three-row-only state.
The diagnostic executable SHA-256 is
`b35d82ea578b313258dd3870035c293bb6718546bb55fac7284df10847fc69fa`.

A second capture uses the **unpaced** production workload for 10 seconds, with
`--max-frames 70 --interval 0.01 --smoke --wall-clock`. It retains 62 complete
images; none after initial loading has the earlier blank-row pattern in the
inspected region. Image 40 was visually reviewed and shows populated rows
525–545. The final screenshot times out with only 96 ms left before the declared
deadline, so the collector exits nonzero and marks the report incomplete. The
partial captures are evidence of those sampled states, not a passing collector
run or proof that every displayed frame is stable. Both captures terminate and
reap their owned children. No GUI overlaps another or a compilation.

Separately, the complete unpaced smoke passes:

```sh
caffeinate -di python3 scripts/measure_table_history.py \
  --build-profile release --smoke --wall-clock \
  --executable _build/default/examples/performance_presented/table/main.exe \
  --output scratch/agents/root-20261004-resumed/table-prepared-history-smoke-001
```

It covers all 1,030 rows in both directions, all 64 columns, selection/reveal and
cleanup. Observed settled peaks are 26 active rows / 1,664 cells and 512 cached
payload rows; the regression and presenter also enforce budgets during command
preparation. Peak process RSS is 363,462,656 bytes. All required post-close
resource and queue checks pass. Executable SHA-256:
`705a47bf2841afaf856ed17f385c933b56966e62e94ad3461563c3d54ae3da8c`.

This functional wall-clock smoke **does not qualify presentation latency or idle
draw counts**, and does not waive or replace the original startup failure.
Ordinary wheel/trackpad scrolling, changed/clipped geometry, full performance
repetitions and hosted execution of this correction remain unqualified here.

[Correction archive](table-scroll-flicker-och17/prepared-scroll.tar.gz) retains
source snapshots, logs, partial-capture failure, pixel-inspection script, capture
reports, selected unmodified PNGs and the complete smoke report.
[Verified manifest](table-scroll-flicker-och17/prepared-scroll-manifest.json).

## Native pixel-wheel follow-up

A further diagnostic at `b457e0c` holds the public table before its programmatic
history loop, using Eio sleeps and viewport logging. The mouse harness sends
12 events each of −32, −96, +32 and +96 pixels through macOS CoreGraphics,
checking that the pointer target belongs to the owned app before each event.
It captures the initial populated table and each subsequent transition, then
terminates and reaps its child. The temporary diagnostic source directory is
removed after building with the same isolated release profile.

The first attempt posts events directly to the PID. Its collector succeeds,
but the independent viewport log stays at row zero throughout: **no scrolling
acceptance follows from that run**. The corrected harness uses the global event
tap with the same ownership hit-test, matching the existing native pointer
walkthroughs. Its 99 viewport observations establish movement from row 0 through
48 and back to 0. All 49 sampled first-column regions remain populated; image 24
was visually reviewed and shows rows 48–68. This provides actual OS pixel-wheel
coverage in both directions, without using a Bonsai scroll controller or changing
the production native table. It does not prove every frame, physical trackpad
momentum, arbitrary velocities, clipping/resizing or presentation-latency budgets.

Diagnostic executable SHA-256:
`854693e6a3ae976d735859b199529cb319702a7ffc5bfc1d8a1342684675a278`.
The source builds as `examples/performance_presented/table_wheel_diagnostic/main.exe`
with the normal presented-table Dune stanza. Both harness versions, source,
logs, reports and selected original screenshots are in the
[18-file wheel archive](table-scroll-flicker-och17/wheel-diagnostic.tar.gz), with
a [verified manifest](table-scroll-flicker-och17/wheel-manifest.json). No application
or global input-source settings were changed. Neither this interrupted diagnostic
nor the earlier functional smoke replaces the still-open startup/performance gate.

On 2026-10-05, the owner also reported that the table flicker looks fixed after
visually checking it. This supports the sampled evidence above; it does not extend
that evidence to every frame, all input devices or performance acceptance.
