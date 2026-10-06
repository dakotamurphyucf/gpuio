# Canvas study: immutable items and a validated scene

[canvas_study.ml](canvas_study.ml) and its [interface](canvas_study.mli) are the pure
application model behind the gallery's [Canvas page](../canvas_page.md). They create
three editable shapes on a dotted paper surface and accept a native move as a new
immutable scene. The module creates no Bonsai graph, native registration, window,
I/O task or resource scope. Its [Dune library](dune) links Core/GPUIO and enables
Jane Street PPX; the consuming page supplies the runtime.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Canvas & drawing**, select or drag Orbit, Prism and Tile, then reset the
scene. There is no independent executable or asset file for this model.
`--trace-canvas` belongs to the page and logs queued public observations. Optional
background launch supports layout work, not foreground keyboard validation.
Toolchain prerequisites are in the [development guide](../../../docs/development.md).
No new build, graphical, VoiceOver or Linux desktop acceptance is claimed here.

Read the helper constructors, `Item`, `build`, `initial`, then `move`. The public
`Item.t` and model `t` are abstract. Internally an item stores its typed ID, name,
local-to-world transform, drawing and local hit region. The model stores both the
item list and the corresponding validated `Canvas_scene.t`, preventing the page
from assembling inconsistent copies of those representations.

`Item.id`/`name` expose identity and label. `Item.position` applies the transform
to local origin (0,0), producing a world-space point in logical pixels; it is not a
device-pixel coordinate or a bounding-box corner. `items` returns the editable
items, `find` performs typed `Item_id.equal` lookup, and `scene` returns the drawing
snapshot. These ordinary functions do not register the scene natively.

## How `build` keeps drawings, labels and hit regions together

`build` creates 12×5 small noninteractive grid ellipses with item IDs 1000–1059.
For each model item it constructs an interactive shape and a separate text label.
Shape IDs are the model IDs; labels use item/resource IDs 100 plus list index.
The same transform moves both shape and its label. The shape's
`Canvas_scene.Interaction.create` stores the name, matching local hit region and
explicit draggable/activatable policies. Grid/labels omit interaction rather than
competing with the shapes for native selection.

`Canvas_scene.create` receives grid followed by each shape/label pair, in native
back-to-front order. The initial scene has 66 drawing items: 60 grid dots and six
shape/label items, while `items initial` has only three editable items. Public
constructors validate unique IDs, supported transforms, geometry bounds and resource
identity; [canvas_scene.mli](../../../lib/core/canvas_scene.mli) states the limits.
`Canvas_resource.path` and `text` are pure scene resources collected during scene
construction, not handles to allocated native registrations.

This syntax is error composition, not Bonsai reactivity:

```ocaml
let open Or_error.Let_syntax in
let%bind shape = S.Item.create (* validated shape arguments *) in
let%map label = S.Item.create (* validated text arguments *) in
[ shape; label ]
```

`let%bind` propagates a failed constructor; `let%map` transforms a successful final
result. `Or_error.combine_errors` collects per-item construction failures before
building the scene. The convenience `ok = Or_error.ok_exn` is used for fixed demo
geometry/colors; `move` itself returns validation errors to its caller.

`initial` uses a 60×60 local ellipse for Orbit, a closed four-point diamond path
for Prism and a 60×60 rectangle for Tile. Their hit regions match those geometries.
IDs 1/2/3 and translations (110,130)/(290,100)/(470,160) establish identity and
initial world positions. The diamond's explicit Close command satisfies the filled
path contract. The single-line labels sit at local origin (-22,42), so scene moves
must also satisfy the text transform restrictions.

## An absolute move, then publication by the caller

`move model id transform` first requires an existing item. It functionally replaces
only that item's **complete transform**, leaves every other item unchanged and
passes the new list through `build`. It neither mutates the old model nor adds the
transform as a delta. A shape can support an affine transform that its text label
cannot: rotation, for example, fails whole-scene construction here because labels
support translation/positive uniform scale. Geometry outside the public coordinate
domain also fails. See [canvas_geometry.mli](../../../lib/core/canvas_geometry.mli)
and [canvas_resource.mli](../../../lib/core/canvas_resource.mli).

A concrete gallery trace: Orbit emits native `Moved (id, transform)` after manipulation.
The page calls `Study.move source.model id transform`. If that transform translates
to (180,144), the new model's positions are Orbit (180,144), Prism (290,100), Tile
(470,160); `initial` remains unchanged. The page then calls `Registered.set` with
`Study.scene model`, stores its model only after local success and updates a Bonsai
notice. Native publication and painting occur asynchronously. The absolute
transform incorporates movement without applying the native displacement twice.
Native hit testing/drag preview belong to GPUIO; source ownership and effect handling
belong to [canvas_page.ml](../canvas_page.ml).

The [existing expect test](../../../test/gallery/gallery_test.ml) named “canvas
moves preserve other shapes and reject invalid whole scenes” checks those positions,
a missing ID, an out-of-bounds translation, unsupported rotation and unchanged
initial state. It is evidence of the intended pure test cases, not a claim that a
new test or GUI run occurred for this documentation review.

For a small adaptation, add a shape tuple to `initial` with a matching drawing and
hit region. Keep shape IDs unique and avoid collisions with label/grid IDs; update
the scene description that currently says three shapes. Labels derive IDs from
list positions, so the current fixed-order fixture is not a general reorderable
identity scheme: introduce stable label IDs if dynamically reordering items.
Keep `build` as the validation boundary and return its error before publishing.
The model requires no cleanup; the caller's page-visit scope owns native lifetime.
