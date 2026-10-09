# Canvas page: accept native moves into an application scene

[canvas_page.ml](canvas_page.ml) and its [interface](canvas_page.mli) mount a native
editable scene with three shapes, selection buttons, viewport commands, hide and
disabled controls. The public component takes the application/window, a reactive
palette and a Bonsai graph, and returns a reactive GPUIO view.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
# Optional public-event logging:
./scripts/gpuio exec _build/default/examples/gallery/main.exe --trace-canvas
```

Choose **Canvas & drawing**. Drag Orbit, Prism or Tile; focus a shape and use Shift+arrows to
move it. Try **Select Orbit**, **Zoom to 125%**, both reset buttons and hide/disable.
The stage intentionally stays light in both themes. No external assets or service
are required. `--background` avoids requesting focus for layout work; real keyboard
interaction needs a focused window. These instructions and this documentation
review do not establish new macOS, Linux GUI or VoiceOver acceptance. Toolchain
prerequisites are in the [development guide](../../docs/development.md).

Read the pure [Canvas_study interface](model/canvas_study.mli) and
[implementation](model/canvas_study.ml), then `Source`, then `component`. The model
uses world-space logical pixels. `Study.initial` places Orbit at (110,130), Prism
at (290,100) and Tile at (470,160); stable item IDs are 1, 2 and 3. Its scene also
contains grid dots and labels under separate IDs. `Study.build` combines drawings,
hit regions and `Canvas_scene.Interaction` records; the three named shapes are
selectable, draggable and activatable. `Study.move` replaces one item's complete
transform and rebuilds a validated scene, preserving identity. An unknown item is
an `Or_error`, not silently appended. The page does not reproduce native hit tests.

`Source.t` owns a `Gpuio_eio.Canvas.t`, the accepted application model and a command
sequence starting at zero. `Source.create` maps the asynchronous registration effect
into this owner. `publish ~reset:false` uses `Registered.set`; `~reset:true` uses
`Registered.reset`. It changes `source.model` only if the local operation succeeds.
Local acceptance does not prove native publication or physical paint; the
[registration contract](../../lib/eio/canvas.mli) explains coalescing and errors.
`Source.command` increments the sequence and calls `Canvas.Command.create`, updating
the counter only when construction succeeds.

## Bonsai state versus the mounted canvas

`Preview_scope.acquire` creates one child scope per active page visit and produces
Loading, Failed or Ready. The page handles these variants before borrowing the
scene handle. Read its [walkthrough](preview_scope.md) for acquisition, cancellation
and rejection of late results.

`B.state` allocates reactive selection, command, zoom, notice and last-activation
values together with setter effects. `B.toggle` does the same for hidden/disabled
Booleans. The graph constructs these once; `let%arr` reads their current values
and derives the view. A setter is a `Bonsai.Effect` scheduled by a native handler,
not an assignment to perform while deriving the view. `E.bind` sequences an effect
and uses its result; `E.Many` groups the lifecycle resets.

`V.canvas` keeps key `gallery-canvas` and borrows `Registered.handle source.canvas`.
`Canvas.Config.create` supplies an accessible label, disabled policy, selection
color and optional sequenced command. The surrounding `Style` sets the stage size
and hidden display. Native code owns pan, zoom, selection, gesture preview and
position overrides; the application owns persistent scene transforms. World-to-view
mapping and command replay rules are in [canvas.mli](../../lib/core/canvas.mli),
with geometry/scene invariants in
[canvas_geometry.mli](../../lib/core/canvas_geometry.mli) and
[canvas_scene.mli](../../lib/core/canvas_scene.mli).

## Trace a move and an explicit command

Drag Orbit. Intermediate movement stays native; a queued `Moved (id, transform)`
observation reports the resulting local-to-world transform. `on_event` runs an
`E.of_thunk` that calls `Study.move`, then `Source.publish ~reset:false`. The
`report` continuation sets a notice containing the new world position. That reactive
notice triggers view derivation, including the description read from the updated
application model. The new scene incorporates the absolute transform, so movement
is not added twice. A same-generation publication retains identity; updating the
source transform resolves the corresponding native override. Failed validation or
publication requests display an error instead of modifying the stored model.

Click **Zoom to 125%**. `issue` constructs `Set_viewport` with origin (0,0), zoom
1.25 and a fresh sequence; `set_command` places it in the next config. Native code
applies it once, reports `Viewport_changed` to update the zoom readout, and reports
`Command_completed` to update the notice. Reusing that same config does not replay
it. Selecting a shape through a button likewise sends `Select (Some id)`; the
readout changes after `Selection_changed`, not by assuming the command succeeded.
A failed native command consumes its sequence, so a retry must use a larger one.

**Reset canvas view** is a viewport command. **Reset canvas scene** republishes
`Study.initial` with `reset:true`, clears OCaml selection and starts a new resource
generation. A new scene publication cancels an unfinished gesture. Hiding the stage
keeps its scope/model; disabled input does not remove the source and explicit
commands can still apply. Leaving the page cancels its scope, releases the native
scene and clears selected/command/notice/zoom/activation via `B.Edge.lifecycle`.
Hidden/disabled preferences remain retained; returning creates a fresh model and
command counter, so it cannot borrow the released scene handle.

The page performs no file/network I/O and starts no polling loop. The Eio runtime
owns asynchronous registration and queued event delivery. `--trace-canvas` prints
event sexps, `Registered.is_published` and observed window activity for diagnostics;
it does not establish physical presentation or alter the scene policy.

To add a fourth shape, extend `Canvas_study.initial` with a drawing, matching hit
region and unique identity, then retain that identity across moves. To change zoom,
construct a validated viewport and issue a new sequenced command instead of changing
only the readout. Keep the UI-domain source owner and lifecycle scope together;
a borrowed handle cannot extend a cancelled registration's lifetime.

The pure model has its own [Canvas study walkthrough](model/canvas_study.md),
including drawing/hit-region construction and whole-scene transform validation.
