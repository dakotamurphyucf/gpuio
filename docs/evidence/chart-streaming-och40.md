# Public chart streaming measurement (OCH-40)

Local Apple M1 Max, 32 GiB unified memory, macOS 14.5 arm64. Stock OCaml 5.3,
Rust 1.97.1, repository development build and pinned GPUI. Measured after
`7a716fe` with the workload/diagnostics changes committed alongside this report.
No simultaneous build or other GPUIO test ran. This is one diagnostic run on the
owner's development desktop, not a controlled release-build benchmark.

The public `examples/chart_stream` application completed 150 desired updates,
coalesced into **80 observed publications**, each followed by an actual native
render callback. Stable source IDs and complete published dataset equality were
checked; exact mode retained all 100,000 values. The default policy is explicit
line-envelope sampling, not a hidden loss of data.

| Dataset / policy / burst | Samples | Build median ms | Update→render median / p95 ms | Max plan bytes | Median submitted bytes |
| -- | --: | --: | --: | --: | --: |
| 10,000 / envelope / 1 desired | 30 | 3.27 | 45.32 / 48.47 | 725,704 | 209,806 |
| 100,000 / envelope / 1 desired | 30 | 34.58 | 218.46 / 225.59 | 754,592 | 2,234,426 |
| 100,000 / exact / 1 desired | 10 | 37.59 | 568.28 / 576.88 | 49,278,656 | 2,234,426 |
| 100,000 / envelope / 8 desired | 10 | 287.06 | 219.42 / 232.20 | 754,592 | 2,234,426 |

The 100,000-point sampled plans retain approximately 1,500 representatives and
8,700 expanded vertices; exact plans retain 100,000 and approximately 566,000
vertices. The same plot size (800×400 logical pixels) and dataset shape are used.
Update latency excludes dataset construction and includes source scheduling,
serialization/FFI/native decoding, worker preparation and the render callback.
It is **not** isolated paint time, GPU timing, physical presentation or FPS.

Whole-child measurement: **21.68 seconds wall time**, **19.49 seconds user CPU**
and **0.80 seconds system CPU**. Peak RSS was **807,387,136 bytes**
(770.0 MiB). This includes the OCaml runtime, native/GPU host, exact
plans, retained burst inputs and allocator/GC high-water state; it is not a hard
framework memory bound. The 8-input burst intentionally keeps its constructed
snapshots alive while the scheduler coalesces, so RSS includes more than the
scheduler's charged data. No per-frame CPU estimate is inferred from process CPU.

Native accepted-command queue peak: **262,162 encoded bytes**, approximately one
256-KiB chart chunk plus framing. Rejected commands never increase that counter;
pop/close release current queue charges, with lifetime peak preserved. This counts
commands waiting for dispatch, not currently executing work or response events.

Sampled OCaml chart scheduler peak: **102,531,200 charged bytes**, below its 128-MiB
limit. The settled 100,000-point registration charge was 51,265,600 bytes, and
release returned registrations/charge to zero. This conservative admission charge
includes canonical/converted data and encoding allowance; it is not heap usage.
At most one pending correlated request was observed. Eight-input bursts used the
same median upload bytes as a single 100,000-point update, and advanced the native
publication exactly once. No submission queue accumulated in the OCaml command
queue at the sampled instants; the native lifetime peak provides the stronger
between-sample command-byte observation.

## Reproduction and interpretation

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/chart_stream/main.exe
python3 scripts/measure_chart_stream.py --output scratch/chart-stream
```

The runner writes the full application log and per-workload median/p95/max JSON,
with revision, dirty-state, hardware/toolchain, exact child CPU and RSS metadata.
It closes/reaps the child on success or failure. The executable has bounded Eio
waits, checks latest data/retention and zero registration charge at release.
Native multi-window/lifecycle tests separately establish zero output/workspace
charges; this workload does not derive native memory cleanup from OCaml counters.

This establishes bounded realistic public updates and a reproducible baseline.
Full 100,000-point replacement is expensive in the development build; applications
should choose an appropriate update cadence and explicit sampling. Sampling bounds
plot work but does not avoid uploading/validating the original dataset. Incremental
native dataset editing and optimized-build comparisons are future optimizations,
not silently assumed capabilities. Required hosted macOS/Linux build/test gates,
OCH-29 integration and Linux graphical release validation remain separate.

## Consolidated rerun at `08423cd`

The final local 80-publication workload also passed all data, queue and cleanup
assertions, with peak RSS 797,048,832 bytes and the same native queue/source-charge
bounds. Median update→render callbacks were 46.43 ms (10k sampled), 224.59 ms
(100k sampled), 602.15 ms (100k exact), and 218.31 ms (100k sampled eight-update
burst). The run took 89.33 seconds wall time, 22.83 seconds user CPU and 2.02 seconds
system CPU. **One exact-mode sample took 63,050.99 ms**, so its p95/max is also
63,050.99 ms. The earlier baseline must not be read as a latency guarantee.

The workload did not record window visibility at that sample, so its cause is
unverified. The pinned GPUI macOS backend explicitly stops the display link when
its native window becomes occluded (`gpui_macos/src/window.rs`,
`window_did_change_occlusion_state`); a requested render callback can therefore
wait for visibility rather than measure continuous rendering work. A later local
Signal Studio screenshot check separately found its live child missing from the
on-screen window list. These observations are consistent with desktop visibility
interference, but do not prove it caused the benchmark outlier.

`App.Window.request_frame` documents this limitation. Application/data readiness
must not wait for paint; Signal Studio now uses model/resource/native-window
readiness and tests initially unfocused link delivery. The input harness raises
only its owned window before capturing evidence. No renderer performance fix or
stronger latency claim is inferred from a successful workload result.

## OCH-17 diagnostic stages (2026-10-03; physical rerun pending)

The historical 63,050.99 ms sample remains unexplained. A source audit found that
`Chart.Event.Ready` is itself emitted by the chart's native preparation during
rendering (`rust/native/src/chart_view.rs`, `State::prepare`), after worker results
become available. It is not a render-independent worker-completion timestamp.
The workload now retains readiness/failure independently of selection events, so
interacting with the chart cannot replace the readiness observation it awaits.
That was a harness weakness, not a demonstrated cause of the old timing sample.
`Gpuio_eio.Chart.is_published` instead observes acceptance of the latest dataset.

The public workload now uses version 2 telemetry with these separate wall-clock
intervals, retaining the original total:

| Metric | Observed boundary | Interpretation limit |
| -- | -- | -- |
| `build_ms` | Build desired datasets | Includes the whole coalesced burst. |
| `published_ms` | Submit desired data → observe latest native publication | Includes UI scheduling, encoding/upload/decoding and observation; not isolated FFI or worker CPU. |
| `published_ready_ms` | Observe publication → observe matching `Ready` | Includes preparation, rendering/visibility scheduling and event delivery; not isolated tessellation time. |
| `ready_frame_ms` | Observe `Ready` → requested render callback | Can wait on display scheduling/occlusion; not GPU completion or physical presentation. |
| `update_frame_ms` | Sum of the three post-build stages | Comparable in meaning, but instrumentation adds observable overhead. |

Each stage logs a sequence ID, elapsed monotonic time and the last public window
active-state snapshot (0 unknown, 1 inactive, 2 active). Activity means focus,
**not visibility**: an active snapshot cannot exclude occlusion or prove physical
presentation. Phase markers before waits retain the last observed stage when a
process fails or times out. No new synchronous native-to-OCaml callback is used.

The runner now writes `report.json` with `complete=false`, error, partial samples,
phase history, last phase and collected child CPU/RSS on failure or interruption.
It checks exact group counts/iteration order, finite metrics, additive stage times,
phase order and zero cleanup charge before reporting measurement completion.
Completion still means a valid diagnostic workload, not a performance-budget pass.
It starts an owned process group and terminates/kills/reaps it on all exit paths.
Output directories must be new, so failed runs cannot overwrite earlier evidence:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/chart_stream/main.exe
python3 scripts/measure_chart_stream.py --output scratch/chart-stream-stage-run-001
```

Local validation on macOS arm64 at the dirty worktree based on `83eb87e`:

- `python3 scripts/test_measure_chart_stream.py`: **9 tests pass**, covering strict
  synthetic telemetry, missing/duplicated groups, invalid numbers, lost cleanup,
  phase ordering, partial reports, real non-GUI child exit/timeout and supervisor
  SIGTERM. A child ignoring TERM is killed/reaped within the bounded cleanup.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/chart_stream/main.exe`:
  passes. No chart window was launched; no new performance numbers were measured.
- The same no-display Python checks are added to both CI platforms. Hosted execution
  has not yet been established for this change.

Next actual desktop qualification must record display/visibility and concurrent
activity, use the staged report to locate any delay, then investigate that stage.
Do not assign the old outlier to occlusion, declare the renderer repaired, or adopt
latency thresholds from the synthetic tests. Named reference budgets, true physical
input/presentation measurements and the rest of OCH-17 remain open.

Final diagnostic checks: Python compilation and CLI help, repository formatting
and `git diff --check` pass. The updated chart executable also builds after the
selection-observation correction. No new frame timing or physical acceptance is
inferred from these compilation/offline checks.
