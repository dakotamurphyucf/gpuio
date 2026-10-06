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
limits. Commands here were statically reviewed, not newly run.

## Await observations without blocking UI processing

on_ui uses [Ui_thread.perform](ui_thread.md) with App.scope. ui wraps a thunk in
Effect.of_thunk, ensuring mutable application/native state is touched on UI domain.
until polls predicates on that domain every 5 ms from the Eio task, with the outer
Application timeout bounding it. Self-test/workload first wait for chart Ready
revision>0 and at least one extension mount; ordinary mode does not. run returns
whether a requested diagnostic completed; no flags returns false normally.

workload's sample reads App.diagnostics and tracks peak combined chart/canvas
source charges and pending requests. frame requests App.Window.request_frame,
then awaits a promise from its on_rendered effect. It observes native rendering,
not a screenshot assertion or real keyboard/IME input. settled means Scene and
Chart published plus no pending requests, distinct from a frame callback.

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
