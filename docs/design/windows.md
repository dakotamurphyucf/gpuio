# Windows and workspace lifetimes

Status: OCH-15 implementation in progress. Local evidence is recorded below;
this is not milestone completion or Linux GUI acceptance.

## Application and window ownership

`App.run` owns one native GPUI application and one OCaml UI domain. Each
`App.Window.t` has a generation-checked native identity, its own Bonsai driver,
and a child task scope. Applications may share immutable values or explicit
UI-domain stores across drivers. A window handle never becomes a replacement
window when a native slot is reused.

`App.open_window` retains its existing defaults; `open_window_config` accepts a
validated `Gpuio.Window.Config`. Configuration selects title, initial logical
bounds, focus, standard/hidden chrome, and user resizability. `focus=false`
opens without application activation. Chrome is a platform preference; Linux
compositors may determine decorations. Global positioning is not offered as a
portable Wayland command.

`App.Window.command` returns a correlated effect with either an observed
snapshot or a typed error. Commands include title, content resize, activation,
zoom, fullscreen toggle, native edited indicator, and observation. Up to64
requests are pending across an application. A delayed effect targets its exact
window generation. An acknowledgement is not a promise that an asynchronous
compositor transition has finished.

`Window.snapshot` caches the latest observation. `Window.on_change` delivers
changes on the OCaml UI domain. Bounds and activation observations coalesce per
window in a separate bounded control lane; they do not consume user-input
capacity. Snapshot width/height describe outer native bounds; content_width and
content_height describe the drawable viewport. Resize requests content size.
All dimensions use logical pixels. Wayland positions are compositor-relative
observations, not a promise of global screen coordinates.

## Close, quit and reopen

`Window.request_close` asks the installed close handler for `Allow` or
`Keep_open`. The handler returns a Bonsai effect and can finish asynchronously
through scoped Eio work or an application dialog. Window work remains live
while the decision is pending. Duplicate requests coalesce. `Window.close` is
an explicit force-close, bypasses the handler, cancels window work and
invalidates delayed decisions. Closing one window leaves other drivers live.

`App.request_quit` asks all live windows and waits for approval before initiating
application shutdown. One denial keeps the application open. New windows are
rejected while quit is pending. At most one close and one current quit waiter
are retained per window, including repeated denied quit attempts while another
window is still deciding. If an ordinary close is already pending, its decision
is shared with the quit attempt; its original reason remains the handler's
reason. Application policies must account for edits made while a decision is
pending. `App.shutdown` is explicit force cleanup and bypasses decisions.

The default exits after the last approved window close. With
`exit_on_last_window=false`, application/conversation scopes can keep working
with no windows. `App.on_reopen` receives the native reopen request; applications
choose whether to activate an existing window or create a new one. Session
restoration and desktop single-instance/deep-link services belong to later work.

The pinned GPUI `Platform::on_quit` callbacks are **cleanup-only on both OSes**;
macOS calls it from `applicationWillTerminate`, and Linux after its loop ends.
Their Boolean result does not provide a veto. GPUIO preserves those callbacks.
A small macOS adapter adds AppKit `applicationShouldTerminate` to a no-ivar
subclass of this application's existing delegate instance. It queues an intent
and returns `NSTerminateCancel`; approved quit uses GPUIO's embedded shutdown
path so `App.run` returns. The adapter inherits GPUI's other delegate methods,
holds the delegate alive, and restores its original class during cleanup. It
never calls OCaml or borrows GPUI from the OS decision callback.

`App.window_capabilities` explicitly reports that macOS supports this native
quit decision. Linux application/menu commands can request asynchronous quit;
termination of the compositor/event loop is not presented as vetoable. OS
force-kill is not a save protocol on either platform.

## Local evidence and remaining work

`examples/window_lifecycle` exercises two public OCaml windows, title commands,
delayed deny/allow, duplicate requests, scope cancellation, stale callbacks,
repeated denied quit attempts, and all-window approval. Its `--last-window`
mode checks the default exit policy. Native `native_window` invokes actual
macOS `performClose`, application termination, and delegate reopen paths;
its parent process requires a marker emitted after all assertions and cleanup,
so an early OS `exit(0)` cannot pass the test.

All eight local window-family validation commands passed on macOS on2026-09-24:
paired Rust/OCaml wire fixtures, public example build, decision/resize scenario,
last-window scenario, native OS callback/reopen scenario, Clippy with warnings
denied, and mailbox/session tests. `native_window` includes the parent marker
check. No hosted CI or merge has run for this milestone yet. Linux build/unit
checks are required at integration; Linux GUI checks remain informational per
the owner's platform policy. Tab/split-pane APIs and the integrated agent app
are the next implementation stages.
