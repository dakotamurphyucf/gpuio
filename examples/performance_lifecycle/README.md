# Window/resource lifecycle measurement

This driver uses public OCaml APIs to open/exercise/close one 1200×800 window at
a time in a single application process. Full mode has three warm-up cycles and
30 measured cycles; smoke has one warm-up and three measured cycles.

Each window owns a streamed Markdown document, a uniquely generated 256×256 PPM
image, a retained rectangle canvas, and the existing qualification extension.
The driver waits for image decode, document publication with exact source bytes,
canvas-command acknowledgement, extension mount and a native render callback.
Even cycles await the extension's delayed operation; odd cycles may close while
that operation is pending. Window-scope cancellation retires registrations.

After acknowledged retirement, the driver drops the model-held registrations,
waits two seconds, verifies zero owned registrations/pending requests/queued work
and the original one-scope/one-worker counts, and emits a numbered checkpoint.
The collector samples the waiting child's RSS with `ps` before acknowledging the
checkpoint on stdin. The next window cannot allocate before the sample. Neither
side forces GC, purges caches, changes process priorities or raises memory limits.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_lifecycle/main.exe
python3 scripts/test_measure_resource_lifecycle.py
python3 scripts/measure_resource_lifecycle.py --build-profile release --smoke --output scratch/lifecycle-smoke
python3 scripts/measure_resource_lifecycle.py --build-profile release --check-budgets --output scratch/lifecycle-full-1
```

Run one owned GUI workload at a time after compilation ends. `--background`
requests unfocused windows; it does not prove visibility and cannot guarantee
render callbacks while occluded. The standalone executable requires its collector
because it waits for exact checkpoint acknowledgements; missing input times out.

The [predeclared target](../../docs/design/performance-qualification.md) is at most
64 MiB baseline growth in the final ten measured cycles. The report keeps every
warm-up and measured sample. Growth is the maximum increase from the first of
those final ten baselines, including transient later peaks, even if the final
sample falls again. It also records the last value and full range. Three full
runs remain required; smoke never satisfies the budget gate. Whole-process peak
RSS and CPU come from exact-child `wait4`, separately from checkpoint RSS.

These counters are application-side registrations and retirement acknowledgements,
not a census of every GPUI entity, decoded cache, GPU allocation or physical memory
footprint. Native ownership tests and physical/GPU observations remain separate
qualification evidence. Resource admission and a render callback do not establish
pixel correctness or document parser completion. The extension enables the optional
profiler; this is not the ordinary-backend overhead comparison.

Reports preserve exact binary hash, checkout metadata, hardware/display/power
information, failed logs and partial samples. Associate them with a preceding
recorded source build; checkout metadata alone is not embedded binary provenance.
