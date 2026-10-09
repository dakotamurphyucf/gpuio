# A typed, deterministic scene model

[plot.ml](plot.ml) builds simulated evaluation data and its scene;
[plot.mli](plot.mli) exposes abstract Plot/Sample values, lookup/move and scene
construction. Read geometry/ID helpers, Sample, create/move, then scene. The
[application walkthrough](main.md) owns registration, effects and interaction;
this module has no Bonsai graph, native handles, I/O, callbacks or executable.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/canvas/main.exe
_build/default/examples/canvas/main.exe
_build/default/examples/canvas/main.exe --large
```

[README](README.md), [dune](dune), [development](../../docs/development.md) and
[platform policy](../../docs/platform-release-policy.md) cover dependencies,
toolchain/display prerequisites and diagnostic commands. No external data or
assets are needed. Large is a workload fixture, not a performance guarantee.

`Sample.t` contains typed scene item ID, name, color and transform. `position`
applies transform to origin; interface describes these world coordinates as
latency x and inverted quality y, not calibrated units. Plot.t contains four
samples plus large flag. `create` gives stable IDs 1..4 to Swift/Sage/Atlas/Orbit
with fixed colors/positions. `find` uses typed Item_id.equal. `move` fails for
absent ID or immutably replaces that sample's transform; it does not increment
native resource epochs or apply a drag command itself.

`scene` constructs validated immutable `Canvas_scene.t`: fourteen grid rectangles
(IDs 10..16 and 20..26), six text items/resources 30..35, decorative point cloud
IDs 100 upwards and four interactive sample ellipses. Default 160 cloud points
means 184 total items; large 19,000 means 19,024. IDs and text resource contents remain
stable across moves, allowing same-generation native scene updates. Point positions
use deterministic modular arithmetic, not random or measured evaluations.

Geometry helpers validate finite points/rectangles/transforms. Clip rectangle
64,64 with size 580×340 bounds clouds and samples. Decorative ellipses have no
interaction metadata. Each sample gets an ellipse hit region, accessible label,
draggable/activatable flags, transform and fill/stroke. Text resources preserve
independent resource IDs. Scene description combines static explanation with
current sample positions for accessibility. Validated constructors and
Or_error.ok_exn treat bad fixture definitions as programming failures.

For a move trace, native canvas finishes a drag and sends Moved(id,transform).
The application calls Plot.move and Plot.scene, then publishes once using the
scoped adapter. Preview frames remain native. This module rebuilds a bounded
complete scene, not per-frame OCaml geometry; in large mode each move reconstructs
19024 items, so do not claim unmeasured throughput. Scene publication/admission
is distinct from native painting and physical display.

To add another sample, extend create with a unique ID that cannot collide with
axes/text/cloud IDs and choose coordinates inside intended clipping. Match
application selection against the new dataset on removals; the current app
assumes selected IDs name these four samples. Keep resource identity/content
stable unless deliberately resetting native generation. Public contracts:
[Canvas_scene](../../lib/core/canvas_scene.mli),
[Canvas_geometry](../../lib/core/canvas_geometry.mli),
[Canvas_resource](../../lib/core/canvas_resource.mli).
