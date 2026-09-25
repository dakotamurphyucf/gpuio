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
explicit reset and release. Native retained-tree rendering is implemented;
mounted pointer/keyboard/accessibility integration and public widget acceptance
remain in progress.

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

## Native geometry preparation

`canvas_mesh`, `canvas_plan` and `canvas_jobs` implement bounded geometry and work
queues. `canvas_host` schedules them on GPUI background workers, and `canvas_paint`
paints admitted meshes. `canvas_content` paints native text and managed images.
The typed `View.canvas` and Bonsai alias reconcile small configurations through
the tree protocol. Native mounted painting is connected to that retained-tree
kind; pointer/keyboard/accessibility and the public OCaml example remain in progress.

### View and observation bridge

`View.canvas ?key ?style ?on_event config` borrows the scene handle retained in
`Canvas.Config.t`. The reconciler checks its application owner before encoding;
a foreign owner produces an absent source for a typed native failure, never an
unchecked resource ID. Configurations travel in `Set_canvas` (operation tag 36),
with `Canvas_view` kind tag 31. A canvas is a leaf with no text/children. Native
tree admission allows at most 128 canvas nodes per window, accounts configuration
retention, and rejects multiple canvas configurations for one node within a
transaction so intermediate commands cannot silently disappear. Transaction
failure preserves the previous tree and quota counters.

`Canvas_event` (event tag 40) includes the window, node, handler, tree revision,
optional source identity, displayed scene revision/generation and typed
observation. Only a pre-acquisition failure may use the zero scene pair; an
absent source also requires such a failure. Config changes rotate the callback
handler while preserving node identity; callback-only refresh retains the handler
and uses the latest closure. Replaced/unmounted handlers, foreign sources,
malformed observations and future tree revisions are rejected.

The Eio adapter additionally requires a live registration and the desired reset
epoch. It accepts the currently acknowledged scene or the exact publication
actually in flight, since an event can precede that publication's acknowledgement.
Queued uploads cannot generate accepted events. Ordinary updates keep the prior
acknowledged revision eligible until publication succeeds; reset intent immediately
fences the old epoch. After acknowledgement, observations of older displayed
revisions are ignored, even while native geometry preparation catches up.
Failed ordinary publication preserves eligibility of the earlier accepted scene.
Release and shutdown suppress subsequent scene observations; mounted native
leases still follow the separate resource-retention contract above. This bridge
does not yet advertise a rendered-canvas capability.

### Mounted native presentation

`canvas_view` acquires each scene lease when the native tree accepts its binding,
before a subsequent registration release can arrive. The mounted native state
retains selection, viewport, position overrides and the command watermark;
prepared geometry, shaping/image caches and worker handles are separate,
disposable presentation resources. Hiding or evicting a view cancels pending
preparation and discards presentation resources, while preserving that native
state. A hide/show transition without an intervening paint still performs this
cleanup. Source replacement and unmount explicitly close the old state and drop
its lease, including when an older paint closure still references that state.

Preparation uses the current viewport and actual window device scale. The last
prepared scene remains visible while a replacement is pending or fails admission.
An accepted ready result advances native interaction state and content caches
together. A scene-generation reset requests geometry using the initial viewport,
resets the viewport when installed, and does not replay an earlier command.
Source publication cancels an unfinished gesture immediately and invalidates
containing virtual-list rows. Rendering uses one shared budget reset by the root
window's first paint callback each frame, rather than allocating an allowance per
canvas or relying on render-call frequency.

Deferred text shaping requests another frame; image/geometry completions wake
their native service. An unchanged failed request is not retried by an idle timer.
Preparation failure identifies the requested publication, since that publication
has not become displayed state; otherwise the Eio revision fence would discard
the error when the old frame remains visible. Failures are reported once per
handler/preparation identity. Existing mounts can still paint their acquired
lease after registration release; a new mount receives `Unavailable_scene`.

Geometry is tessellated in local coordinates before applying the item's affine
transform. This preserves stroke width semantics under nonuniform scale, shear
and reflection. Fills use even-odd; strokes use butt caps and miter joins with
miter limit four. Rectangles, ellipses and path curves share the same mesh output.
A local f64 anchor avoids losing subpixel detail solely because an object is
translated near the coordinate-domain edge. Tessellation uses the same locked
Lyon 1.0.19 package already used by GPUI, now an explicit native dependency.

Lazy f64 curve flattening is capped at 16,384 input events/segments per mesh;
ellipse chord-error admission computes its required segment count before
allocation. A custom Lyon output builder checks 65,536 vertices and 196,608
triangle indices before growth, checks cancellation and finite geometry, and
never exposes a partial mesh. Working allocations within Lyon are distinct from
the retained output quota; input/output and worker counts bound separate parts of
the workload, not a process RSS or wall-clock guarantee.

A scene plan caches at most 4,096 unique meshes and admits at most 1,048,576
expanded draw vertices across all referenced meshes. Shared geometry therefore
cannot bypass frame-work admission. Rectangles/ellipses normalize their local
origin so translated equal-size shapes share geometry. Path keys include resource
identity/generation; stroke width and curve accuracy are part of mesh identity.
Accuracy uses a conservative affine stretch bound and a downward-rounded
power-of-two tolerance derived from zoom and actual device scale. Zoom remains
0.05..64; admitted device scale is 0.25..16. This controls flattening accuracy,
not a guarantee of exact floating-point rasterization.

All plans in one job pool share a 128 MiB retained-mesh accounting quota, separate
from the native scene registry and OCaml adapter quotas. Charges include vector
capacities, plan metadata and conservative cache/Arc allowances; externally held
mesh readers keep their charge until their final drop. A mesh under construction
and bounded preparation indexes are transient workspace, not silently included
in the retained charge. Text shaping, decoded image and GPUI frame/cache budgets
remain separate integration requirements.

The job pool admits 128 mounted-view handles, two running workers and only one
latest desired request per handle. Superseded work is cancelled and obsolete
completions are discarded. Snapshot/quality identity suppresses redundant jobs;
publication, zoom or device-scale changes can request preparation, while ordinary
paint/pan/translation does not require rebuilding the same geometry. Work is
Send and can run entirely off the UI thread, with no OCaml callback. Closing the
pool prevents new work and cancels pending/running work; retained external readers
remain charged until released.

The application host uses a bounded completion channel and explicit worker-exit
fences. Accepted completions refresh their window; stale completions release their
results without requesting a frame. There is no idle polling. Both asynchronous
shutdown and the synchronous application quit path cancel and drain workers;
workers never wait on a UI callback. Handles verify their originating application
and tracked window before admitting an update.

Mesh painting adds the normalized shape origin before the item transform, then
applies viewport translation/zoom. World clips stay fixed during object movement;
GPUI applies device scale and inherited clipping/opacity. Culling precedes triangle
expansion. A shared `FrameBudget` admits at most 1,048,576 expanded vertices per
window frame, checking before allocation. The mounted view integration must share
that budget across canvases rather than allocate a separate allowance per canvas.
The helper accepts meshes only; text and images require their own painting paths.

## Native text and images

`canvas_content` retains one admitted scene snapshot and indexes its resources.
Text uses GPUI's native single-line shaper. The origin is the top-left of a line
box with height ascent plus descent. Its logical font size is the resource size
times the item's positive uniform scale and viewport zoom; GPUI applies actual
device density once. The `system` family maps to GPUI's platform UI font. Shaped
lines are cached by canonical resource key, logical font size and color, so pan,
dragging and ordinary redraws reuse them.

Each mounted content cache retains at most 512 shaped text variants. A separate
bounded cache keeps up to 20,000 measured ink bounds, keyed without color, so
offscreen variants can be culled without retaining full lines or reshaping every
frame. A cold offscreen size is measured once under the same shaping allowance.
Offscreen variants do not consume the visible-variant allowance. All canvas caches
in an application share a 32 MiB text accounting budget, including measurements,
text, run/glyph vector capacities and bookkeeping allowances. Unused least-recently-used
variants can be evicted before admission. Output is limited to 32,768 glyphs per
line and a 512-device-pixel font size. Native shaping's temporary allocations and
GPUI/platform font and glyph caches are separate; this is not a process RSS or
hard shaping-time guarantee. A single shaping input is already limited to 16 KiB
by the resource contract. Invalid native metrics become a typed failure.

The shared window `FrameBudget` admits at most 32 new shaping calls and 64 KiB of
new text per frame. Additional misses return `Deferred`; the owner requests a
later frame. Painting admits at most 65,536 glyphs and 16,777,216 estimated glyph
pixels, using conservative font bounding-box areas at actual density. This is
work accounting rather than exact atlas-byte measurement. Clipping/culling uses
the native shaped ink bounds before charging visible-line drawing work. Cache or
draw admission failures return `Render_limit`; the owner must report the failure
and stop deferred retries for that failed render.

Images stretch their first frame into the local destination rectangle, under
the admitted positive axis scale/translation. Canvas images are static; automatic
animated-image playback is not part of this canvas vocabulary. Visible SVGs use
exact displayed physical dimensions, rounded up, with fill fitting. Known-size
SVG requests enter the existing image scheduler directly, without an unused
intrinsic decode. Raster pixels and SVG size variants share the existing bounded
decoder/atlas accounting and worker-exit cleanup. Sources come from the scene's
existing image leases, so retirement prevents new acquisition but does not break
an existing scene's painting or SVG resampling.

Content caches admit 256 image variants, while the shared frame budget admits
eight new image requests and 1,024 image draws. Image work is requested only after
world/viewport/inherited clipping and destination culling. `Loading` waits for
native image completion to wake the window; it does not introduce polling.
Frame-end drops unused image variants, including obsolete SVG sizes. Publication
prunes removed/replaced resources; scene-generation reset clears both caches.
Hiding/disposal must clear mounted caches. A mounted source change recreates the
content owner, just as it recreates interaction state. These painting helpers are
validated natively; their public widget/lifecycle integration is still required.

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
the native mesh/plan admission above bounds expanded geometry. The mounted host
must also apply the admitted plan and separately bound its frame/font/image caches.

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

`canvas_state` implements the native interaction model independently of GPUI
event dispatch. It retains an admitted snapshot, an index of at most 2,048
interactive items and at most one completed transform override per interactive
item. One gesture holds either an object preview or a pan preview. It has no
event queue and emits no observations for intermediate pointer movement.

Hit testing follows reverse scene order, using the effective native transform
and unchanged world clips. Pointer dragging starts after three logical pixels of
movement. Keyboard/accessibility movement can use the same translation operation.
Cached source drawing/hit hulls constrain movement to the admitted coordinate
domain while preserving the affine linear coefficients. Selection navigation
follows scene order (first/last/next/previous), stopping at the endpoints;
activation and movement respect item policies.

New publications cancel unfinished gestures. Same-generation publications retain
completed overrides only while their source transform and interaction remain and
the new geometry still admits the override. Accepting a completed transform in
OCaml replaces its native override without applying it twice. Generation resets
clear selection/overrides and restore the configured initial viewport. Ordinary
configuration changes preserve viewport state, clamping zoom if limits narrow.

Explicit commands work even when user input is disabled. Each new sequence is
consumed once, including failed commands; callers retry with a larger sequence.
Older sequences are ignored, and changing the latest sequence's action reports
one invalid-command observation. Scene resets do not reset this high watermark.
Zoom preserves the world point under its logical-pixel anchor unless the world
origin must clamp to the coordinate-domain edge. A completed pan emits a viewport
observation; cancelling a preview restores the prior viewport. The mounted host
must coalesce wheel/zoom observations and stamp them with the displayed snapshot's
revision/generation.

Input starts disabled until the host enables it. The host must immediately
disable/cancel it for hiding, modal exclusion and window deactivation, even when
no render occurs between disable and enable. Changing input policy or replacing
the scene also cancels previews. Real pointer/keyboard dispatch, focus handling,
AX nodes and this lifecycle integration are still required; pure state tests and
direct state calls in the GPU test do not establish native input acceptance.

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
Geometry, worker and native content/frame bounds are implemented as described
above. Their integration into the public mounted widget remains in progress.

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
tessellation and native font/image/frame admission described above.

Validation covers finite geometry, nonsingular transforms, positive dimensions,
valid path topology, valid UTF-8, unique identities, exact resource generations,
clip depth, labels/actions and total admission budgets. Rejected updates preserve
the prior published scene and revision. Scene/node/window generations fence late
observations. The implementation must measure large-scene update, hit and render
workloads, retained memory after repeated disposal, and work while interaction and
streaming are active. Record macOS native evidence separately from Linux build and
later graphical acceptance.


## Mounted native input

The mounted canvas participates in native focus traversal. Left click selects;
a double click activates an activatable item. Left drag previews an item's local
to world transform in Rust and reports its resulting transform on release.
Middle drag pans. Wheel samples pan; Control/platform-modified wheel samples zoom
around the pointer. Wheel viewport observations occupy one pending slot per canvas
and coalesce until the next actual root paint. A keyboard/pointer action flushes
an earlier wheel observation first; configuration changes and disposal discard
pending delivery. No new timer or per-sample frame callback is allocated.

With canvas focus, arrows select previous/next interactive items in scene order;
Home/End select first/last. Enter/Space activate the selected item. Shift+arrows
move it by one logical pixel, Alt+Shift+arrows by ten. Alt+arrows pan by twenty
logical pixels; +/- zoom around the canvas center. Escape rolls back an unfinished
drag. Tab cancels any gesture and uses the existing application focus traversal.
The native scene state enforces selectable, draggable and pan/zoom policies.

Capture survives repaint by rebinding to the current canvas hitbox. Focus loss,
window deactivation, hiding, modal exclusion, disabled input, configuration changes,
scene publication, changed bounds, source replacement and unmount cancel gestures.
Cancellation rolls back preview transforms and releases only this canvas's capture.
Old frame callbacks are fenced by configuration identity and the current scene
lease; a frame retained while replacement geometry prepares cannot interact as
though it represented the newer publication. Disabled canvases leave tab traversal;
explicit application commands can still update them.

Selection paints one transformed outline using the configured color, clipped to
the canvas and the selected item's world clip stack. Rectangle/ellipse hit bounds
or a polygon's bounding rectangle determine the outline. This fixed-size decorative
stroke is additional to the scene mesh vertex budget; it never uploads a changed
scene or requests an OCaml paint callback. Root focus semantics are present;
per-object accessible semantics remain required before full canvas acceptance.
