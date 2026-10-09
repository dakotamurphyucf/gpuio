# OS delivery with explicit permission, receipts and window routing

[main.ml](main.ml) implements a notification lab, separate from in-app toasts.
Read identity/content helpers, `main` service attachment and operations, component,
window lifecycle and final plist branch. [README](README.md) gives macOS bundle
harness commands; [dune](dune) links Core/GPUIO/Bonsai/Eio with PPX. Setup/platform
limits are in [development](../../docs/development.md) and
[release policy](../../docs/platform-release-policy.md). Actual support and
presentation are backend/OS policy, not inferred from building.

`identity` validates desktop identifier com.gpuio.notification-lab and name.
Tag build identifies one logical live notification; validated action IDs open/
inspect are separate from labels. `content ~updated ~index` constructs bounded
plain-text title/body with one action. The Unicode fixture is not markup or a
service result. Receipts are opaque process-local lifetimes; never persist them
or use a tag as window slot. [Notification](../../lib/core/notification.mli)
defines value limits.

`App.run ~desktop:identity ~exit_on_last_window:false` keeps app/service alive
without windows, with GPUI on OS thread and one OCaml Eio UI domain.
`N.attach` owns one application service and does not prompt/post/activate windows.
`status` is external reactive `B.Expert.Var`; receipt/count/current_window/
target_window refs are UI-domain application routing state. `emit` logs and sets
status. Component `let%arr` reads that status to derive view; graph is unused,
and there is no separate `B.state` or reducer. Effects execute on actions rather
than while building views.

`permission` maps explicit request_authorization result into status. `probe`
uses `E.bind` to sequence capabilities then authorization and `E.map` to format
results. Post reads latest count in an effect, calls `N.post ~tag`, and only on
success retains receipt and current exact window as target. `with_receipt` reads
current receipt when effect runs; Replace keeps its lifetime but changes body/
action, Dismiss clears it after successful admission. Ready enables queued event
intake; Close permanently disables this service for app run. Normal mode calls
ready automatically; harness `--self-test` waits for the Ready button instead.
See [adapter](../../lib/eio/notification.mli).

On native action, ordered handler logs event and compares delivered receipt using
typed equality against current receipt. Matching Activated/Action returns saved
exact target window; terminal matching receipt clears routing state. The next
bind issues `App.Window.command Activate` and formats result. Closed/Failed does
not activate. A replaced/new window is not silently substituted for a closed
notification target. Native delivery/selection is asynchronous; OS acceptance
of Post does not prove banner visibility or user attention.

`ensure_window` opens a 780 × 490 window with background activation requested only if none/current
closed, retaining exact handle. `App.on_reopen` invokes it; with no windows,
service remains live until Quit lab or deadline. App-scoped Eio clock task exits
after 180 seconds in harness mode or one hour ordinarily. Shutdown closes service
and scopes, suppressing queued late events; no file/network work occurs here.
macOS packaging/permission is real prerequisite, not a demo fallback.

From repository root after building:

```sh
_build/default/examples/notification/main.exe --print-info-plist
_build/default/examples/notification/main.exe --unbundled-check
python3 scripts/test_notifications_macos.py --artifact scratch/notification-os/review-001
```

Plist-only branch uses `Eio_main.run` and explicit stdout via `Gpuio_eio.Output`;
it opens no window. Unbundled check expects macOS authorization Unavailable and
shuts down; it is backend-specific. The harness copies executable into .app,
emits plist, ad-hoc signs matching identity, registers bundle and opens it with
`--self-test`. It drives permission/actions/dismissal/replacement/target routing
and reaps its app. `--self-test` alone is not autonomous acceptance automation;
it changes readiness/deadline only. Permission/Notification Center activity and
Accessibility prerequisites are detailed in README. No signing certificate is
needed for local harness; release signing is separate. Linux uses freedesktop
service/capabilities and Not_required authorization; private-bus evidence and
actual desktop presentation remain distinct in
[the contract](../../docs/design/os-notifications.md).

To route many notifications, replace single receipt/target refs with a bounded
application map keyed by receipt, keeping stable logical tag and exact window
handle separate. Handle Busy/Denied/Unavailable explicitly and choose an in-app
toast fallback without claiming OS delivery. Start I/O with explicit Eio
capabilities/scopes outside `let%arr`; call ready only once application routing
can consume queued events safely.

From the repository root, build and launch with the configured toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/notification/main.exe
_build/default/examples/notification/main.exe
```

The ordinary executable is not a substitute for the macOS packaged harness above.
Its `--self-test` flag requires that harness to drive permission and service actions.
