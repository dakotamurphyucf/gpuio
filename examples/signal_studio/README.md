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

`Save workspace` and `Open workspace` use native file panels and Eio I/O. Saves
atomically replace the chosen file with the submitted workspace snapshot; edits
made while saving remain dirty. Files are versioned and limited to 16 KiB. Invalid
loads leave the model intact. Save or reset unsaved changes before opening another
workspace. `Reveal file` requests the platform file manager for the saved path.
The native represented-file/edited badge is independent of file-operation success.
This example writes private files (0600); it does not preserve existing file
permissions or claim crash durability of the containing directory.

Closing the window keeps the workspace in this process. Reopening the packaged app
restores it; `Quit Studio` ends the process. There is no autosave or quit-confirmation
flow in this acceptance example, so save changes before quitting. `--exit-on-close`
is available for the interaction harness.

Packaged `gpuio-signal://sample/{1..4}` links select a known sample after readiness;
links never grant filesystem access. Generate platform metadata with
`--print-info-plist` (bundle executable `gpuio-signal`) or
`--print-desktop-entry /absolute/path/to/gpuio-signal`. Linux desktop startup uses
`--open-uris` followed by URLs; explicit `--open-uri=URL` arguments work as well.
Use `--directory=/absolute/path` to choose the file panels' initial folder.

Notifications, dedicated motion/resource workloads and final platform gates remain
in progress. See [design and acceptance](../../docs/design/signal-studio.md).

## Local checks

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe @examples/signal_studio/model/runtest @examples/signal_studio/files/runtest @fmt
./scripts/gpuio exec dune exec examples/signal_studio/main.exe -- --self-test
python3 scripts/test_signal_studio.py --output scratch/signal-studio
python3 scripts/test_signal_desktop.py --output scratch/signal-desktop
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

The desktop walkthrough creates and removes a disposable application bundle,
checks actual Launch Services cold/warm delivery and same-process window reopen,
and drives real save/open panels. It also runs a document self-test with native
edited/file metadata assertions, delayed-load races, busy admission, reset fencing,
and invalid-load preservation. Its files stay inside the disposable fixture.
