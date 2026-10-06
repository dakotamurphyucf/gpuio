# Opening a menu from an OCaml effect

This component demonstrates explicit popup positioning, closing, command routing
and safe rejection of a request captured before its menu definition changes.
The [README](README.md) contains build/run commands and platform behavior.
[The startup walkthrough](main.md) explains presentation flags and runtime ownership;
[component.ml](component.ml) contains the example's state and views.
The [interface](component.mli) exposes the component constructor.

## State and responsibilities

`Controller.create window graph` allocates a stable Bonsai key and stores the
latest native menu observation. Pass that key to `V.context_menu` and pass
`Controller.observe controller` as `on_change`. This connects a mounted menu to
its commands; no Rust handle or callback appears in application code.

A Bonsai graph is the persistent computation structure constructed by `create`.
`B.state` allocates count `0`, replacement `false`, observing `true`, status
`Ready` and released `false`; `B.state_opt` starts saved snapshot and artwork
absent. `B.toggle ~default_model:true` owns icon visibility. Each returns a
reactive value and a setter/toggle effect. Reactive values change over the
graph lifetime; `let%arr` reads their current ordinary values and derives the
View. Its `and` bindings list dependencies, not parallel threads. State allocation
occurs outside that derivation, so rendering does not reset it. Constructing
effects does not execute them. The
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
result. `report` uses `E.Let_syntax`/`let%bind` to wait for that asynchronous result,
converts a `Menu.Command_error.t` with its sexp printer, then returns a status
setter effect. The local `command` helper applies this to the primary controller.
The `decorate` helper adds icon metadata to a context-menu view only when
artwork exists and platform presentation/icons are enabled. The Close button
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

## Decorative native icons

`B.Edge.lifecycle ~on_activate` registers an activation effect with the graph.
Its `let%arr` reads current setters; `E.Let_syntax` and `let%bind` run the
asynchronous registration and wait for its result. The activation effect constructs an encoded SVG and registers it with
`Gpuio_eio.Asset.register` in the window scope. The source is an in-memory
triangle, so this example performs no file I/O. The `Ok asset` result becomes
Bonsai state through `set_artwork`; `Error error` updates status instead; its handle is passed to `View.with_menu_item_icons` at path `[0; 1]`
(first menu, second item: Run). Both menu owners use the same registered asset.
The default platform presentation supplies a 16-point AppKit template icon;
`--drawn` demonstrates the ordinary controller without platform icon metadata.
On Linux the default platform presentation uses the drawn icon-and-label fallback.

Hide menu icons removes the decorative slots. Show menu icons mounts new ones.
Release icon source retires the registration but leaves existing native readers
alive. Opening an already mounted menu still shows its retained icon. Hiding and
showing icons after release creates new readers, which cannot acquire the retired
source; the menu remains usable with its text and actions. A tracking popup keeps
its ready bitmap snapshot until dismissal. Registration completion means encoded
publication, not completed decoding: an early open may have no icon yet.

The ordinary state, controller snapshots and editor ownership remain independent
of the artwork. Application code neither parses SVGs nor holds NSImage pointers.
The window scope releases an unreleased registration when the window closes.
Registration failure is displayed in the status text while the menu remains usable.

`python3 scripts/test_native_menu_icons_macos.py --output scratch/menu-icons`
checks real AppKit pixels, clear/remount, release during tracking, retained versus
new readers, and command selection. It also accepts `--binary` for an installed
consumer. Its screenshots cover the owned popup rectangle; this is separate from
VoiceOver, Linux desktop and performance qualification.

The [asset interface](../../lib/eio/asset.mli) specifies that cancellation
suppresses user completion and releases any late allocation. The asset lives
until explicit release or window-scope end; its handle is not persistent identity
and does not extend registration lifetime. `released` disables the release
button after its first activation. `Asset.release` is idempotent asynchronous
retirement. This example deliberately exercises retired decorative bindings;
ordinary applications should remove them and avoid creating new bindings from
a released handle. There is no application streaming, worker task or polling;
registration and correlated commands use the runtime adapters.
