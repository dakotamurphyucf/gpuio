# How `bench.ml` measures static and clock-dependent windows

[README](README.md) · [Source](bench.ml) · [Dune](dune)

This is a short graphical measurement executable, not an interactive app or
acceptance test for physical input. `B` abbreviates `Bonsai.Cont`, `E` is
`Bonsai.Effect`, and `View` is `Gpuio_bonsai.View`.

`component` constructs 50 keyed static text rows. Without `--clock`, `B.return`
returns that constant view. With `--clock`, `B.Clock.approx_now` adds a one-second
reactive dependency; `let%arr` derives timestamp text plus the static subtree.
The graph owns clock observation, while native layout/painting owns each window.
There are no input callbacks or changing application model beyond observed time.

The first argument is parsed as window count (default 1), restricted to 1–8.
It must come before flags; `bench.exe --clock` tries to parse `--clock` as an
integer. `--hz=` sets App tick frequency (default 60), validated by the runtime.
`App.run` opens that many 400 × 400 windows. An app-scoped task waits up to
15 seconds for initial commits, then requests/awaits each native frame with a
5-second bound and allows another 250 ms settling period.

Only then does it snapshot runtime counters, `Stdlib.Sys.time`,
`Gc.allocated_bytes`, and monotonic time. After a three-second monotonic sleep,
it computes deltas and prints `RUNTIME_MEASURE`. Static mode asserts commit count
did not change; the shared clock can still tick. Clock mode permits timestamp
updates. The result callback unwraps task failure; success shuts down the app.

CPU percent is process CPU seconds divided by measured elapsed seconds, scaled
to one core; multicore activity can exceed 100%. Allocation counts are OCaml
allocation, not native allocations, RSS, or GPU memory. Turns/ticks/commits/render
notifications are runtime counters, not frame-rate or physical presentation
measurements. No native input latency is measured. Occlusion can defer the frame
warmup, and concurrent workloads affect comparisons.

From the repository root in the [repository environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/runtime/bench.exe
_build/default/examples/runtime/bench.exe 1
_build/default/examples/runtime/bench.exe 4 --clock
_build/default/examples/runtime/bench.exe 4 --hz=30
```

Use a graphical session/native backend and finish compilation before comparing
runs. This source has bounded warmup/frame waits but no external whole-process
watchdog. Commands above claim no measurements were run. To adapt the benchmark,
state exactly which dependency changes and retain warmup/measurement separation;
never interpret no commits as no process/native work or as platform acceptance.
