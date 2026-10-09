# How `main.ml` checks the current bridge transaction protocol

[README](README.md) · [Source](main.ml) · [Native boundary](../../lib/native/gpuio_native.mli)

This executable runs a finite low-level graphical diagnostic on every launch.
It uses the current `Gpuio_protocol.Wire`, unlike the foundation experiment's
private schema. It creates no Bonsai graph, reducer, or public `View` tree;
application-like refs drive explicit wire operations. Ordinary applications should
use the public [App runner](../../lib/eio/app.mli).

`id` and `window` validate explicit slot/generation identities. `worker` owns
accepted/painted revision counters and stop/rollback/containment flags.
`send` submits a typed message and converts admission error codes to `Or_error`.
`apply` sets next revision to base plus one. `initial` creates a container/text,
sets style, splices the child, and sets root; it describes a transaction rather
than making synchronous native widget calls.

Hello negotiates protocol version/capabilities. On Welcome, the script tries to
dispose a running handle and requires a contained `Failure` without poisoning
it. It opens a 640 × 480 primary window, then a 400 × 240 independent window;
after both Opened responses it submits their initial transactions.

Accepted checks increasing primary revisions and requests a correlated frame.
Rendered verifies the observed revision is not ahead of acceptance.
Frame_requested checks correlation/revision: it closes the secondary window,
and advances the primary through 50 acknowledged frames. After frame 1 it sends
an intentionally invalid transaction containing changed text plus a cycle-making
splice. Rejected Invalid_tree must leave revision 1 usable; a valid replacement
with the same base proves rollback preserved accepted state. Later updates change
text, with no native input event required.

The model→native trace here is a diagnostic counter/event → explicit Apply →
Accepted → requested frame → next scripted counter/update. There is no Bonsai
effect scheduler between these steps. The worker drains native events and waits
on a dedicated wake pipe, under a 30-second Eio timeout. Shutdown requires both
primary frame 50 and secondary closure, then Stopped asserts rollback and
containment. Success prints `PRODUCTION_BRIDGE_PASS`.

The outer switch owns the wake pipe. `Gpuio_native.create` duplicates the write
descriptor; OCaml closes its original. An OCaml domain runs the Eio worker while
`Gpuio_native.run` owns the OS main thread. Worker failure calls emergency abort
and preserves its backtrace. The caller waits for native return, joins the worker,
disposes, and raises captured failures. Only that ordering makes normal disposal
safe; the deliberate mid-run disposal probe is a rejection test.

From the repository root using [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/bridge/main.exe
_build/default/examples/bridge/main.exe
```

There is no self-test flag or ordinary interactive mode. A graphical backend is
required, and manual close can disrupt the scenario. The marker's
`panic_contained=true` refers to this invalid-disposal rejection; the source does
not deliberately generate a Rust panic. The sequence checks transaction/frame
ordering, rollback, two-window independence, and cleanup, not physical input,
GPU pixels, or [Linux GUI acceptance](../../docs/platform-release-policy.md).

When adapting runtime code, preserve generation identities, acknowledgement
ordering, and cleanup after joining. For application content, avoid hand-built
wire transactions and use the typed View/Bonsai/App layer instead.
