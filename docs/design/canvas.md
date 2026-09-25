# Retained canvas

Status: OCH-24 implementation design in progress. The bounds below are proposed
admission limits to validate with the implementation and workload evidence, not
claims of completed functionality.

The current implementation provides validated `Canvas_geometry` and
`Canvas_path` OCaml values and matching Rust protocol geometry. The immutable
scene wire schema, bounded Rust decoder, reference/geometry admission and pure
topmost hit testing are implemented. The native session also owns a tested staged
scene registry, connected through the bridge and the raw Eio expert request lane.
`Canvas_resource` and `Canvas_scene` provide pure typed construction and owner-aware
encoding. `Gpuio_eio.Canvas` now provides scoped publication, coalesced updates,
explicit reset and release. Native rendering and canvas interaction remain in
progress; registration does not yet expose a rendered canvas widget.

## OCaml construction

`Canvas_resource` uses phantom kinds for path, text and image resources, with
distinct resource IDs and positive generations. `Canvas_scene` provides item IDs,
paint/stroke values, drawing constructors, interaction policies and immutable
snapshots. A scene collects its resources from the items, deduplicates exact
identity/data matches and rejects conflicts under the same resource ID. The
comparison includes exact canonical float bytes and image application identity.
Path bounds and closure are cached at resource construction; sharing one large
path across many items does not traverse its commands for every item.

Colors resolve against the supplied theme during scene construction. Ownership
remains attached to image references until encoding checks the application's asset
owner. Scene handles have a separate application lifetime identity; equal native
slot/generation numbers from different applications do not compare equal.
Scene equality is immutable snapshot identity for reactive cutoffs. Publishing a
new scene, retaining its registration and resetting its native generation belong
to the Eio adapter, not to these pure constructors.

For example, this constructs one interactive rectangle; it does not open a window:

```ocaml
let scene () =
  let open Core.Or_error.Let_syntax in
  let module G = Gpuio.Canvas_geometry in
  let module S = Gpuio.Canvas_scene in
  let%bind bounds = G.Rect.create ~x:0. ~y:0. ~width:160. ~height:64. in
  let%bind paint = S.Paint.create ~fill:(Gpuio.Color.rgb_exn 0x8b5cf6) () in
  let%bind interaction =
    S.Interaction.create
      ~label:"Task"
      ~hit_region:(G.Hit_region.rectangle bounds)
      ~draggable:true
      ()
  in
  let%bind id = S.Item_id.of_int64 1L in
  let%bind item = S.Item.create ~id ~interaction (S.Drawing.rectangle bounds ~paint) in
  S.create ~description:"A task diagram" [ item ]
```

## Ownership and updates

OCaml constructs immutable scene snapshots with stable item IDs, reusable drawing
resources and explicit resource generations. An Eio-scoped scene registration
owns the native resource; views borrow its application-bound generational handle.
Registering or updating a large scene uses bounded, revision-checked uploads and
atomic publication, following the existing asset/document ownership model. A
partial upload never becomes visible. Releasing registration prevents new binds;
existing views own leases until they unmount. Old snapshots remain charged while
retained, and cancellation releases staging reservations.

### Scoped publication

`Gpuio_eio.Canvas.create app ~scope scene` completes its effect only after the
first successful native publication. The application owns the registration;
`handle` returns its borrowed identity. All adapter operations run on the UI
domain. Cancelling the scope suppresses late completion and retires even an ID
allocated while cancellation was in flight. Initial publication failure reports
an error and releases the registration. Foreign image owners and unrelated scopes
are rejected locally before allocation.

`set` accepts an immutable desired snapshot. The current upload finishes, while
unstarted replacements coalesce to the latest snapshot. `reset` additionally
requests a new scene generation: coalesced resets consume just one native
generation; another reset during that upload advances one more generation after
its acknowledgment. No reset can skip the native registry's one-step rule.
The borrowed handle remains stable. Uploads occur on explicit changes, not frames.

An update rejected during Begin/Chunk/Publish preserves the last accepted scene.
The adapter aborts any staged bytes, records the typed error, and does not retry
the rejected intent automatically. A newer desired snapshot or explicit `set`/
`reset` can recover. Successful publication clears the error. Closed/stale/native
failure responses retire the registration; an abort failure is also terminal.
`is_published` compares desired and acknowledged scene/reset identity; it reports
native acceptance, not presentation. `scene` is the desired snapshot, not a query
of the last painted frame.

The scheduler has one correlated request in flight and at most four staged
uploads, with round-robin progress and cleanup priority. It admits 256 entries and
64 MiB of conservatively charged OCaml snapshots/upload buffers. Each registration
holds at most desired, accepted and uploading snapshots, deduplicated by immutable
scene identity within that registration. Cross-registration/shared-object storage
is deliberately overcharged. The scene's cached charge includes boxed geometry,
list/record metadata, resource canonical strings and an encoded buffer. Fixed
registration metadata is separately bounded by 256 entries; transient encoding
uses at most a 4 MiB Bigstring, and the one pending chunk is at most 256 KiB.
These are logical retention bounds, not a GC/RSS or native painting-cache limit.
Local setter admission failure leaves its previous desired scene unchanged.

### Planned mounted component

The canvas uses a dedicated typed view configuration so application ownership
survives until reconciliation; it must not hide a scene ID inside arbitrary
extension bytes before that check. The native view will use the existing scene
lease and the extension SDK's guarded/revocable event contract. It does not add a
process-global scene store or require a static extension factory to capture
non-Send UI state. Its small configuration references a registered scene.
Scene data changes cross the bridge; ordinary paint, native dragging and pan/zoom
do not require scene re-upload or a synchronous OCaml callback.

Native rendering owns tessellated geometry, shaped text, decoded image leases,
viewport state, selection and immediate gesture state. OCaml receives semantic
observations and may publish new scene snapshots or issue explicit commands.
Stable item identity preserves selection across updates when the item still
exists. A scene generation reset cancels interaction and resets transient state.
Removing an item cancels its gesture and clears its selection. Resource references
must match both ID and generation in the scene; missing or stale references fail
before publication. Image resources also validate their existing application asset
identity and use the established asset lease/cache limits.

The native registry admits 256 scene registrations and four simultaneous uploads.
Begin requires the exact accepted base revision, next revision, and either the
current scene generation or an explicit one-step generation reset. Nonempty
chunks are ordered and at most 256 KiB. Publish validates the complete scene and
acquires image leases before swapping the shared snapshot; failures preserve both
the published scene and staged bytes for retry or Abort. Release/close discard
staging and prevent new acquisition; existing readers retain their snapshots.

Within a scene generation, reusing a drawing-resource ID at the same generation
requires exactly equal canonical resource bytes; lower generations are rejected.
Removing a resource retains its generation/history entry. This prevents removal
and reintroduction from bypassing cache identity. History is capped at 4,096 IDs
and 4 MiB of canonical buffer capacity; explicit scene-generation reset clears
it. Higher resource generations replace an ID's prior canonical history.

Registered, staged, current and externally retained snapshot data share a 128 MiB
accounting quota. Charges include vector/string capacities and conservative
metadata allowances; old readers retain their charge until their final drop.
Publication also has a separate conservative 64 MiB temporary-capacity bound
derived from schema counts plus candidate history/validation structures, checked
at compile time for the target's type sizes. These are accounting limits, not
RSS/allocator ceilings or GPUI mesh/font/image-cache budgets. The fixed 256-slot
registry metadata also remains bounded after release. Native tessellation/cache
limits and their actual workloads remain required.

Bridge message tag 13 carries a positive correlation and canvas resource request;
event tag 39 carries its reserved response. Capability `1073741824` advertises
scene registration only (aggregate capabilities `2147483647`), not a rendered
canvas widget. `App.Expert.canvas` admits at most 63 pending raw requests, leaving
one lane for the scoped adapter. Chunk length is checked before native allocation;
responses survive input mailbox pressure. Raw callers own release and late-reply
cleanup. Successful publication schedules native redraw without calling OCaml
synchronously. App stop completes pending raw requests with Closed.

## Coordinates and drawing vocabulary

World and local coordinates are logical pixels. A matrix `(a,b,c,d,tx,ty)` maps
`(x,y)` to `(a*x + c*y + tx, b*x + d*y + ty)`. Composition applies the local
matrix first, then its parent. All coefficients and geometry are finite and
bounded; transforms used for hit testing must be invertible. Device scale belongs
to GPUI and is applied once, after the world-to-viewport transform.

Geometry constructors admit finite coordinates within +/-1,000,000, positive
rectangle dimensions up to 1,000,000, and corners within the coordinate domain.
Affine linear coefficients are within +/-1,000; the absolute determinant must
be at least 1e-8. Composition and public point transformations validate their
results. Reflections are allowed. Inverse point mapping subtracts translation
before solving the linear system, avoiding unnecessary cancellation at the origin.
Containment includes boundaries with a 1e-7 logical-pixel tolerance; rectangular
clips whose intersection has no area still admit no hits.

Paths admit at most 4,096 commands each. Every contour starts with Move and
contains a line or curve before another Move or Close. Closed contours require
another Move before further drawing. Open paths are valid for strokes; fills
require every contour explicitly closed. The pinned GPUI builder implements
quadratic curves through `curve_to(endpoint, control)` and cubic curves through
`cubic_bezier_to`. Encoded command bounds alone do not bound tessellation work;
the native adapter still needs expanded-output admission limits.

The initial vocabulary includes rectangles, ellipses, filled/stroked paths,
native-shaped single-line text and managed images. Paths contain move, line,
quadratic/cubic curve and close commands. Draw order is scene item order, with the
last item on top; hit testing visits that order in reverse. Items may reference
shared path/text/image resources. Paint colors resolve OCaml theme tokens before
publication. Shaders, custom blend modes, arbitrary native paint callbacks and
unbounded user tessellation are outside this vocabulary.

Per-item transforms map local geometry into world space. Nested clip rectangles
are explicitly in world coordinates and intersect; an empty intersection draws
and hits nothing. Viewport clipping also applies. This avoids claiming that an
axis-aligned GPUI content mask implements a rotated or arbitrary-path clip. Native
text items currently admit translation and positive uniform scale only; image
items admit translation and positive independent axis scales. The pinned
`ShapedLine::paint`/`paint_image` APIs do not provide arbitrary affine transforms
for those draws. Rotation, shear and reflection of text/images are rejected,
never silently approximated. Paths and shape geometry retain affine support.

Interactive items declare labels and explicit local hit regions (rectangle,
ellipse or polygon). Transform inversion applies before containment tests, and
world clips apply before selecting the topmost hit. Drawing and hit-region bounds
are separate: a thin line can deliberately have a larger usable hit target.
Polygons admit 3..256 points, require non-collinear geometry, and use even-odd
containment independently of winding. Self-intersections are permitted under
that rule; this does not imply arbitrary-path clipping support.

## Mounted configuration contract

`Gpuio.Canvas` now defines the pure mounted-view configuration and semantic event
vocabulary. This is the interface/codec stage: mounting, command execution and
native interaction are still being implemented and are not advertised as working
capabilities. `Gpuio_eio.Canvas` continues to own registration independently.

A viewport stores the world point at the top-left and positive zoom; local pixels
are `(world - origin) * zoom`, before GPUI device scaling. Zoom is within
0.05..64 and configurable ordered limits contain the initial viewport. Native
pan/zoom preserves that mapping on resize. Initial viewport applies on mount or
scene-generation reset; explicit commands can select/deselect an item, set/reset
the viewport or clear native position overrides. Positive monotone command
sequences prevent replay across ordinary scene publications/resets. A new mounted
node or source starts a fresh command history.

Selection is single-item and stable by item ID. Native drag preview reports one
completed `Moved` observation with the resulting local-to-world transform.
Overrides survive same-generation publications while that item's source transform
is unchanged. Updating its source transform accepts/replaces the override;
removal, scene reset or `Reset_positions` clears it. Cancelling a gesture restores
its previous completed position, and does not commit an interrupted preview.
Dragging changes translation only, preserving the admitted linear transform.

Observations include selection, activation, completed movement, viewport change,
command completion and typed failure. Each identifies the scene revision and
generation; only a failure before acquiring a scene may use zero for both.
Missing/foreign application ownership encodes an unavailable source instead of an
unchecked native ID. Labels and theme-resolved selection color are validated.
The native configuration decoder limits its standalone envelope to 2 KiB and the
label to 1 KiB, validates nested values and rejects trailing or truncated data.

## Native interaction and accessibility

The component has one primary focus entry. Keyboard navigation selects labeled
interactive items; accessible item nodes expose equivalent selection/activation
and movement actions. Decorative marks require a meaningful scene-level text
description. Visible geometry alone is not an accessibility representation.

Native policies provide selection, bounded object dragging and pan/zoom, with
explicit limits and keyboard alternatives. Report completed semantic changes and
bounded/coalesced viewport observations, rather than sending each paint frame to
OCaml. Escape, reconfiguration, removal, hiding, disabling, modal exclusion,
window deactivation and unmount cancel in-progress manipulation. Cancellation
must happen before a hidden instance can resume; sink rejection alone is not
sufficient to reset a retained gesture. Extend the SDK lifecycle hook if needed.

## Proposed admission and validation

Start with a 4 MiB encoded scene limit, 20,000 items, 4,096 drawing resources,
65,536 total path commands and 1 MiB aggregate text. Individual messages remain
within the bridge envelope. Bound concurrent staging, total live scene bytes,
tessellation vertices, decoded/shaped caches and accessibility nodes separately;
serialized byte bounds do not bound those expanded resources by themselves.

The version-1 immutable scene schema now enforces those wire limits, a maximum
of eight world clip rectangles per item and 2,048 interactive items per scene.
The 1 MiB text budget includes scene description, native text, font-family names
and interactive labels. Text resources contain at most 16 KiB of single-line
UTF-8, a bounded font-family name, size 4..256 logical pixels and weight 100..900.
Resources have positive 64-bit IDs/generations; item IDs are positive and distinct
within the snapshot. Path, text and image references require matching resource
kind and exact generation. Images reference an existing native asset handle;
application/liveness checks occur in the implemented adapter and native registry
publication. Cross-publication resource generation history is enforced there.

The decoder checks nested and aggregate counts before allocating lists/strings,
checks the complete 4 MiB envelope, validates UTF-8/finiteness and requires exact
consumption. Domain validation follows decoding. Reused paths cache their
control-point hull and closure status during admission so 20,000 references do
not trigger 20,000 path traversals. Domain validation is distinct from native
tessellation and font/image allocation budgets, which remain required.

Validation covers finite geometry, nonsingular transforms, positive dimensions,
valid path topology, valid UTF-8, unique identities, exact resource generations,
clip depth, labels/actions and total admission budgets. Rejected updates preserve
the prior published scene and revision. Scene/node/window generations fence late
observations. The implementation must measure large-scene update, hit and render
workloads, retained memory after repeated disposal, and work while interaction and
streaming are active. Record macOS native evidence separately from Linux build and
later graphical acceptance.
