# How `Bonsai_component` reuses a plain counter

[README](README.md) · [Source](bonsai_component.ml) · [Shared views](components.md)

This module is compiled into `main.exe` but is not called by its entry point.
The executable's active path is the [low-level bridge loop](main.md). The module
shows the type/syntax boundary between ordinary view functions and Bonsai;
building it does not establish interactive Bonsai acceptance.

`module Bonsai = Bonsai.Cont` selects the continuation graph API.
`component graph` calls `Bonsai.state 0 graph`, obtaining a reactive integer and
reactive state-setter function. The graph owns this state for its mounted
computation lifetime. There is no global reference or native-owned counter value.

Opening `Bonsai.Let_syntax` enables `let%arr`. It reads current `value` and
`set_value` and derives `Components.counter`. The `and` syntax binds reactive
dependencies; it does not launch threads. `on_increment` is an ordinary callback
`fun () -> set_value (value + 1)` returning an effect. Rendering constructs it;
a subsequent native event runs it through the runner's effect scheduler.
The model changes, Bonsai reevaluates the counter, and GPUIO submits new text.
Native focus/hover/press state remains in the retained button owner.

`reset on_reset` demonstrates `Gpuio_bonsai.View.button`, whose `on_click` takes
an effect directly. It is just a view helper: it neither owns state nor sets this
counter to zero. Its caller must supply a reset effect such as the appropriate
state setter. Neither `component` nor `reset` opens a window.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/view_api/main.exe
```

[Dune](dune) includes this module and enables Jane Street and Bonsai PPX.
Use the [repository environment](../../docs/development.md). There is no separate
executable/test flag for this sample. Running `main.exe` launches the bridge
example, not this component. To make it interactive, mount it from an
`App.open_window` factory under `Gpuio_eio.App.run` as in
[getting started](../getting_started/README.md), adapting the factory arguments.
The runner owns graph activation, event/effect delivery, native lifetime, and
shutdown; this function owns only its graph state and derived counter content.

When composing multiple counters, create separate graph state nodes rather than
sharing a mutable ref accidentally. Keep side effects in returned effects and
preserve the shared view's stable keys during updates.
