# Loaded-history measurement

This qualification workload uses the public Bonsai managed list and a private
statically linked snapshot probe. It is not an installed GPUIO API. The ordinary
native backend does not enable its profiler.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance/main.exe
python3 scripts/measure_list_history.py --build-profile release --smoke --output scratch/performance-smoke
python3 scripts/measure_list_history.py --build-profile release --check-budgets --output scratch/performance-full-1
```

Smoke exercises 96 rows and two seconds of idle. The full workload loads 10,000
mixed 64/256/2048-byte UTF-8 rows, covers every row in both directions, grows the
first row beyond the viewport while preserving its anchor, and checks 60 seconds
of settled idle. At most 32 row computations are active. Both runs verify raw
histogram page ordering/counts and window/resource cleanup. `--background`
requests an unfocused window; this does not prove occlusion or inactivity. Run
one GUI at a time and finish compilation before measuring.

The probe settles for two seconds before beginning each native interval without
requesting redraws. Finish captures before its command's redraw; histogram page
retrieval is outside the interval. Reports retain capture overhead, native draw,
dirty-to-submission, animation submission interval, input-to-frame and inputs per
frame separately. This workload does not generate typing or claim physical
presentation, GPU execution time, IME or accessibility acceptance.

The runner preserves logs and JSON even on failure and reaps its child. Reports
include binary hash, revision/dirty state, the explicitly supplied build profile,
machine/display/power metadata, CPU usage and peak RSS. Retain the preceding build
log: a filename cannot prove optimization. Use a committed source revision for
reproducible acceptance runs; record any source/profile change and new build.

`--check-budgets` enforces the predeclared loaded-list thresholds and sample floor
in [the performance plan](../../docs/design/performance-qualification.md).
One passing run is not release acceptance: the plan also requires repeated runs,
collector overhead comparison, other workloads and physical/GPU evidence.

Portable checks: `python3 scripts/test_measure_list_history.py`.
OCaml codecs: `./scripts/gpuio exec dune runtest examples/performance_probe/ocaml`.
Native probe: `./scripts/gpuio exec cargo test --locked --offline -j2 --manifest-path examples/performance/backend/Cargo.toml -p gpuio-performance-probe --lib`.

The optional `--wall-clock` mode uses monotonic phase intervals without mounting
the native snapshot probe. It retains traversal/growth/cleanup checks but makes no
frame-time or zero-idle-redraw assertion. Use it with the profiled and ordinary
backends for the [paired overhead comparison](../performance_plain/README.md);
the runner refuses frame-budget acceptance in this mode.
