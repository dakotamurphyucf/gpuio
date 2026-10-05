# Ordinary-backend performance comparison

This executable compiles the exact OCaml workload from `../performance/main.ml`
through a Dune copy rule, using `gpuio.native.default`. It does not link the
qualification probe's Rust package or enable GPUI's optional profiler. The OCaml
probe codec is pure and contains no native registration.

Run this executable with `--wall-clock`. That mode renders a 1px placeholder in
place of the measurement extension, uses Eio's monotonic clock for phase timing,
and preserves the same row data, full traversal, frame acknowledgments, growth,
60-second idle interval and cleanup checks. It does **not** collect native frame
histograms or prove idle redraw counts. Omitting the flag fails waiting for the
unregistered native probe; it cannot accidentally claim measurement acceptance.

For a profiler-overhead comparison, build both executables with the release
profile, then run **both** in wall-clock mode. The instrumented backend retains
GPUI's profiler feature, while this ordinary backend does not. This isolates
ongoing profiler work from snapshot collection and bucket-page traffic. Compare
phase time, CPU usage and peak RSS over repeated runs; the original histogram
mode separately qualifies draw timing and settled idle.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release \
  examples/performance/main.exe examples/performance_plain/main.exe
python3 scripts/measure_list_history.py --build-profile release --wall-clock \
  --executable _build/default/examples/performance/main.exe \
  --output scratch/performance-profiled-wall-001
python3 scripts/measure_list_history.py --build-profile release --wall-clock \
  --executable _build/default/examples/performance_plain/main.exe \
  --output scratch/performance-plain-wall-001
```

Use `--smoke` for functional validation before full runs. Stop compilation and
other GPUIO GUI workloads before measurement. `--check-budgets` intentionally
rejects wall-clock mode: elapsed time is not a frame histogram. Record exact
executable hashes, feature-graph verification and every repeat, including failures.
Implementation and successful compilation alone are not overhead evidence.
