# Await a UI effect from an Eio task

[ui_thread.ml](ui_thread.ml) and [ui_thread.mli](ui_thread.mli) expose one function:
`perform : Scope.t -> 'a Bonsai.Effect.t -> 'a`. This is a specialized startup and
integration-test adapter, not a Bonsai component or pure model calculation.
Read the entire short implementation alongside
[Scope](../../lib/eio/scope.mli) and its [implementation](../../lib/eio/scope.ml).

From the repository root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --self-test
```

The last command runs optional native checks, not just pure expectations. See the
[README](README.md#local-checks) for platform coverage and physical-input harnesses.
These are documented commands, not newly executed evidence.

## Promise, queued handler and completion

perform creates an Eio.Promise and resolver, then Scope.Expert.enqueue schedules
a UI job. The job maps the supplied effect's result to Promise.resolve and starts
it with Effect.Expert.handle. The Eio caller awaits the promise and returns the
same result type. Effect.map transforms an eventual result; it does not eagerly
execute the effect. The promise connects the later UI completion to the waiting
Eio fiber. Await cooperatively suspends that fiber while the UI loop must continue
processing jobs and native events.

[Application](application.ml) calls this from an application-scoped Scope.start
startup task. Its on_ui helper wraps native Scene.create/Chart.create and other
effects; a ui helper wraps UI state mutations in Effect.of_thunk before forwarding
them. The trace is Eio startup fiber → queued UI handler → native resource-create
effect → reply resolves promise → startup fiber adopts the returned handle on
UI domain. Startup waits for a window snapshot and announces desktop readiness;
that is not proof of first chart paint. Resources belong to the application's
scope and survive window close, not to this promise or a responsive view branch.

[checks.ml](checks.ml) uses the same boundary from its Eio test task to request
UI/native operations, then waits for explicit observations. Ordinary
[Component](component.ml) instead uses let%arr on observable snapshots to derive
Ui.view. It needs no waiting adapter. A reactive value represents changing data;
a Bonsai effect represents a later action; an Eio promise carries one completion.

## Preconditions and lifetime limits

Do not call perform inside a Bonsai effect handler or view construction: waiting
for work queued to the same UI loop can prevent its completion. Scope's expert
queue is UI-domain owned and checks that domain; this is not an arbitrary-thread
marshalling API. Scope.Expert.enqueue skips inactive scopes. perform itself has
no closed-scope Result, exception reply, cancellation token or timeout, and only
resolves when the mapped effect completes. An unscoped call with a closed scope
could therefore wait indefinitely.

The actual application caller wraps startup/check work in an active application
Scope.start and Eio.Time.with_timeout_exn (60 seconds, or 150 for its workload),
so caller cancellation/timeout bounds the waiting fiber. Preserve that ownership
when adapting the example. Cancellation of a waiter does not by itself establish
that a native operation never ran; each resource/effect retains its own contract.
This small module creates no native handle or filesystem capability of its own.

For a new startup operation, add a scoped Eio call through on_ui and explicitly
adopt/release its returned resource as Application does. For button actions, keep
ordinary deferred effects in the UI action flow instead of awaiting them here.
There is no independent pure adapter test in this directory; its exercised
boundary is the optional application's native startup/integration path. Compilation
alone cannot establish callback delivery, cancellation or physical rendering.
