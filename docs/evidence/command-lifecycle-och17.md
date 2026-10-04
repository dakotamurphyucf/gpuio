# Pending command lifecycle — OCH-17

Local checkpoint, 2026-10-01. These checks exercise the production OCaml application
request registry, actual Eio fibers/promises and native transport allocation and
submission. They **do not run the GPUI event loop or open a desktop window**.
Opened/Closed/editor-result events are injected at the application dispatcher;
physical input, native command execution and end-to-end window shutdown remain
separate acceptance requirements.

## Reproduced failure and repair

An editor-command completion that raised during `release_window` stopped cleanup
before the remaining editor callbacks, scope cancellation, Bonsai driver disposal
and window-registry removal. The regression observed one uncalled completion,
one registered window, an active scope and a retained driver after the exception.
Those pending editor entries had already been removed, so subsequent cleanup could
not recover their callbacks.

Application teardown now attempts each independent completion and cleanup step,
then re-raises the first exception with its captured raw backtrace. Close-decision
waiters, per-window request completion and whole-runtime window disposal use that
policy. A driver is detached before its disposal callback can raise. Runtime
cleanup continues through other windows and closes the scheduler inbox even when
one completion fails. Exceptions are propagated rather than converted into success.

The application-record constructor and runtime finalizer are private helpers used
by both the production worker and inline tests. No public API or test-only window
constructor was added. The tests build an actual picker View through Bonsai and
obtain its query node identity from the real reconciler transaction.

## Executed checks

Five inline expect tests cover:

- Commands admitted before closing may complete before the close acknowledgment;
  new commands return `Closed`. Wrong-node and late/duplicate replies do not invoke
  another completion. Two actual Eio waiters finish exactly once.
- Window-slot reuse increments identity; old snapshots return `Stale_editor`.
  Runtime disposal completes remaining waiters with `Closed`.
- The 64-request limit is shared across windows. Closing one releases only its
  pending requests, allowing another request on the surviving window.
- A raising completion does not strand another command, retain the window driver
  or leave its scope active; the exception still propagates.
- A failure in one window does not prevent another window's waiter from finishing.
  Runtime disposal empties window/request/command collections, closes the inbox
  and remains idempotent afterward.

The first three tests passed before the repair. The fourth failed with
`(true 1 0 1 true false)` instead of `(true 1 1 0 false true)` and a `Closed` result;
it passed after repair. Scratch logs:
`scratch/agents/root-20260929-m7-resumed/command-lifecycle-exception-repro.log` and
`command-lifecycle-tests-fixed.log`.

The focused command is `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2
@lib/eio/runtest`. Broader final validation is recorded below when complete.
Native transport creation only duplicates the wake descriptor and allocates
queues; the default backend initializer is empty. Tests dispose each native
handle and keep its Eio pipe alive until disposal. No OS input/accessibility,
Linux runtime, retained-process RSS or visual claim follows from these checks.


Final broader validation: `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2
@runtest @fmt examples/gallery/main.exe` passes, including the fifth cross-window
regression (`command-lifecycle-dune-final.log`). This rebuilds the public gallery
with the current native row-layout repair as well. It does not launch that gallery.
