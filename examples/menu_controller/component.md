# Opening a menu from an OCaml effect

This component demonstrates explicit popup positioning, closing, command routing
and safe rejection of a request captured before its menu definition changes.
The [README](README.md) contains build/run commands and platform behavior.
[main.ml](main.ml) only selects presentation and opens a window;
[component.ml](component.ml) contains the example's state and views.
The [interface](component.mli) exposes the component constructor.

## State and responsibilities

`Controller.create window graph` allocates a stable Bonsai key and stores the
latest native menu observation. Pass that key to `V.context_menu` and pass
`Controller.observe controller` as `on_change`. This connects a mounted menu to
its commands; no Rust handle or callback appears in application code.

The count, replacement flag, saved snapshot and result text are Bonsai state.
Each `let%arr` dependency is named separately before composing the View. The
snapshot is an opaque authority for one window, node and subscription, not a
copy of the menu's content. Its identity changes when the definition changes.
The draft editor owns its text and selection natively and stays at a stable
position in the View.

`Command.Registry` maps the Run ID to a Bonsai effect. The menu contains that ID
and a passive section label. A menu command named `Show` is distinct from the
application's Run command: Show changes popup state; selecting Run invokes the
registered application action.

## Positioning and asynchronous results

`position` calls the validated `Menu.Position.create` constructor. Coordinates
are logical window content coordinates, with an upper-left origin. The two
buttons request `(100,100)` and `(400,200)`. Native placement may adjust to fit
the screen; the drawn fallback stays inside the window. These are not absolute
screen pixels.

`Controller.command controller (Show point)` returns an effect with a typed
result. `report` binds that result and updates status text. The Close button
uses the same mechanism with `Close`. Success means native admission, not that
a frame was physically presented. An overlapping Show returns Busy. A second controller owns the separate secondary
anchor; Show other owner demonstrates that AppKit's active lease also rejects a
request from a different owner. Each controller has its own key/subscription.
The native
menu still handles navigation and selection without waiting for Bonsai.

The buttons remain unavailable until the first native observation supplies an
owner identity. Removing the view should call `Controller.reset` to clear local
bookkeeping; native generation checks protect stale requests independently.
This example keeps the context owner mounted until the window closes.
Detach observer deliberately removes its callback and calls `reset`; an open
AppKit lease is cancelled. Attach observer installs a new subscription, and the
Show buttons become enabled when its first observation arrives. The editor stays
mounted through both transitions.

## One interaction, end to end

Press Show first position. GPUI queues the button action to the OCaml domain.
Its effect sends the observed identity and requested position through the bounded
bridge. Rust verifies ownership and interaction eligibility, accepts the request,
and schedules AppKit tracking (or opens the drawn fallback). The correlated reply
updates the status. Choosing Run queues a separate application-command event;
Bonsai increments the count and GPUIO publishes the resulting View update.

The Close command remains usable while AppKit tracks. Closing the window retires
native state and completes pending controller requests once. Eio runs the
application domain, but this example starts no background I/O tasks or timers.

## Why the captured request fails

Capture old menu saves a `Menu.Snapshot`, then Replace menu definition changes
the immutable menu. Show captured menu deliberately uses the lower-level
`App.Window.Expert.menu_command` with the old snapshot. It returns Stale_menu
rather than opening the replacement. Ordinary applications should use the
controller's current value; the Expert call is included to explain the lifetime
contract. Changes to a callback or style alone preserve the subscription.

For a small modification, change the second button's `position 400. 200.` to
another validated point. To add an application action, add its registry entry
and reference its stable ID in the menu. Keep the controller key and editor
placement stable across recomputations.

See [Menu](../../lib/core/menu.mli),
[Menu_controller](../../lib/eio/menu_controller.mli) and the
[command contract](../../docs/design/menu-commands.md).
The macOS driver `python3 scripts/test_menu_controller_macos.py` opens foreground
windows, checks actual AppKit geometry, Busy/Close, selection and stale-definition
rejection, and reaps its app. This is separate from ordinary launching and does
not establish VoiceOver, Linux GUI or physical frame-time acceptance.

An independent public-library build is available without changing opam switches:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example menu_controller \
  --workspace scratch/menu-controller-consumer
python3 scripts/test_menu_controller_macos.py \
  --binary scratch/menu-controller-consumer/consumer/_build/default/main.exe
```

The install prefix, copied example and build artifacts live entirely in that
workspace. Use a fresh directory for each independent build.
