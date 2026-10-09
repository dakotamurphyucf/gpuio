# Native diagnostics and controlled races

[checks.ml](checks.ml) and [checks.mli](checks.mli) are for example maintainers and
integration diagnostics. They do not build views or implement ordinary startup.
Context.t explicitly supplies initial model, Var, window, native controllers,
observation references and application callbacks. [Application](application.md)
invokes run from an application-scoped Eio startup task under its 60/150-second
timeout, after desktop readiness/probing. Read Context, UI helpers, workload,
unavailable check, self-test/document scenarios and final release.

From the root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --self-test
python3 scripts/measure_signal_studio.py --output scratch/signal-workload
python3 scripts/test_signal_desktop.py --output scratch/signal-desktop
```

The desktop harness chooses disposable document paths. --document-path= only
exercises files alongside --self-test, and that scenario writes, truncates and
removes its destination; use the disposable harness fixture, never a user document.
The unavailable check is only meaningful on an actually unavailable backend (such
as unbundled macOS); it asserts that specific condition, not general permission
denial. The [README](README.md#local-checks) describes full native input/platform
limits. Dated [readiness and workload evidence](../../docs/evidence/window-readiness-and-pie-backing-och17.md)
records the checks actually run; listing a command here does not qualify every
diagnostic mode.

## Await observations without blocking UI processing

on_ui uses [Ui_thread.perform](ui_thread.md) with App.scope. ui wraps a thunk in
Effect.of_thunk, ensuring mutable application/native state is touched on UI domain.
until polls predicates on that domain every 5 ms from the Eio task, with the outer
Application timeout bounding it. Self-test/workload first wait for chart Ready
revision>0 and at least one extension mount; ordinary mode does not. run returns
whether a requested diagnostic completed; no flags returns false normally.

workload's sample reads App.diagnostics and tracks peak combined chart/canvas
source charges and pending requests. Its await helper calls sample and the
predicate inside until's UI turn; the Eio task sleeps between failed attempts,
leaving UI event processing free to receive native acknowledgements.

At the start of every workload cycle, the actual code is:

```ocaml
let window = ui (fun () -> ensure_window ()) in
await (fun () -> App.Window.is_open window);
frame window;
```

ensure_window returns an existing window or creates a handle whose native opening
can still be in progress. [App.Window.is_open](../../lib/eio/app.mli) queries that
lifecycle on the OCaml UI domain: it becomes true only after the native Opened
acknowledgement, while closing has not begun and the app is not stopping. Both
is_open and is_closed are false during Opening, so testing not is_closed would
not establish readiness. A Window.snapshot can already be Some after an early
Window_changed event during Opening; snapshot presence is therefore also an
incorrect barrier before request_frame. Neither is_open nor a snapshot proves
focus, visibility, resource publication or a completed frame. A later operation
can still encounter closure after the query.

The frame helper creates an Eio promise, submits this request on the UI domain,
then awaits the promise in the Eio task:

```ocaml
App.Window.request_frame window ~on_rendered:(fun ~revision:_ ->
  E.of_thunk (fun () -> Eio.Promise.resolve resolver ()))
|> ok
```

E aliases Bonsai.Effect. E.of_thunk builds a unit effect describing work to run;
the on_rendered callback returns that effect to resolve the promise when the
native render callback arrives. ~revision:_ explicitly ignores the callback's
revision. The pipe passes the request's unit Or_error.t to ok (Or_error.ok_exn),
so rejection fails this diagnostic rather than silently waiting. request_frame
requires an open window and permits at most one pending request per window.
The Eio.Promise.await belongs outside the UI thunk: waiting there would prevent
the UI from handling the callback needed to resolve it.

For example, a reopened cycle can receive window metadata while is_open is still
false. The next poll after Opened sees true; only then does frame submit its
request. Later batches use set_run/update callbacks to change the application's
Bonsai Var on the UI domain, letting its reactive view rebuild; checks read that
same Var with B.Expert.Var.get. This file uses effects and Var reads rather than
building a Bonsai computation with let%arr. See [Application](application.md) and
the [Component walkthrough](component.md) for the state-to-view wiring.

settled separately means Scene and Chart are published and no requests are
pending. Changed chart data also requires a later observed chart Ready revision
before the batch requests its frame. A render callback establishes native render
observation; it does not establish resource/data readiness by itself or physical
screen presentation. Occlusion can defer it because the pinned macOS backend
stops its display link while a window is occluded. Screenshot assertions and
real keyboard/IME validation require their separate harness evidence.

## Workload batches and resource lifetimes

Twelve cycles each open/reuse a window and perform eight batches. One UI turn
publishes four desired runs, cycling modulo 101. There are 384 desired updates in
96 batches; native resource schedulers coalesce unstarted publications, so 384
native uploads are not promised. Each batch checks asynchronous scene/chart
errors, waits for settlement and a later chart revision if data changed, requests
a frame and checks final Workspace/chart data agreement and one registration
per resource. Logs report publish/update-frame elapsed milliseconds and actual
submitted byte/message deltas. Source charge is a resource accounting field,
not total process RSS; the external measurement harness collects CPU/RSS.

Each cycle then sends one explicitly sequenced counter command, awaits its exact
ack without a new mount, and clears it so a later window cannot replay it. Hide/
show preserves mount count; an explicit generation increment requires one new
mount. Window close waits for no windows/pending requests/queued commands while
asserting chart/canvas registrations remain alive. Finally explicit Scene.release/
Chart.release and clearing snapshot handles must return registrations and source
charges to zero. Extension package lifetime diagnostics and screenshot evidence
are external harness concerns, not inferred from these counters alone.

## Self-test and document races

Self-test issues Canvas Select and awaits acknowledgement, checks observed model
selection, sets run 7 and awaits chart revision ≥2. A synthetic Extension.Data 101
checks model rejection without a physical click. With optional disposable path:

- Save and inspect native represented-file/edited metadata, then save submitted
  run 8 while a following effect sets run 9. Disk retains 8 and the UI stays dirty.
- Load 8 and check a clean workspace. A second injected Documents controller uses
  promise gates to delay reads deterministically. An overlapping save must be
  rejected by busy admission; editing to 10 before releasing the gate preserves 10.
- A second gated read is superseded by Documents.reset; its reply cannot replace 10.
  Then invalid file contents and a missing file must also preserve 10 and report
  load failure. The fixture is deliberately overwritten/deleted for these cases.

The delayed reader is a controlled asynchronous fake; other save/load operations
use the real [file helper](files/document_file.md). These assertions exercise
controller contracts separately from actual native picker gestures. At the end,
self-test waits for a render callback, explicitly releases resources and checks
zero registration bytes. layout/mount/motion counts are logged; this function
does not assert every motion event or inspect pixels.

unavailable_check asserts probe's Some(Error Unavailable), then Enable/Notify
must keep alerts disabled and show an unavailable fallback. It never grants OS
permission. When mixing diagnostic flags, blocks run sequentially against the
same context; the documented harnesses choose suitable modes rather than treating
arbitrary combinations as independent clean tests.

For new assertions, add a meaningful observation and timeout-bounded wait, not
a fixed delay assuming completion. Keep fake callbacks explicit and typed.
These diagnostics cover asynchronous publication/resource accounting; physical
input, screenshots, OS routing/panels/notification actions and macOS/Linux
qualification require their separate harness evidence.
