# Getting started

A small application using installed public Core/Bonsai/Eio APIs and the default
native backend. Increment/Reset update Bonsai state; Close requests normal window
closure. The complete implementation is [main.ml](main.ml), with dependencies in
[dune](dune).

Read the three named parts of `main.ml` in order:

1. `counter_view` is GPUIO layout: ordinary values and effects in, a view out.
2. `component` is Bonsai: `state` allocates the counter once; `let%arr` computes a
   new view from current values. Button effects request state changes.
3. The entry point uses `App.run` and `App.open_window` to own the runtime and mount
   that component. No native handles or Rust code are needed.

The `View`, `Bonsai`, `Effect` and `App` aliases identify these boundaries. This
small example stays in one file; the [gallery](../gallery/README.md#reading-the-code)
shows the same separation using modules as the application grows.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/getting_started/main.exe
_build/default/examples/getting_started/main.exe
```

The second command opens a window. For an independent public-library build without
opening a window:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py \
  --example getting_started --workspace scratch/starter-consumer-001
```

Use a new workspace path for each run. Read the [application guide](../../docs/getting-started.md)
for toolchain setup, ownership and package composition, and
[API compatibility](../../docs/api-compatibility.md) for current limits.
