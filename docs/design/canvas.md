# Retained canvas

Status: OCH-24 implementation design in progress. The bounds below are proposed
admission limits to validate with the implementation and workload evidence, not
claims of completed functionality.

The current implementation provides validated `Canvas_geometry` and
`Canvas_path` OCaml values and matching Rust protocol geometry. Scene resource
registration, native rendering and canvas interaction are still in progress;
these pure modules do not yet expose a rendered canvas widget.

## Ownership and updates

OCaml constructs immutable scene snapshots with stable item IDs, reusable drawing
resources and explicit resource generations. An Eio-scoped scene registration
owns the native resource; views borrow its application-bound generational handle.
Registering or updating a large scene uses bounded, revision-checked uploads and
atomic publication, following the existing asset/document ownership model. A
partial upload never becomes visible. Releasing registration prevents new binds;
existing views own leases until they unmount. Old snapshots remain charged while
retained, and cancellation releases staging reservations.

The canvas itself is a built-in native component using the extension SDK's
instance lifecycle and event delivery. Its small properties reference a registered
scene instead of embedding the entire scene in the SDK's 64 KiB property envelope.
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
text/image transform support must be checked against the pinned renderer before
the public interface is finalized; unsupported combinations must be rejected,
never silently approximated.

Interactive items declare labels and explicit local hit regions (rectangle,
ellipse or polygon). Transform inversion applies before containment tests, and
world clips apply before selecting the topmost hit. Drawing and hit-region bounds
are separate: a thin line can deliberately have a larger usable hit target.
Polygons admit 3..256 points, require non-collinear geometry, and use even-odd
containment independently of winding. Self-intersections are permitted under
that rule; this does not imply arbitrary-path clipping support.

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

Validation covers finite geometry, nonsingular transforms, positive dimensions,
valid path topology, valid UTF-8, unique identities, exact resource generations,
clip depth, labels/actions and total admission budgets. Rejected updates preserve
the prior published scene and revision. Scene/node/window generations fence late
observations. The implementation must measure large-scene update, hit and render
workloads, retained memory after repeated disposal, and work while interaction and
streaming are active. Record macOS native evidence separately from Linux build and
later graphical acceptance.
