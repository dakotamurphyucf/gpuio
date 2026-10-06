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

The model is one OCaml integer, initially zero. `Bonsai.state` allocates it when
the component graph is constructed. `let%arr` reads that state and the setter;
it derives a view whenever those inputs change. Clicking **Increment** delivers
a native button event to its OCaml effect, requests `count + 1`, and causes the
dependent view text to update through GPUIO. Creating `counter_view` alone does
not execute an effect. `Reset` requests zero through the same path.

`counter_view` is a pure presentation function receiving current values and
actions. It uses a column, text, buttons and validated logical-pixel spacing.
The component defines neither a serialized bridge protocol nor a native widget.
There is no editor, virtual list or explicit resource handle requiring stable
application keys in this example.

`App.run` owns the Eio/native runtime. The unused `_env` would provide explicit
I/O capabilities to a larger application; this counter starts no file, network
or timer task and needs no child task scope. `App.open_window` mounts the Bonsai
component in one window. **Close** uses `Effect.of_thunk` to request ordinary
closure when activated; it does not terminate the process during view creation.
Window teardown disposes its component/runtime resources. View acceptance is
separate from the screen physically presenting a frame.

To add a **Decrement** action, give `counter_view` a `~decrement` effect argument,
add another `View.button`, and pass `set_count (count - 1)` from `component`.
Negative counts are valid for this demo; use a reducer with an explicit invariant
if your application's count must stay nonnegative. Keep state allocation outside
the pure view helper and avoid adding I/O inside `let%arr`. For actions from many
asynchronous producers, use a state machine that applies actions to the current
model rather than carrying a previously observed count in delayed work.

The example uses stock OCaml/Core and Bonsai v0.17. The current release is
macOS-first; Linux compilation/consumer checks and actual Linux desktop
qualification are distinct. See the [platform policy](../../docs/platform-release-policy.md).
