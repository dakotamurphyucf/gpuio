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
