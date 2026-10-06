# Build a pure three-stage canvas scene

[run_diagram.ml](run_diagram.ml) and [run_diagram.mli](run_diagram.mli) define
simulated stages, world-space positions and canvas scene construction. This is
pure application geometry: no Bonsai graph, native registration, task or file
operation exists here. [Diagram](diagram.md) owns the scoped scene/controller.

From the repository root with [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Explore run diagram; move/select/open a stage and compare the text stage
list. The simulation reads/modifies no checkout files. macOS is the v1 target;
Linux GUI qualification is [informational](../../../docs/platform-release-policy.md).

## Stages, IDs and translation invariants

Read `Stage`, `create`, `transform`, `position`, `move` and `scene`. Stage is
Read | Draft | Review with names/descriptions, stable Canvas_scene.Item_id values
1/2/3 and a cyclic `next` for detail navigation. `of_id` uses typed ID equality
and returns None for unknown items. Scene drawing links are distinct IDs 10/11;
resource IDs belong to a different type/namespace from item IDs.

The abstract `t` is internally an association list mapping every Stage to a
validated Transform. `create` places stages at x = 34, y = 42/136/230, in
world-space logical pixels. `transform` can use find_exn because callers cannot
construct a missing-stage representation. `position` applies the transform to
local origin (0,0); it is not a device-pixel coordinate or viewport position.

`move t stage transform` validates the resulting position, rebuilds the matching
translation and compares it using Transform.equal. Rotation/scale/skew are
rejected: these labels only support translation. It additionally requires
absolute x/y ≤ 999,000, reserving 1,000 pixels inside the canvas coordinate domain
for bounds/connectors/arrowheads. Invalid moves return Or_error without changing
the old model; valid moves return an immutable updated association list.
Read [canvas geometry](../../../lib/core/canvas_geometry.mli) for coordinate/
transform bounds.

## Text nodes and noninteractive connectors

`scene t ~palette ~generation` constructs a complete declarative scene. Optional
annotation Empty uses palette.accent; a Color converts the accepted RGBA without
dropping alpha. The caller must supply a positive resource generation that
increases when resource contents change; this pure function does not allocate
that counter or register the result natively.

For each stage, `Canvas_resource.text` builds a resource with ID 1/2/3, size 16,
weight 600 and numbered label. A local rectangle (-10,-8,260,40) provides a hit
region larger than the text. `Canvas_scene.Interaction.create` supplies stage
accessible label, draggable/activatable flags. The scene item combines the
stable stage ID, current translation, interaction and text drawing in palette
text color. Public [resource](../../../lib/core/canvas_resource.mli) and
[scene](../../../lib/core/canvas_scene.mli) constructors validate those descriptors.

Two links connect Read→Draft and Draft→Review; there is no drawn Review→Read
edge even though detail `next` cycles. Each uses positions to build a line and
small arrowhead path, resource IDs/items 10/11 and the supplied path generation.
A 1.5-pixel stroke uses the resolved annotation. Links have no interaction
metadata; opening/selecting stages belongs to the text nodes. `Scene.create`
places links before nodes so labels paint above lines and supplies a simulation
accessibility description pointing to the equivalent stage list below.

## Trace a move and a style update

Native dragging eventually reports a complete resulting Transform through
Diagram.on_event; `move` validates it, the controller stores the new model and
builds another `scene`. Connectors are recalculated from accepted positions,
while stage/item identity remains stable. A changed annotation/theme creates a
new scene from the same geometry with new colors/path generation. The native
resource/controller handles publication and mounted view separately; a pure
scene value is not a live widget or physical paint result.

A small adaptation is another stage: extend Stage.all/name/description/id/next,
add a unique item/resource ID and explicit desired link topology. Keep create's
complete stage mapping and translation-only rule, and reserve coordinate margin
for the largest geometry you add. Use another domain model if stages must own
real job status; these descriptions are static and never orchestrate Eio work.
[Diagram](diagram.md) and [Inspector](inspector.md) explain observations, command
acknowledgements and route replacement; this model review runs no GUI test.
