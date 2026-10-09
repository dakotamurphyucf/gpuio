# Connecting Bonsai values to the gallery shell

[`Component.component`](component.ml) is the gallery's reactive composition
layer. Its [interface](component.mli) lists supplied services, app/window handles
and observable application variables. It turns them into page content and
`Shell.Snapshot` / `Shell.Actions` values. It owns no layout implementation or
direct filesystem operation.

Build and launch using the [application guide](application.md). Switch the
appearance, choose **Follow system**, change logical size or select a page.
These are the interactions implemented here. Native macOS checks and Linux
build coverage remain distinct; this guide adds no platform acceptance claim.

Read `effective_appearance`, `palette`, the theme-change hook, `Pages.component`,
then the final `let%arr`. The supporting source interfaces are
[theme selection](model/theme_selection.md),
[appearance](model/appearance.md), [Palette](palette.md),
[Pages](pages.md) and [Shell](shell.md).

`Bonsai.Expert.Var.value` observes variables created at window startup. `let%arr`
combines current values into a derived result; it is not an event handler.
`Theme_selection.resolve` chooses explicit Light/Dark or resolves a System
preference from the latest native snapshot, falling back to Dark before one
arrives. The gallery initially chooses explicit Dark. A loaded profile supplies
its own colors; the logical size choice remains independent.

The resulting `Palette.t` supplies both concrete view colors and a native theme.
`Bonsai.Edge.on_change` compares themes with typed `Theme.equal`, issuing
`App.Window.set_theme` only on a change. This keeps native widgets in agreement
with the application's drawn views without calling the window API while merely
constructing a view. It is not a per-frame polling loop.

`Pages.component` receives the page value, palette and required service
capabilities. The final `let%arr` combines its content with current state and
constructs effects for `Shell.Actions`. `Effect.of_thunk` delays ordinary
application mutations until an action runs. Native fullscreen/minimize/zoom
commands return asynchronous results; this demo discards those results, so an
application needing error feedback should handle them explicitly.

For example, click **Follow system**. The shell activates its supplied effect;
`Theme_selection.choose` stores a new System preference and retires any loaded
profile selection. Bonsai recomputes the effective appearance and palette. The
theme hook updates native theme tokens, and `Shell.view` receives new colors and
button state. A later native appearance observation repeats that derivation
without changing the user's System preference. View acceptance is not a physical
presentation timestamp.

Clicking a page button similarly changes only the page variable. Page branching
and scoped widget lifecycle belong to [Pages](pages.md); this component does not
globally reset every model. Scale changes also derive new presentation values;
they do not simulate display DPI. The app-level motion variable is shared across
windows, whereas theme and scale variables are per-window.

To add a global **Reset zoom** control, define its desired model/command at the
appropriate owner, add an effect to `Shell.Actions`, and render its button in
the shell. Keep I/O in a supplied Eio service and invoke it through a scoped
component task if needed. Do not place effects inside pure palette construction
or mutate state from a view-building expression. For a simpler version of the
same separation, read the [counter](../getting_started/README.md).
