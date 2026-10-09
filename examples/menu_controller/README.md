# Positioned menus

Read the [startup walkthrough](main.md), then
[the component walkthrough](component.md) and [component.ml](component.ml)
for Bonsai state and GPUIO composition.
The [two-window walkthrough](multiwindow.md) explains independent controllers,
native editor targets and window activation.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/menu_controller/main.exe
./scripts/gpuio exec _build/default/examples/menu_controller/main.exe
./scripts/gpuio exec _build/default/examples/menu_controller/main.exe --drawn
./scripts/gpuio exec _build/default/examples/menu_controller/main.exe --two-windows
```

Default presentation uses AppKit on macOS and the drawn fallback on Linux.
`--drawn` uses the in-window renderer on both platforms.

There is no built-in self-test flag. The companions describe separate macOS
foreground diagnostic drivers and their evidence boundaries. No network service
or file assets are required; the single-window demo embeds its SVG icon source.
