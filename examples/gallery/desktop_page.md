# Window document metadata and shared desktop actions

[desktop_page.ml](desktop_page.ml) and [desktop_page.mli](desktop_page.mli) build
the Desktop services gallery page. Read aliases/result_message, local state and
activation refresh, apply/choose/file_action, then three cards. `B = Bonsai.Cont`
is reactive graph construction, `E = Bonsai.Effect` is deferred work, and
`V = Gpuio_bonsai.View` describes native presentation. This page borrows one
[application desktop session](desktop_session.md) and one exact window; they have
different lifetimes.

From the root after [setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section desktop
```

Choose Desktop services. Ordinary launch uses fixed preview content, not external
credentials. Direct macOS launch can display Unavailable for packaged services.
The native driver is authored coverage, not evidence newly executed by this review;
[README](README.md#desktop-integration) separates packaged checks and Linux scope.

## Reactive observations and exact window ownership

B.state creates optional document and local file-action notice plus reactive setters.
A let%arr derives refresh from the setter: App.Window.command Observe obtains the
native snapshot and extracts Window.Document. B.Edge.lifecycle runs it on page
activation. This matters when re-entering a page after another action changed
window metadata. The final let%arr combines palette, shared session snapshot,
document and setters into the view. It does not execute all displayed actions.

Absent observed metadata, current defaults to an untitled document with edited=false.
apply calls Desktop.set_document on this window and updates from its returned
snapshot; it reports errors rather than inventing success. [Desktop's contract](../../lib/eio/desktop.mli)
sets represented path/edited badge for an exact window generation without reading
or writing a file. A closed/unsupported window reports an error. Mark saved simply
clears metadata's edited flag; it is not a storage operation or close confirmation.

choose invokes a native Open panel labelled Represent. Cancellation reports locally;
exactly one path becomes represented metadata; an unexpected multiple selection is
rejected. Paths remain typed native bytes and are not rendered as UTF-8 text labels.
Clear removes represented path. Mark edited/saved keeps the current optional path.
Reveal/Open disable without a path and call explicit application desktop operations
when clicked. Their request-accepted notice is not proof of visible file-manager
selection or target application consumption. Selecting a represented file by
itself invokes neither operation.

## Shared services are independent of this page

The identity/support/link card observes Desktop_session.snapshot. Check uses its
serialized support/authorization lane; Retry requests incoming intake again;
Activate application is a separate process-level Desktop.activate effect. Register
Studio links explicitly calls register_scheme for the declared scheme; it can
change the OS handler and may prompt. It is never called automatically at entry.
The packaged identity/bundle must match; Linux runtime registration is Unsupported.

Notification controls call shared Allow/Post/Replace/Dismiss. Disabled state derives
from session.busy/has_receipt, but the controller rechecks admission at effect
execution across all windows. Only Allow requests permission. Link/notification
observations update shared text and do not automatically activate windows or open
files. Leaving this page disposes its native presentation, not the application
receivers/receipt. The local represented-file metadata stays with its window.

Concrete trace: Choose represented file → panel reply supplies File_path →
set_document exact window → returned native snapshot updates document setter →
let%arr rebuilds label/buttons. In a second window, shared support/notification
text can update simultaneously while represented-file observations differ.
E.bind sequences effect replies; E.Many dispatches the two local state updates.

For real documents, add an explicit storage controller with validated contents
and save/load ownership; do not infer persistence from edited metadata. For an
extra OS action, retain typed error reporting and qualify acceptance versus visible
presentation. [Packaged harness](README.md#desktop-integration) exercises actual
OS routing/services; compilation/source review does not establish GUI acceptance.
