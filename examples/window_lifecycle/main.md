# How `main.ml` tests delayed close and atomic quit decisions

[README](README.md) · [Source](main.ml) · [Window API](../../lib/eio/app.mli)

This executable is a scripted lifecycle diagnostic, even without a `--self-test`
flag. The default runs `run_decisions`; `--last-window` selects a shorter scenario.
Its buttons exist for normal close requests, but the script drives its own window
operations and decisions. Avoid interacting while it runs.

## Static Bonsai content and application effects

`component label window _graph` uses `B.return` to return a constant column.
There is no `B.state`, reducer, or `let%arr`: its label does not change reactively
when a native title changes. `E.of_sync_fun App.Window.request_close window`
wraps an ordinary close request as a button effect. Native click → effect → close
policy is the interaction path; the view itself has no separate open/closed model.
The application and native window lifecycle own that state.

`App.run` owns GPUI on the OS thread and one OCaml Eio UI domain. `App.open_window`
returns an exact window-generation handle and owns a window scope. The two default
windows request background opening at 600 × 360 logical pixels. App scope work
outlives one window; window scope work is cancelled when that window closes.
`Scope.Expert.on_cancel` counts cancellation for the first window.

## Commands and deferred decisions

The app-scoped producer has a 20-second timeout. `on_ui` enqueues an effect using
`Scope.Expert.enqueue`, handles it through `E.Expert.handle`, and resolves an Eio
promise with `E.map`; awaiting it sequences script operations. `sync` adapts a
synchronous UI function, and `command` adapts `App.Window.command`.
`Observe` retries `Not_ready` in 5 ms intervals. `Set_title "Renamed λ"` checks
that only the first window changes. `Resize (640., 400.)` requests content size;
its command reply is not completion of compositor resizing, so the script waits
for observed content dimensions separately. See [window values](../../lib/core/window.mli).

`set_close_handler` installs a function returning a decision effect. Here
`E.Expert.of_fun` intentionally captures its callback without completing it.
Two `request_close` calls coalesce into one pending decision. While pending,
the window and its work stay live. Calling the saved callback with `Keep_open`
resumes the window without cancellation. A subsequent request creates another
decision; answering `Allow` closes it and cancels its scope exactly once.

Commands through the closed handle return `Closed`. The script opens a
replacement window and invokes saved old answers again: exact generations and
decision identities prevent those answers from closing the replacement. A label
or recycled native slot would not be a safe substitute for the original handle.

## Atomic application quit

The default runner uses `exit_on_last_window:false`, allowing application policy
to control exit. It installs handlers on the surviving windows and calls
`App.request_quit` twice. Each handler checks reason `Application_quit`; repeated
requests coalesce. Opening a new window during pending quit is rejected.

One window answers `Keep_open`. Repeated subsequent denied attempts must not
accumulate callbacks on the other window's still-pending decision. The script
checks 20 denied attempts, then starts another quit, force-closes the replacement,
and finally allows the remaining window. `App.Window.close` deliberately bypasses
the close handler; use `request_close` for ordinary user actions. Passing this
script prints `GPUIO_WINDOW_LIFECYCLE_OK` after `App.run` returns.

The result callback returns `E.Ignore` on success; on failure it shuts down and
raises the error. This is diagnostic bookkeeping with refs/callbacks, not a
production unsaved-document implementation. In production, return an effect
that resolves from a real user decision and owned save operation, without
blocking native callbacks or retaining answers after their lifetime.

## Last-window scenario and commands

`run_last_window` uses default exit policy, opens two background-requested
500 × 300 windows, and starts a 10-second app-scoped task. It waits for snapshots,
requests the first close, verifies the second survives, then requests the last
close. Returning from `App.run` prints `GPUIO_LAST_WINDOW_OK`.

From the repository root in the [configured environment](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/window_lifecycle/main.exe
_build/default/examples/window_lifecycle/main.exe
_build/default/examples/window_lifecycle/main.exe --last-window
```

Both modes require a graphical session/native backend and close their windows
programmatically. [Dune](dune) links the protocol library alongside GPUIO/Eio and
enables Jane Street/Bonsai PPX. These diagnostics exercise decision ordering,
command observations, cancellation, and shutdown. They do not generate real
close-button/keyboard events, validate document persistence, or establish
[Linux graphical acceptance](../../docs/platform-release-policy.md).
