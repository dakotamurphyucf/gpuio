# Document operations, submitted snapshots and stale replies

[documents.ml](documents.ml) and [documents.mli](documents.mli) implement a
UI-domain controller around the pure [workspace](model/workspace.md), native
file panels and application-scoped Eio file work. Read State/Model, create/notify,
refresh/reset, admission/start_io, save/load helpers, then public picker actions.
This module contains no Bonsai graph construction: it returns deferred
`unit Bonsai.Effect.t` actions and publishes State through an injected callback.

From the repository root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe -j 2
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --directory=/absolute/path
```

Choose an existing directory for the panel's initial folder. Save/Open/Reveal
exercise native services, so compilation does not prove their behavior. The
[README](README.md) describes macOS desktop checks and deferred Linux GUI scope.
No provider or network is involved; files contain the deterministic model fixture.

## Controller state and injected capabilities

State.t exposes optional saved path, edited/busy flags and an optional native
metadata result. None metadata means no current reply, not file failure.
Model.t supplies current, replace, stop_stream and report callbacks. The abstract
controller stores these, the application, a getter for the current window,
explicit load/save functions, on_state, serialized saved baseline, epoch and
metadata revision. create records the current encoding as its initial baseline.

[Application](application.ml) injects filesystem capabilities into
[Document_file](files/document_file.mli), publishes replacement models to canvas,
chart and observable Ui.Snapshot, and updates its document snapshot via on_state.
The controller never discovers ambient filesystem authority or turns a deep link
into a path. load_path/save_path accept explicit validated File_path values;
the caller authorizes those destinations. They are useful for integration tests.

notify compares the entire current Workspace.encode string with saved. Selection
is serialized, so selecting a different sample can mark the workspace edited.
It calls on_state after recomputing the flag. Unlike the immutable Workspace,
this controller has mutable UI-domain coordination state; keep its calls on that
domain. Ordinary [Component](component.ml) uses let%arr to observe a snapshot and
derive a view; it does not perform file I/O during rendering.

## Trace a save and a competing edit

save_as returns an effect. When handled, with_picker admits one operation by
setting busy, requires an open current window and captures epoch. File_dialog.save
opens a native panel with suggested name workspace.signal. Cancellation/error
clears busy and reports a result. A selected destination proceeds only if epoch
still matches. `E.bind` sequences a completed effect into the next action;
`E.map` processes its reply, and `E.of_thunk` defers UI-domain mutations.

save_admitted captures the workspace **after** the picker selection and its encoded
contents. start_io runs the injected save in Scope.start using App.scope, not a
window scope. Its UI completion always clears busy. If the epoch still matches,
success records the submitted contents/path, then refresh recomputes dirty state.
An edit made during I/O differs from that baseline and remains dirty. A file
success does not mean the current model was saved if it changed meanwhile.

Document_file.save writes a private temporary sibling, synchronizes it and
atomically replaces the destination, cleaning up temporary files on failure or
cancellation. Files use 0600 permissions; existing permissions are not preserved,
symlink destinations are replaced rather than followed, and containing-directory
crash durability is not claimed. See its interface for the actual filesystem
contract; these guarantees are not provided by the picker itself.

## Trace a load, reset and window close

open_ first notifies/rejects edited workspaces with a Save-or-reset message. It
stops streaming and captures the current encoding before showing the open panel.
The panel's first selected path is passed to load_admitted with that baseline.
Direct load_path captures the baseline at admission instead. The injected reader
bounds input and Workspace.decode validates version, identities and coordinates.
A successful read replaces the model only if its before string still equals the
current encoding. An intervening edit reports an abandoned load and stays intact.
Invalid/missing reads preserve the existing workspace and report a generic UI
failure while logging the detailed error. Or_error.join flattens task failure and
the reader/writer's own recoverable result.

reset increments epoch, records the already-current model as the new baseline,
clears path and refreshes. The application resets its model separately. Epoch
fences old picker/I/O acceptance; it does not cancel an already-running write or
clear busy immediately. That operation drains before another is admitted, and a
write may still finish on disk even though its old completion is ignored.

The application scope owns file tasks. Closing a window therefore preserves
workspace/controller and can leave file work running; quitting ends the owning
scope. Scope.start admission failure clears busy and reports task unavailability.
There is no autosave, implicit discard or quit-confirmation flow here.

## Native document metadata is a separate asynchronous result

refresh first notifies and obtains the current window. No window or a closed one
means no native request. Otherwise Desktop.Document.create describes path/edited,
metadata is cleared and metadata_revision increments. Desktop.set_document's
reply is accepted only for the latest revision and an open window. Its Result is
stored independently of file success; an OS badge failure cannot undo a completed
save. Reopening uses the application's current window getter and later refresh.
reveal reports missing saved path or submits Desktop.reveal_file. Success means
reveal was requested, not that Finder selected an existing file.

[checks.ml](checks.ml) supports document integration scenarios: submitted-save
versus later edit, edits during delayed load, busy admission, reset fencing,
invalid/missing reads and native path/edited metadata. The actual harness command
is documented in [Local checks](README.md#local-checks):

```sh
python3 scripts/test_signal_desktop.py --output scratch/signal-desktop
```

That macOS harness uses disposable files and real panels; this documentation
review does not run it or claim native acceptance. Pure model expectations cover
serialization separately. To adapt this controller, inject another explicit
storage implementation while preserving submitted-snapshot and stale-load checks.
A discard confirmation requires an intentional new action flow; removing the
edited check alone would change data-loss behavior.
