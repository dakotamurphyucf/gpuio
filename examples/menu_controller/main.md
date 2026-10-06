# Starting one positioned-menu demo or two independent windows

[main.ml](main.ml) is the application entry point. It contains no application
state reducer or menu implementation: [Component.create](component.md) builds
the single-window demo and [Multiwindow.create](multiwindow.md) builds each
independent window. Their interfaces are [component.mli](component.mli) and
[multiwindow.mli](multiwindow.mli). Read this launcher, then the chosen component.
The [README](README.md) contains exact isolated-toolchain build/run commands;
[dune](dune) links Core, GPUIO, its Bonsai/Eio adapters, Bonsai and `eio_main`, and
enables Jane Street/Bonsai PPX for the component modules.

`Array.exists (Sys.get_argv ()) ~f:(String.equal "--drawn")` checks arguments
using typed string equality. `platform` defaults true; `--drawn` makes it false.
Platform context presentation means AppKit on macOS and the drawn fallback on
Linux. `--two-windows` selects the second branch; both flags can be combined.
There is no `--self-test` mode here. The scripts linked from the component guides
are separate macOS foreground diagnostics. No file assets or network service
are required; the single-window component registers an in-memory SVG.

`Gpuio_eio.App.run (fun _env app -> ...)` owns GPUI on the OS main thread and
starts the OCaml Eio UI domain where initialization, component graphs and effects
run. `_env` is unused because this launcher starts no file/network/timer task.
The single-window branch calls `App.open_window` with 660×700 logical-pixel
geometry and the partially applied `Component.create ~platform ~app`; the runtime
supplies that factory with its window and Bonsai graph. The app argument lets the
component register its decorative asset in the window scope.

The two-window branch uses `List.iter [ "A"; "B" ]` to open 540×640 windows with
`Multiwindow.create ~name ~platform`. Each factory invocation creates independent
Bonsai state and native controllers. The shared names/command-ID definitions are
immutable values, not shared editor or popup resources. `Or_error.ok_exn` treats
failure to open these fixed demo windows as a startup error; an application may
instead handle that result. The annotated `App.Window.t` is intentionally ignored:
the runtime retains ownership after opening.

When a native button invokes Run, the window's registry callback returns its
Bonsai setter effect, dependent views change, and GPUIO submits the new native
transaction. The launcher itself participates only in establishing the runtime
and window ownership; detailed event/command traces belong in the components.
Closing a demo window cancels its window scope and cleans its controllers and
scoped asset; the default runner exits after the final window closes. A command
acknowledgement is distinct from physical display.

To start three independent windows, extend the immutable name list in the
`--two-windows` branch and give each a distinct title. State remains local to
each invocation. To introduce shared data, create an explicit application model
and per-window subscriptions; do not reuse one window-bound controller in another
window. Setup and capabilities are documented in the
[development guide](../../docs/development.md) and
[App interface](../../lib/eio/app.mli). Current
[platform policy](../../docs/platform-release-policy.md) distinguishes required
Linux build checks from deferred Linux desktop qualification; these launch
instructions do not establish graphical acceptance on either platform.
