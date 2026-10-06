# Window/resource lifecycle measurement

Read the [launcher implementation walkthrough](main.md) for the shared workload
call, fixed auditing options and acknowledged RSS checkpoints.

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

## Separate macOS physical-memory audit

Add `--physical-memory` to collect `/usr/bin/footprint` JSON and raw
`/usr/bin/vmmap -summary` output at every closed-window checkpoint. The collector
checks the exact owned PID, byte units, all category fields and tool diagnostics
before acknowledging the next cycle. Errors, warnings, timeouts and partial
output are retained; incomplete observations cannot pass. The two tools each
have a six-second timeout within the application's 20-second checkpoint wait.
No privilege escalation or machine-wide setting changes are performed.

This audit intentionally runs separately from responsiveness measurements: OS
inspection can suspend the child and perturb timing. Existing RSS qualification
remains distinct. The report adds all settled physical-footprint values and
final-ten growth/range as observations, with no retroactively chosen physical
pass/fail threshold. Raw category names/accounting, including any GPU/IOSurface
categories exposed by the OS, are retained without calling them a complete
Metal allocation census. Closed-window samples do not establish peak live-window
GPU memory or physical presentation.

The additional `--check-closed-surfaces` flag requires `--physical-memory` and
rejects any closed checkpoint whose IOSurface category retains nonzero bytes or
regions. This regression gate was declared before the full repaired runs, after
the first audit exposed three retained surfaces per closed window. It checks OS
category observations on the reference system, not all Metal resources. See the
[teardown contract](../../docs/design/gpui-macos-adaptation.md).

The shared OCaml workload now lives in `../lifecycle_workload`; this executable
continues to select the ordinary performance backend. The separate
[resource audit](../resource_audit/README.md) runs it with optional native entity
tracking. Do not mix that backend's timings with ordinary performance results.
