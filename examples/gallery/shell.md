# Stateless navigation, layout and window chrome

[`Shell.view`](shell.ml) draws the gallery frame from ordinary inputs. Read the
[interface](shell.mli) first: `Snapshot.t` contains current display state,
`Actions.t` contains effects, and `content` is the already-composed active page.
There is no Bonsai graph, mutable model, Eio task or resource acquisition here.

Use the [application build/run commands](application.md). The ordinary window
shows a navigation rail, page heading and appearance/size/window buttons. To see
the optional custom title bar, launch:

```sh
./scripts/gpuio exec _build/default/examples/gallery/main.exe --custom-chrome
```

This requests custom chrome, not a different application model. Capabilities and
actual native window observations determine which window controls are available.
Compilation is not proof that those controls behave on every desktop platform.

`Snapshot` distinguishes effective appearance from the user's theme preference.
It also carries optional native state/capabilities, because the initial component
can precede the native handshake. `Actions` makes effects explicit; `view` passes
them into controls without executing them. [Component](component.md) supplies
the effects and [Application](application.md) owns the window.

Read `navigation`, `workspace`, then the custom-chrome branch. Navigation is a
list built from [`Page.all`](model/page.mli), using stable variants and titles.
The rail has its own scrolling region. The workspace's content uses
`Grow`, `Min_width 0` and `Min_height 0` so it can shrink and scroll inside a
bounded window instead of forcing an oversized flex child.

The header also wraps. Its heading column uses a 320-logical-pixel `Basis`,
`Grow 1` and `Min_width 0`: it takes spare space while allowing the description
to wrap. Appearance, scale and New window controls form a separate row with
`Shrink 0` and `Max_width 100%`. The outer row can move that toolbar below the
heading; the toolbar can wrap its own buttons when necessary. This keeps long
page descriptions and Large-scale text from pushing controls beyond the window.
It uses ordinary layout styles, without a Bonsai breakpoint model or resize
callback. The active page's model and editor owners are unaffected by reflow.

The page scroll container has a key derived from `Page.key`, independent of its
displayed title or list position. Changing the page therefore changes this
native container's identity. That does not independently describe how every
Bonsai model is retained; page/resource behavior belongs to [Pages](pages.md).
The container has a named accessibility group, **Component preview**, useful
both to users and native test traversal. A label alone is not a screen-reader
qualification result.

With custom chrome, `View.title_bar` uses the backend capability and fullscreen
state. Before capabilities arrive, a plain row is drawn without native drag
regions. The fullscreen button checks the current presentation's allowed
controls. `View.window_controls` receives the snapshot and minimize/zoom/close
effects; no OCaml handler implements window dragging or native layout itself.

Follow **New window**: this pure function installs `actions.open_window` on a
button; a later native click delivers that effect, and the application creates
the window. The shell does not become stateful merely because it accepts an
effect. A selected navigation button likewise represents a snapshot rather than
mutating selection during construction.

To narrow the navigation rail, change its 236-logical-pixel width and review the
padding/text constraints. To add a control, extend `Actions` and supply the
effect in `Component`; keep the shell's rendering deterministic from its inputs.
The [Palette helpers](palette.md) share theme styling without requiring inheritance
or a separate widget framework.

For the focused macOS header-layout check, build the gallery and run
`python3 scripts/test_gallery.py --section header-layout --images scratch/header`.
The check uses actual window/control bounds, captures representative renders,
and clicks New window. It does not qualify every page's internal layout or
screen-reader behavior.
