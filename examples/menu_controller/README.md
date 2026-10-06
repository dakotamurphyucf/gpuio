# Positioned menus

Read [the component walkthrough](component.md), then [component.ml](component.ml)
for Bonsai state and GPUIO composition. [main.ml](main.ml) only launches the window.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/menu_controller/main.exe
./scripts/gpuio exec _build/default/examples/menu_controller/main.exe
./scripts/gpuio exec _build/default/examples/menu_controller/main.exe --drawn
```

Default presentation uses AppKit on macOS and the drawn fallback on Linux.
`--drawn` uses the in-window renderer on both platforms.
