# How `Support` bridges diagnostic effects into the UI scheduler

[README](README.md) · [Implementation](support.ml) · [Interface](support.mli)

The filesystem, outline, and lifecycle scenarios call this small helper. It owns
no application, graph, native view, filesystem capability, or long-lived model.
The caller supplies the scope whose lifetime should own each task.

`start scope f` calls `Gpuio_eio.Scope.start` with an Eio producer `f`. Its
`on_result` returns `E.of_thunk` that unwraps `Or_error`; failures raise at the UI
effect boundary. The returned `Scope.Task.t` is deliberately discarded, because
these callers use scope lifetime rather than an individual cancellation handle.
Scope cancellation retires owned work and queued completion delivery.

`perform scope ui_effect` creates an Eio promise/resolver, then starts a trivial
producer returning `()`. Its completion callback checks success and returns
`E.map ui_effect ~f:(Eio.Promise.resolve resolver)`. Running that effect in the
UI scheduler executes the supplied action and resolves the promise when the
action returns. `Eio.Promise.await` yields the diagnostic fiber until then.
This is an effect-entry bridge, not a new worker thread that mutates Bonsai.

`E` abbreviates `Bonsai.Effect`. An effect describes deferred work; constructing
it is not the same as running it. `E.map` transforms its eventual result and
`E.of_thunk` defers a synchronous mutation. Neither helper uses `let%arr` or a
Bonsai graph; the callers use them around effects derived from their graphs.
See [scope contract](../../lib/eio/scope.mli) and [runtime](../../lib/eio/app.mli).

For example, `Filesystem_demo` captures a tree target, constructs
`Tree.Controller.set_expanded ... true`, and passes it to `perform`. Completion
means that effect was processed. The script must separately await the loader
snapshot and native viewport observation. It does not mean a page loaded or a
frame appeared. A retired target may legitimately make a controller effect do
nothing while still completing the bridge.

`perform` has no internal timeout and requires an active scheduler/scope.
A cancelled scope may suppress the callback that would resolve its promise.
The surrounding scripts and external harnesses provide bounded diagnostics;
this helper is not a general blocking API to invoke during graph evaluation or
from a synchronous native callback. For production work, prefer ordinary event
or producer-completion effects and explicit cancellation/result handling.

Build and run a caller from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/tree/main.exe
_build/default/examples/tree/main.exe --outline --self-test
```

Use the [configured toolchain](../../docs/development.md) and graphical session.
There is no `support.exe` or separate test flag. Its exercised behavior belongs
to the callers' scripted native scenarios; those checks do not establish physical
input or Linux graphical acceptance. Preserve caller ownership when adapting a
bridge and place deadlines around the overall scenario, including awaited effects.
