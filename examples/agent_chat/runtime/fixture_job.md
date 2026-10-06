# Bound CPU fixture production

[fixture_job.ml](fixture_job.ml) and [fixture_job.mli](fixture_job.mli) implement a
small window-scoped job controller: at most one producer runs and at most one
newest replacement waits. It supports the 100,000-source/result demonstrations
without starting another worker domain for every click. It owns mutable scheduling
state but no Bonsai graph, views, fixture payloads after delivery or domain manager.

Use the [isolated toolchain](../../../docs/development.md) from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open **Explore sources** or **Explore results**, choose the 100,000-item fixture,
then restore the sample or change queries while construction is pending. These
fixtures are synthetic: no checkout files, external search or provider request
is involved. macOS is the v1 desktop target; Linux builds/unit checks remain
required while GUI qualification is [informational](../../../docs/platform-release-policy.md).

## Types and the deferred pump

`Request.t` stores a serial, `run : unit -> 'a` and
`complete : 'a Or_error.t -> unit Bonsai.Effect.t`. The controller's abstract
`'a t` stores its owning scope, a mutable serial, `running` Boolean and optional
pending request. It retains only the newest pending closure; callers should avoid
capturing unrelated large state in that closure. Active closures live until
their producer/completion retires. `create ~scope` starts at serial zero with no
running or pending job and registers `cancel` using public `Scope.on_cancel`.
It can fail if cleanup registration cannot be admitted.

`submit t ~f ~on_result` **returns an effect**. Constructing this effect does not
run the calculation. When scheduled on the UI loop, its first `E.of_thunk`
increments the serial and replaces `pending` if the scope is active. `pump` then
examines state in another deferred thunk. If a producer is already running or
the scope has closed, it returns `Effect.Ignore`. Otherwise it removes the
pending request, marks `running = true` and calls `Scope.start`.

`let%bind` in this file belongs to `Bonsai.Effect.Let_syntax`: it sequences these
deferred operations. It is not Bonsai graph-building `let%arr`. Mutation and
`Scope.start` happen outside graph evaluation. The
[Scope interface](../../../lib/eio/scope.mli) runs `f` as an Eio fiber and queues
`on_result` for the UI domain; ordinary producer exceptions become errors, while
external Eio cancellation remains cancellation.

The local completion effect clears `running`, then checks whether its serial
still equals the controller's serial and its scope is active. Only a current
result reaches `request.complete`. Whether delivered or obsolete, completion
then runs `pump` again so the newest pending request can start. If task admission
itself fails, the same completion path handles that error and clears the running
flag. The controller does not silently retry a failed calculation.

## Trace replacement and cancellation

Suppose request A is calculating, then requests B and C are submitted:

1. A's producer keeps running. B becomes the pending replacement and advances
   the serial; C replaces B and advances it again.
2. A completes. Its serial is obsolete, so its payload/error is not delivered to
   the UI callback. The running flag clears.
3. The pump starts C. B never runs. If no newer submit/cancel intervenes, C's
   result reaches its callback.

`cancel t` advances the serial and drops pending work. It deliberately does not
clear `running` or call `Task.cancel`: an already-running pure CPU calculation
must finish before another starts. This retires its delivery rather than
claiming immediate CPU interruption. A new submit after cancel waits for that
producer to drain. Parent scope cancellation is different: the scope cancels its
Eio tasks, invokes the cleanup and suppresses queued completions. A worker-domain
calculation may still need to drain according to the Eio capability's own
cancellation semantics; this helper never starts another replacement in a closed
scope. It is not an interruptible general-purpose task queue.

## Concrete callers and the UI boundary

Read [sources.ml](sources.ml): `create` allocates a fixture job under the window
scope, `build_large` sets the observable busy flag and submits `t.build_large`,
and `reset` calls `cancel_build` before replacing sample data. Repeated build
requests are additionally ignored while its busy flag is true. The successful
callback resets the tree loader; failure clears busy and updates the notice.

In [results.ml](results.ml), `query` cancels pager requests and previous fixture
delivery, captures the current source, then either replaces the small sample
synchronously or submits `t.build source query` for a large query. `accept` resets
the table pager and clears busy. `paged_sample` cancels fixture delivery before
starting the paginated sample path. These callers own reactive state and
controller resets; `Fixture_job` only schedules/adopts the callback.

[Application.run](../application.md) supplies `Eio.Domain_manager.run` closures
for CPU construction. The worker produces pure data and does not mutate Bonsai,
use native handles or own visible rows. Callback effects adopt data on the UI
domain; reactive snapshots then drive native tree/table views. The window scope,
not page visibility or virtualized row lifetime, owns the job. An accepted result
is still distinct from physical table/tree presentation.

A small adaptation is to use this helper for a bounded local index build. Pass a
window-scoped job controller, capture an explicit worker capability in `f`, and
return immutable data. Reset busy/error UI in the callback, and call `cancel`
when resetting the index. Keep one result type per controller, current-serial
checks and one pending replacement. If each request must run, or running work
must be actively interrupted, this newest-wins helper is the wrong scheduling
contract and needs an explicit new design. Existing native fixture checks are
listed in the [README](../README.md); this prose review adds no new workload or
resource acceptance evidence.
