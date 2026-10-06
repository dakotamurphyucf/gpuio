# How `main.ml` runs the typed view API through a low-level bridge

[README](README.md) · [Source](main.ml) · [Components](components.md)

This executable deliberately owns its bridge loop. It does not mount
[`Bonsai_component.component`](bonsai_component.md), use `Gpuio_eio.App.run`, or
construct a Bonsai graph. Ordinary applications should use the public runner;
this code illustrates the acknowledgement protocol beneath it.

`Action.t` is Increment or Change_theme. The worker owns mutable count/theme
state, a `Reconciler`, and flags for dirty/open/stopped plus one pending update.
`window` is an explicit protocol ID with slot 0 and generation 1. Stable IDs and
native acknowledgements are runtime responsibilities here, not application
labels. See [reconciler](../../lib/core/reconciler.mli).

`pump` runs only after opening, while dirty, and with no update in flight.
It calls `Components.app` with current values and action-producing callbacks,
then `Reconciler.prepare ~theme (Some view)`. Preparation does not publish
callbacks. If there is no wire message, it accepts immediately, including refreshed
callbacks. Otherwise it submits the message and retains the pending update until
native `Accepted` arrives. Only then does `Reconciler.accept` publish it.

A native button event reaches `Reconciler.dispatch`, resolving callbacks from
accepted state. Increment changes the worker count; Change_theme toggles the
palette; both mark dirty. `pump` derives a new immutable view and submits its
diff. Hover/focus/selection/paint remain native; the worker does not receive a
per-paint callback or duplicate native editor state.

The event loop submits Hello, opens a 760 × 520 window after Welcome, drains
events, pumps, and reads a dedicated wake pipe with `Eio.Flow.single_read` when
idle. It handles close/quit by submitting protocol commands. Rejected/Failed/
Overloaded abort the example; many unrelated event families are ignored because
this small fixture mounts none of their owners.

The outer `Eio.Switch` owns the pipe. `Gpuio_native.create` duplicates its write
end; the OCaml original is closed afterward. The worker runs in `Domain.spawn`,
while `Gpuio_native.run` owns the OS main thread. Worker failure invokes emergency
`abort` and preserves its backtrace. The main thread captures its own failure,
joins the worker, disposes the native handle, then raises captured errors. Keep
the read end alive until both runtime and worker finish. See
[native contract](../../lib/native/gpuio_native.mli).

From the repository root using [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/view_api/main.exe
_build/default/examples/view_api/main.exe
_build/default/examples/view_api/main.exe --self-test
```

The graphical self-test has a 30-second worker timeout. After each Accepted it
requests a frame; the frame event increments count and changes theme every fourth
increment until revision 20, then submits Shutdown. It prints `TYPED_VIEW_PASS`.
The marker explicitly reports `native_selection=not_exercised`: this sequence does not generate
selection or physical button/keyboard/clipboard input. Those native interactions
need separate tests. A frame is not physical presentation evidence.

When adapting application content, keep reusable views in ordinary functions or
Bonsai components. Prefer the public App runner for scheduling, IDs, close policy,
and cleanup. If studying this runtime loop, preserve one in-flight update and
accept-after-acknowledgement; publishing prepared callbacks early breaks correlation.
