# How `main.ml` manages toast content

[Example README](README.md) · [Source](main.ml)

This program displays in-window toasts. The message says a draft was saved, but
there is no persistence operation. These toasts are separate from the operating
system notification service demonstrated by [notification](../notification/README.md).

## The model and reactive graph

`Model.t` contains `next`, the next unused integer identity, and `items`, the
identities currently displayed. Initially `next` is 1 and `items` contains 0.
`Action.t` is either `Add` or `Dismiss of int`. `B.state_machine0` installs the
model and reducer in the Bonsai graph. Its returned `model` is a reactive value;
`inject` creates an effect that submits an action. `Add` appends the next identity
and increments the counter; `Dismiss` filters that identity out.

`graph` connects these state nodes to the component's lifetime. `B` abbreviates
`Bonsai.Cont`, and `E` abbreviates `Bonsai.Effect`. Opening `B.Let_syntax` enables
`let%arr`: the view expression runs again when its bound reactive model changes.
The `and` binding observes `inject` alongside the model. It does not run an
injection during rendering. Effects attached to buttons run when an event arrives.

`Toast.Timeout.after` validates the timeout: 5 seconds normally, or 0.3 seconds
with `--self-test`. `Toast.Config.create` validates the accessible label and
configuration. `Or_error.ok_exn` makes a rejected configuration fail immediately;
an application accepting user configuration should handle the error instead.

## From an event to native dismissal

Each model identity becomes a `View.toast` with a stable `Gpuio.Key`, content text,
and an application dismissal button. `View.toast_stack` validates and groups these
nodes. The window runtime renders the stack and owns native placement, timeout
progress, and dismissal delivery. Bonsai owns the list and content.

Clicking “Save another draft” runs `inject Add`. The reducer adds an identity,
`let%arr` produces another keyed toast, and the native runtime mounts it. When its
active timeout expires, native delivery calls `on_dismiss` with `Timeout`.
`E.Many` submits `Dismiss id` and records the reason in the diagnostic reference.
The next view omits that key, so the toast disappears. Clicking “Dismiss from
application” directly injects `Dismiss id`; that path does not set the diagnostic
reference or pretend a native timeout occurred.

Timeouts count active time. Hidden or blocked windows, stack hover, and contained
focus can pause them. The default stack shows up to three toasts and dismisses
older overflow entries. If adapting this example for bursts, bound the submitted
application list as well: the stack contract caps a submitted keyed list at 32.
Use unique identities for simultaneous messages and handle dismissal as removal
of the specific message, rather than clearing the entire queue.

## Startup and diagnostic ownership

`App.run` owns the Eio application runtime. `App.open_window` mounts `component`
in a 640 × 360 window. Normal execution stays open for interaction.

With `--self-test`, `Scope.start (App.scope app)` starts an application-owned task.
It waits in 5 ms intervals for the callback reference, inside a 15-second timeout,
and checks that the reason is `Timeout`. It then requests a native frame with
`App.Window.request_frame`; the callback returns an effect that resolves an Eio
promise with the rendered revision. Awaiting that promise checks that rendering
has caught up before the completion effect closes the window. The final marker
is `GPUIO_TOAST_PUBLIC_OK`.

The reference is diagnostic bookkeeping, not reactive application state. This
check covers native expiry, callback delivery, keyed removal, rendering, and
shutdown. It does not test saving a file, all stack positions, or every hover and
focus pause condition. Manual interaction can change the first observed reason,
so run the diagnostic without manipulating its window.

## Build, run, and adapt

From the repository root, use the configured repository toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/toasts/main.exe
_build/default/examples/toasts/main.exe
_build/default/examples/toasts/main.exe --self-test
```

These executions require a graphical session and the native backend; they are
not headless unit tests. See [development setup](../../docs/development.md).
The Dune stanza uses `ppx_jane` and `bonsai.ppx_bonsai` for the reactive syntax.
For real saves, perform the save in an owned asynchronous task and inject `Add`
after success, carrying meaningful message data in the model. Keep native
callbacks as effects and keep the rendered content derived from that model.
