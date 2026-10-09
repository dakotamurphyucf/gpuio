# How `bench_input.ml` measures the OCaml window driver

[README](README.md) · [Source](bench_input.ml)
· [Driver interface](../../lib/runtime_core/window_driver.mli)

This benchmark runs without a native handle, OS window, or graphical event loop.
It measures internal driver scheduling, Bonsai computation, reconciliation,
and simulated transaction acknowledgement. It does not measure actual user input,
FFI transport, native dispatch/layout/rendering, or screen latency.

`B` abbreviates `Bonsai.Cont`; `Driver` is the internal `Window_driver`.
`component graph` installs integer state 0, then `let%arr` reads state/setter to
build one button. The setter effect describes the next state; it runs only when
a dispatched Press resolves that callback. `and` binds reactive dependencies,
not concurrent tasks. Native button state is absent in this benchmark.

For window counts 1 and 4, the program validates explicit slot/generation IDs and
creates drivers with current time, default theme, and the component. Each initial
`Driver.cycle` derives an Apply message. Local `apply` calls `submitted` and
`acknowledge` immediately with its revision, simulating successful native acceptance.
It finds the created Button's node/handler IDs in that transaction.

Each of 100 samples refreshes the first driver's clock, dispatches a synthetic
correlated Press at its acknowledged revision, cycles **all** drivers, and applies
their messages locally. Exactly one must produce a transaction: the target's
effect changes model, Bonsai derives new text, and the reconciler emits its diff;
unchanged windows remain quiet. Timing covers that whole OCaml sequence, including
cycling unrelated drivers, rather than only callback execution.

Sorted samples report index 50 as median, index 95 as p95, and the maximum in
milliseconds. These are fixed sample positions, not a statistical confidence
interval. Drivers are closed after sampling to retire graphs/lifecycles.
`Eio_main` supplies real/monotonic clocks and stdout, but there is no App scope
or asynchronous native producer. This module uses an internal API deliberately.

From the repository root in the [configured environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/runtime/bench_input.exe
_build/default/examples/runtime/bench_input.exe
```

No flags or graphical session are required by this source path. It always measures
both 1 and 4 drivers and prints `INPUT_CORE_MEASURE`; there is no native keyboard
harness. The benchmark cannot establish end-to-end input responsiveness or Linux
GUI acceptance. For actual application behavior use [runtime main](main.md), and
for native idle/clock counters use [bench](bench.md). Keep internal-driver examples
as diagnostic code rather than substituting manual acknowledgements into an app.
