# Starting Component Studio and owning its windows

This is the gallery's application layer: runtime startup, explicit Eio
capabilities, application services and window lifetimes. Read
[`main.ml`](main.ml), the [Application interface](application.mli), then
[`Application.run`](application.ml). Continue with the
[reactive component](component.md) and [stateless shell](shell.md).

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

The first window opens on Presentation with a Dark, Comfortable appearance.
**New window** opens an independent gallery window, up to four live windows.
Page selection, appearance and scale are per-window; the motion preference and
desktop session are application-owned. This is a macOS-first example with Linux
build/consumer coverage; see the [platform policy](../../docs/platform-release-policy.md).

## Entry-point modes

`main.ml` only dispatches startup modes. `--check-catalogs` verifies that the
linked backend registers the expected example counter and document-profile
schemas. It prints through Eio without opening a window. `--print-info-plist`
prints package metadata for `gpuio-studio`; it does not package, sign or install
an application. Normal execution calls `Application.run`.

```sh
./scripts/gpuio exec _build/default/examples/gallery/main.exe --check-catalogs
./scripts/gpuio exec _build/default/examples/gallery/main.exe --print-info-plist
```

The [Dune file](dune) links the public GPUIO libraries, model/file/sample libraries
and the example's statically composed backend. The catalog checks are useful for
detecting an incorrectly linked backend. Ordinary application composition here
needs no Rust code; writing a new native extension is a different audience, covered
by the [extension package](../extension_package/README.md).

`Application.run` also recognizes these ordinary launch options:

| Option | Effect |
| --- | --- |
| `--background` | Requests windows without initial focus; does not establish foreground-input test coverage. |
| `--custom-chrome` | Uses the gallery's custom title bar and capability-aware window controls. |
| `--open-uri=URI` | Supplies a startup link to the desktop application service. |
| `--document-defaults` | Demonstrates Markdown selection export and a larger paragraph gap. |
| `--trace-windows` | Logs close requests while allowing normal closure. |
| `--trace-window-appearance` | Logs changes in observed native appearance. |

These modes differ from the optional external GUI acceptance driver. Starting
the gallery does not run that driver or its benchmark/keyboard sequences.

## Services and state ownership

`check_catalogs` runs before desktop startup. `App.run_desktop` then applies the
application identity, startup links and document defaults. Its callback receives
an Eio environment and app handle. The environment's filesystem, secure random
source and monotonic clock are captured explicitly by service functions:

- `save_settings` calls the [settings-file adapter](files/settings_file.mli)
  using `Eio.Path` and the secure random capability.
- `load_theme` calls the [theme-file adapter](files/theme_file.mli).
- `search_palette` is a delayed mock search, not a network connection. Its
  [walkthrough](external_palette_preview.md) explains the caller's task scope.

The application prepares edit filters, constructs a
[`Desktop_session`](desktop_session.mli), and holds the shared motion preference.
It passes these services into components; it does not perform file work in a
view constructor. The private window list tracks only live handles. The serial
names new windows; closing one does not reuse its title number.

`open_window` creates fresh Bonsai variables for page, theme selection, logical
scale and the latest native window snapshot. These variables are observable
inputs to the [component](component.md), not a parallel copy of editor text or
widget internals. A per-window search provider gets a child of the app scope
before the native window exists. Failure to create that provider or window
cancels the scope. Once the window exists, its cancellation hook owns cleanup
of that search scope.

## Follow a window interaction

Click **New window** in the shell. The component's effect calls `open_window`,
which prunes closed handles, checks the four-window limit and mounts a new Bonsai
graph through `App.open_window`. Native snapshot observations update that
window's `window_snapshot` variable. Bonsai then derives its effective appearance
and capability-aware controls. The component does not synchronously ask native
layout or paint to execute OCaml.

Closing a window retires its scoped resources. Reopen requests call the same
bounded `open_window` function. `Desktop_session.ready` runs after the initial
window setup. `App.run_desktop` may return `Exited` after the app finishes or
`Forwarded` when desktop activation was forwarded; neither is a failure.
Other desktop startup errors are raised with their structured diagnostic.

To change the initial section, change `Page.Presentation` in `open_window`.
To share a business model across windows, create it at application scope and
pass it explicitly, while retaining separate window controllers and native
resource lifetimes. Do not share one editor/controller merely because two
windows display the same document. Raising the demo's window cap also requires
revisiting resource budgets and tests; it is not a framework-wide maximum.
