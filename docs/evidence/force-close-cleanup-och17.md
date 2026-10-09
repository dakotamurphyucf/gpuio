# Force-close cleanup and native request delivery — OCH-17

On 2026-10-08, review of the public application/window lifecycle found an exception
path that could leave a native close request unqueued. Base revision: `f1208273`.
This extends the earlier [scope cleanup](scope-cancellation-och17.md) and
[application teardown](application-teardown-och17.md) repairs.

## Problem and change

`Window.close` first marked a window as closing, then completed close-decision
waiters and cancelled its scope. If either step raised, it skipped the native
close command for an open window, or skipped the runtime wakeup for a window
still opening. A decision-completion failure also skipped scope cancellation.
Repeated close calls returned immediately because the phase was already closing.
Similarly, `App.shutdown` marked the application stopping before cleanup; a
cleanup failure could skip its `Shutdown` command, with later calls returning
without retrying it.

Both paths now use the existing attempt-all cleanup helper. They retain logical
closing/stopping before callbacks, attempt the remaining independent cleanup,
queue the close/shutdown request (or wake the opening-window path), then re-raise
the first failure with its original backtrace. A late opening acknowledgment
still causes the native close request through the normal state machine.
Repeated and reentrant calls remain idempotent.

This queues native work; it does not synchronously destroy an OS window or prove
the native event loop has stopped. Scope cleanup callbacks still must not raise,
block or perform I/O. The change contains an erroneous callback without treating
its operation as successful.

## Regression evidence

Before the repair, the new regressions showed:

- An open window had no queued `Close` after a scope-cleanup failure.
- A decision-completion failure left the window scope active; the open-window
  path also had no `Close`. In the opening-window case, its late acknowledgment
  could encounter the still-pending scope failure and again skip the command.
- `App.shutdown` ran the registered cleanups but queued zero `Shutdown` requests.

After the repair, the four combinations of opening/open window and failing
close-decision/scope callback verify first-exception identity, both cleanup calls,
inactive window scope, runtime wakeup, cleared waiters, exactly one native close
request after any opening acknowledgment, an empty window map and restored
scope/task/cleanup counts after closure. Reentrant/repeated close calls do not duplicate delivery. Unexpected
errors during delayed opening and closure acknowledgments are checked explicitly.
The shutdown regression verifies reentrancy, both callbacks, preserved first
exception, one shutdown request and zero remaining scope/task/cleanup counts.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @lib/eio/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt
```

The focused Eio tests and full OCaml test/format aliases pass on local macOS
arm64. No expect result was promoted. The first fixture compilation used an
unavailable `Exn.Result` alias; the explicit result type corrected that fixture
before the actual before/after behavior comparison.

These tests use real Eio fibers and native allocation, with injected lifecycle
acknowledgments and inspection of queued protocol requests. They open no desktop
windows and do not establish native dispatch, GUI, performance or Linux acceptance.
Hosted validation of this revision remains pending.

The [archive](force-close-cleanup-och17/reports.tar.gz) and
[manifest](force-close-cleanup-och17/manifest.json) preserve compile/before/after/
full/final logs and exact application implementation/interface sources. The
public contract is in [App](../../lib/eio/app.mli). OCH-17 remains In Progress.
