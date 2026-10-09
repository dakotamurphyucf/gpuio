# Own the model, asynchronous services and native registrations

[application.ml](application.ml) and [application.mli](application.mli) expose
run and implement Signal Studio's runtime. Read CLI configuration, snapshot/
publication helpers, event handlers, streaming, documents/actions, window/service
wiring, then startup and optional Checks.run. The pure [Workspace](model/workspace.md)
computes fixture data; [Component](component.md) observes the Var with let%arr;
[Ui](ui.md) describes views. This module owns their effects and lifetimes.

From the root after [isolated setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --background
```

The model is simulated locally without credentials. Background avoids initial
focus; it is not evidence of foreground keyboard/IME acceptance. Exact native
checks and macOS-first/Linux coverage are in [README](README.md#local-checks).
This walkthrough does not execute them.

## Configuration and one application state

run recognizes self-test, workload, notification-unavailable check, motion tracing,
background/exit-on-close flags and optional validated document path. Full/reduced
motion are mutually exclusive; absent overrides use System. App.extension_catalog
forces [linked backend initialization](backend/backend.md), and the code asserts
that Counter.schema is available before launching. Startup links collect
--open-uri= values before --open-uris plus all trailing arguments after that marker.
App.run_desktop receives the [shared identity](application_identity.md), motion,
links and exit policy; ordinary window close leaves the application alive.
Exited/Forwarded/error outcomes are handled distinctly. Diagnostic flags also
require completed=true after exit, so they are intended for the local instance.

Inside the application callback, Workspace.create supplies initial state.
Bonsai.Cont.Expert.Var.create stores Ui.Snapshot with no native handles yet,
inspector open, wide header initially, generation 1, no commands and disabled alerts.
update reads/replaces the snapshot on UI domain. Runtime references track actual
controllers/windows, command/ack sequences and diagnostic observations; they are
not serialized workspace fields. with_alerts reads the optional controller when
an effect is handled, then returns its effect or Ignore. This avoids capturing a
not-yet-created service into button actions.

## Publish data and process asynchronous observations

publish sets desired Workspace.scene/chart on existing scoped controllers, updates
the workspace/status Var and refreshes document dirty/native metadata. Scene.set/
Chart.set accept local desired values and coalesce unstarted publications; their
[contracts](../../lib/eio/canvas.mli) and [chart contract](../../lib/eio/chart.mli)
separate acceptance/error from painting. The ordinary publisher checks immediate
Result failures; the workload also polls asynchronous controller errors.
command increments a positive int64 sequence and publishes a Canvas.Command.
Selecting a sample button sends Select; model selection changes on its observation.

on_canvas returns Effect.of_thunk: Selection_changed validates selection and
refreshes documents; Activated also opens the inspector; Moved extracts the
transform's origin, applies Workspace.move and republishes; Viewport_changed only
updates status. Pan/zoom remain native. Command_completed updates the maximum ack;
Failed raises a typed error. on_chart records Ready data revision or changes
selection status, without mapping chart datum selection to workspace selection.
Its Failed also raises. on_extension accepts Data through set_run, records mounts
and exact command acknowledgement, and raises Failed. set_run uses the model's
0–100 validation; a late native click at the disabled limit reports rejection
without replacing the valid workspace.

A concrete drag trace is native event → deferred UI thunk reads current model →
validated new workspace → Scene/Chart desired updates and Var.set → Component
let%arr → Ui.view → native reconciliation/publication. A render callback is needed
to establish a frame; none of these value changes alone proves physical paint.

## Streaming cancellation and latest model reads

stop increments stream_epoch, cancels the stored Scope.Task, clears it and marks
not running. start calls stop first, records the resulting epoch and starts one
application-scoped Eio task. Twelve iterations sleep 0.3 seconds, enqueue a UI job,
read the latest run and, only for the matching epoch, increment it or wrap 100→0.
Each iteration awaits a promise resolved by that UI job. This is ordered delivery,
not twelve captured workspace snapshots or a provider stream.

Scope.Task.cancel suppresses its late on_result delivery; the epoch additionally
fences queued mutation. On successful completion, the UI effect clears running,
sets Run sequence complete and notifies alerts with the current run. If the task
fails, ok raises rather than manufacturing a success. Application-scope cancellation
ends work at quit; responsive hiding or window close does not own this task.
The [Scope contract](../../lib/eio/scope.mli) is essential when adapting this loop.

## Document and action ownership

Documents.create receives current model, replace, stop_stream and report callbacks,
a current-window getter and explicit filesystem/secure-random capabilities.
replace republishes, increments extension generation and issues Select for the
loaded selection. [Documents](documents.md) owns busy admission/save baselines/
load fingerprint/reset epoch; [Document_file](files/document_file.md) owns bounded
validation and atomic local replacement. Directory comes from --directory, then
HOME, then /, with invalid paths falling back to /. Neither URL routing nor the
file panel itself creates ambient file authority.

Actions dispatch these existing functions as effects. Reset stops streaming,
publishes initial workspace, increments extension generation, clears extension
command, sends Select None and resets documents. It does not call Scene.reset or
Chart.reset: those are separate native resource-generation APIs. Lock/Hide change
presentation flags without domain run reset. on_layout records the native branch
for header/outer-height state. on_motion increments diagnostic count and optionally
logs typed events. quit calls App.shutdown.

## Current windows and application services

ensure_window reuses an open exact handle or opens 1160 × 860 using Component with
the shared state/actions. Its first on_change refreshes native document metadata.
activate ensures a window and, if it has a snapshot, submits Activate; failures
are logged. No raw window slot is stored in a notification.

Notification.attach creates one application-owned receiver. The injected Alerts
backend uses authorization/capabilities/post/replace/dismiss and explicit retry
before permission request; all posts use tag completed-run. The controller's
activate callback resolves the current window each time, so an old window's close
does not invalidate a later alert action. Scope.on_cancel closes Alerts terminally.
App.on_reopen uses the same activation path. Probe queries permission without
prompting or opting in; only an Enable action requests it.

Desktop.attach routes Link through Workspace.route. Accepted known sample links
update the model, ensure a window, send native Select, refresh documents and
activate. Rejected/overflow/service-failure events log without granting filesystem
access. The receiver and notification service are independent of individual
windows and are marked ready only after startup can route to live state.

## Startup handles, timeouts and diagnostic boundary

A Scope.start startup task runs under Eio.Time.with_timeout_exn: 60 seconds ordinarily,
150 for workload. [Ui_thread.perform](ui_thread.md) lets that Eio task await UI
Scene.create/Chart.create effects. Their completion is first native publication;
scope cancellation releases registrations and borrowed handles do not extend
lifetime. A UI thunk adopts controllers and handles in snapshot. The task waits
for a native window snapshot, announces Desktop.ready/Notification.ready, then
probes alerts. It deliberately does not await chart paint before desktop readiness:
an occluded macOS window may need the incoming link to activate it first.

Checks.run receives explicit [diagnostic context](checks.md), including observations
and callbacks. Ordinary run has no check flags, so it introduces no chart-paint
assertion gate. Diagnostic completion triggers shutdown and final completed assertion.
Normal resource ownership persists through window close/reopen and ends with
application scope; checks also test explicit releases. Attach/native failures
usually raise/log in this acceptance demo rather than presenting a production
recovery workflow.

To adapt to real model results, preserve pure validation and publish on UI domain
from a scoped task with stale-result fencing. Change storage/notification policy
through their injected interfaces, and retain current-window activation. Avoid
moving registration or blocking waits into Ui.view; that would mix layout with
resource lifetime and break the startup/event boundaries described here.
