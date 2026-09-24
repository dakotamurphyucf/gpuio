# Bonsai/Eio runtime (OCH-9)

`gpuio.eio` provides `Gpuio_eio.App`, `Scope` and `Stream`. Pure views, styles and
Bonsai components remain independent of Eio. Use `Gpuio_bonsai.View` with
`Bonsai.Cont`, and pass a component factory to `App.open_window`; see the complete
[two-window example](../../examples/runtime/main.ml). No Async runtime is required.
The native scheduling policy uses the original clock interface; the additive
lifecycle and action-history driver extensions below support managed lists.

## Ownership and scheduling

Call `App.run` on the OS main domain. GPUI owns that thread; the runner starts
one OCaml UI domain with Eio. Initialization, graph construction, all window
drivers, shared Bonsai variables and effect callbacks live on that UI domain.
Public runtime operations check domain ownership. CPU-intensive tasks may use
Eio's domain manager with immutable inputs, then deliver results through their
scope; worker domains must not access Bonsai or runtime handles.

Every window has a Bonsai driver, monotonically advancing time source and keyed
reconciler. The loop drains native events and bounded completion jobs, flushes
actions, observes the result and prepares at most one native transaction per
window. An accepted transaction advances the acknowledged tree and triggers
lifecycle effects. Rejected transactions fail the runner rather than committing
an inconsistent tree. Native backpressure retains the pending transaction.

An unchanged tree still refreshes callback bindings and runs lifecycle effects.
One additional flush processes lifecycle-generated actions immediately; further
lifecycle work waits for another wakeup so unconditional after-display effects
cannot create an unbounded synchronous loop. These are **logical display cycles**:
transaction acceptance and native render observations are not physical screen
presentation acknowledgments.

The loop refreshes clock samples before delivering events and effects, so new
sleeps do not use the preceding frame's sample. Input, task completion and
requested-frame responses wake the loop independently
of the clock. One shared Eio monotonic timer targets 60 Hz by default, configurable
with `App.run ~tick_hz` in [0.01,240]. It sleeps until absolute deadlines, skips
missed intervals and never busy-waits. Each tick samples wall time and advances
all live Bonsai clocks; backward wall-clock corrections clamp until time catches
up. Ordinary `Bonsai.Clock.sleep`, `at` and `approx_now` work. Background windows
are not silently throttled. The timer is canceled when the runtime scope ends.

A timer-only effect can run up to roughly one tick late, plus scheduling and
processing delay. Applications depending on `Clock.now` can produce work each
tick. Static results do not serialize, commit, request frames or force a GPUI
redraw. See [measurements](../evidence/runtime-och9.md). This accepted periodic
strategy supersedes the historical requirement to remove permanent idle polling.

## Task and stream lifetimes

`App.scope` owns application tasks. `Scope.child ~name` creates conversation or
other explicit lifetimes. `App.Window.scope` owns window tasks. Closing a window
cancels that scope and its descendants, while application/conversation tasks can
outlive it. Row visibility has no implicit relationship to a conversation scope;
put generation work in the conversation scope, not in a transient row scope.

`Scope.start ~f ~on_result` starts an Eio producer fiber. Capture required I/O
capabilities in `f`; its result is queued and delivered on the UI loop as a Bonsai
effect. Ordinary producer exceptions become `Or_error` results. External Eio
cancellation remains cancellation. Callback/initialization/graph failures clean
up both runtimes and propagate to the caller with their backtrace. A task must
cooperate with cancellation; a non-yielding computation cannot be forcibly stopped.

Canceling a task or scope suppresses queued delivery, including results already
produced. `Task.is_finished` means the producer fiber finished, not that its
queued callback ran. Cancellation is idempotent. Native close acknowledgments
control slot reuse; generation checks reject stale native identities. Closing
before opening or from a lifecycle callback is supported without reentering the
driver. By default closing the last window exits the app; set
`~exit_on_last_window:false` for background work and call `App.shutdown` explicitly.

`Stream.create ~scope ~capacity ~on_batch` batches values in order into one effect
per scheduler delivery. `push` is for UI-domain producers, never yields, and
returns an error without accepting the value when the scheduler queue or batch
is full, or after closure. Callers
choose how to retry or truncate; loss is not silent. Closing the stream/scope
clears buffered values and suppresses queued delivery. Do not push during graph
construction.

Bounds: 32 live windows, 1024 live scopes, 4096 scoped cleanup registrations,
1024 queued UI jobs, default 1024 live producer tasks (configurable 1..65536),
1..4096 buffered values per stream, and one pending explicit frame request per
open window. Task and stream capacities count items, not bytes; applications
must bound payload sizes. Native protocol memory/byte bounds remain separate.

`Window.request_frame` is explicit; use it after activation when the native window
is open. Routine clock ticks do not call it. Closing the window drops its pending
callback. `Stats.rendered` counts delivered/coalesced native render observations,
not GPU submissions or physical frames. `completed_jobs` includes stream batches.

## Validation

`./scripts/gpuio test` runs deterministic expect tests for lifecycle-generated
actions, unchanged-tree transitions, callback refresh, revisions, ordinary clocks,
wrong-domain rejection, queued-completion cancellation, task bounds, ordered
streams and an idle completion wake with no periodic timer. The native runtime
example's `--self-test`, `--shutdown-test` and `--last-window-test` exercise real
windows and the public runner. CI requires those on macOS; Linux builds/tests
remain required and Linux GUI results are informational under OCH-17.


## Lifecycle snapshots for asynchronous acceptance (OCH-13)

Native acceptance is asynchronous. The submitted view and its lifecycle collection
must come from the same stabilization. Buffering a window's actions while its
transaction is pending avoids unnecessary work, but does not freeze its observers:
flushing another Bonsai driver stabilizes the shared Incremental universe.

The pinned Bonsai driver therefore has a small GPUIO extension:
`Bonsai_driver.Expert.snapshot_lifecycles` captures a typed, single-use
`Lifecycle_snapshot.t` alongside the prepared result. Only acceptance triggers
that snapshot; rejection/retention retry drops it. Triggering diffs it against the
last displayed collection using Bonsai's ordinary lifecycle implementation. No
model-reset, Incremental or clock semantics change. Immediate display paths keep
the existing `trigger_lifecycles` API. A snapshot retains its originating driver,
so it cannot accidentally target another window, and a second trigger is rejected.

The regression test changes an external source while a native commit is pending,
then flushes a second driver before acknowledging the first. The first accepted
snapshot runs the original after-display closure; the later accepted transaction
runs the new closure. The pre-extension implementation failed this test. The
managed-row retry test also verifies that discarding a candidate causes no row
reset/deactivation. Patch bytes and digest are recorded with the existing Bonsai
vendor provenance; this is an adapter extension, not a claim of an upstream bug.

## Native action-history lifetime (OCH-13)

Window drivers select `Bonsai_driver.Action_history.Release_after_flush`. Bonsai
tracks action paths through the entire batch, then the driver drops that cache
after stabilization. This avoids retaining paths for every recently visited
virtual row while leaving within-batch dependency decisions intact. The additive
option defaults to upstream `Keep_recent`; it changes cache lifetime, not row
reset semantics. See the [managed-list memory evidence](managed-lists.md).
