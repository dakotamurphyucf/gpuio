# How `main.ml` measures publication, preparation, and rendering

[README](README.md) · [Source](main.ml) · [Chart registration](../../lib/eio/chart.mli)

This is a finite graphical measurement workload. It runs immediately, closes on
completion, and has no ordinary interactive mode or `--self-test` flag. It does
not stream network data. Its measurements distinguish validated source data,
native publication, prepared geometry, and render callbacks.

## Dataset and graph

`B` abbreviates `Bonsai.Cont`, `V` is `Gpuio_bonsai.View`, and `Registered` is
`Gpuio_eio.Chart`, the scoped dataset-registration adapter. `E` abbreviates
`Bonsai.Effect` for deferred UI operations.

`dataset count phase` constructs a line with one stable series ID and positive
point IDs `i + 1`. X increases with `i`; Y is a sine value wrapped in `Some`.
Validated Point/Series/line constructors reject invalid domains/identities/bounds
before native registration. Updates preserve identities while changing values.
See [data contract](../../lib/core/chart_data.mli).

`B.Expert.Var` owns optional borrowed handle and exact-policy flag.
`component` opens `B.Let_syntax` and uses `let%arr` to derive preparing text or
`V.chart` from their current values. `and` binds reactive dependencies rather
than threads. The view has an 800 × 400 logical-pixel chart within an
820 × 440 foreground window. `Chart.Config.create` borrows the registered data;
it does not upload it. Default line sampling uses explicit envelope reduction;
exact mode retains every point. See [sampling](../../lib/core/chart_sampling.mli).

`on_event` ignores Selection_changed and returns a thunk recording Ready/Failed
observations. These diagnostic refs are not a selection model. Ready metrics
mean native preparation, not physical presentation. Hover/paint stay native and
no per-frame OCaml model update is used.

## Window-owned producer and publication trace

`App.run` owns GPUI on the OS thread and an Eio UI domain. `Scope.start` uses the
window scope, so closing the window cancels the workload/registration. The task
has a 180-second Eio timeout. `on_ui` enqueues/handles an effect and resolves an
Eio promise; `ui` wraps a thunk. Mutations of Bonsai Vars and registration stay
on the owning UI domain. Dataset construction occurs in the producer, outside
graph derivation.

The task creates 10,000 points with `Registered.create`, then publishes its
borrowed handle to Bonsai. `wait_ready` requires matching data revision, no
registration/native failure, and `Registered.is_published`. It separately awaits
`App.Window.request_frame` through a promise. Publication means accepted data;
Ready means prepared geometry; the frame means a render callback. None certifies
visible presentation to a person.

The four stages run 30 updates at 10,000 points, 30 at 100,000, 10 exact updates
at 100,000, then 10 bursts of eight desired 100,000-point datasets. Each burst
constructs all eight first and submits all `Registered.set` calls in one UI
thunk. The source scheduler coalesces not-yet-uploading desires to the last value;
an in-flight upload still completes before a later desired snapshot. The workload
expects one new publication revision per burst and validates accepted data equals
the final desired dataset. It never silently reduces source data to make exact
rendering fit; a native render limit would fail the workload.

## Metrics and cleanup

Monotonic elapsed times separate construction, publication, Ready, and frame
latency. `App.diagnostics` samples chart source charges, pending requests, queued
commands, and bridge traffic without an extra message. The workload checks source
count; exact mode also checks retained count. Metrics report representatives,
vertices, quads, and retained plan bytes, not process RSS or GPU memory.

After 80 samples/150 desired updates, it releases registration, clears the view
handle, and waits for registry count and source charge to return to zero. It
checks sampled peak source charge stays within 128 MiB and logs `CHART_STREAM_OK`.
The result effect unwraps errors and shuts down the app; the final assertion
requires completion. Registry cleanup is distinct from independently tested
native cache/worker reclamation.

## Commands and measurement limits

From the repository root using [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/chart_stream/main.exe
_build/default/examples/chart_stream/main.exe
python3 scripts/measure_chart_stream.py --output scratch/chart-stream
```

The measurement script uses the already-built binary, defaults to a 200-second
child limit, collects CPU/peak RSS with `wait4`, writes log/JSON/environment
metadata, and terminates/reaps its child on failure. Both paths need a graphical
session; finish builds and avoid concurrent workloads for meaningful comparisons.
Read the README for exact accounting distinctions. Process RSS includes retained
burst inputs and allocator high-water state; source charges and plan bytes do
not substitute for it. Frame timing is not an idle FPS or physical presentation
benchmark, and these command listings claim no measurements were run.

For a real producer, bound data and payload construction, use explicit Eio I/O
capabilities, retain registration in the intended scope, handle publication errors,
and preserve stable datum identities. Observe data revision when consuming chart
events; never apply an old selection position to a newer dataset. Keep source
publication work outside `let%arr` and qualify backend acceptance separately.
