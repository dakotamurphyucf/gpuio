# Signal Studio

A small, simulated model-evaluation workbench combining an OCaml canvas, native
charts and the independently packaged `gpuio_example_counter` component. It uses
the public Core/Bonsai/Eio and extension SDK APIs. The agent-chat example is a
separate application and is unchanged.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe
./scripts/gpuio exec dune exec examples/signal_studio/main.exe
```

Select a sample, drag its point or use Shift+Arrow to adjust latency and quality.
Scroll inside the canvas to pan; Control+scroll zooms. The chart updates from the
same immutable OCaml workspace. Click plotted marks or use Home/Arrow/Enter with
the chart focused to select a chart value. `View data` exposes original values.
The run control changes the simulated signal; `Stream runs` publishes twelve
updates, and `Pause stream` cancels the current task. No network or LLM service is
used. Values are illustrative rather than measurements of actual models.

Resize to switch between side-by-side and stacked plots. The inspector uses a
native spring, the run status uses an opacity sequence, and both activity labels
share a native animation clock. The component can be locked, hidden or remounted
by resetting the workspace. Native canvas/chart handles belong to the application
scope and survive responsive branch changes. OCaml stores accepted point
positions; pan/zoom stay in the native view.

The pure `model/Workspace` module also provides a validated, versioned document
codec and a strict sample-link decoder. Desktop, file and notification workflows
are still being integrated; those codecs alone do not implement OS integration.
See [design and acceptance](../../docs/design/signal-studio.md).

## Local checks

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe @examples/signal_studio/model/runtest @fmt
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --self-test
python3 scripts/test_signal_studio.py --output scratch/signal-studio
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example signal_studio --run
```

The AppKit script requires macOS Accessibility permission and briefly uses the
foreground window. It restricts input to its child process, releases mouse buttons
on failure and terminates/reaps the child. Screenshots and logs go to the requested
output directory. The self-test uses public resource publication, semantic command
acknowledgement, a native render callback and explicit resource release; it does
not replace physical input or pixel validation.

The consumer check stages public libraries into a fresh prefix, copies this app
and the separate component, and builds the locked composed backend. It never
installs packages into or changes the current opam switch. Omit `--run` for the
Linux build-only gate. Full Linux GUI validation remains tracked by OCH-17.
