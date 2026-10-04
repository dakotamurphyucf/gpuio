# Flat resizable panel groups — OCH-41

Status: Core/Bonsai view, native bridge and gallery implemented; local integration
checks pass ([evidence](../evidence/split-group-bridge-och41.md)). Physical and release
qualification remains open. The existing two-pane `Split_pane` remains supported. Neither
TestPlatform results nor this contract establish physical desktop release acceptance.

The pinned base resizable group exposes horizontal/vertical arbitrary panels,
per-panel size ranges and initial sizes, native dragging, programmatic resize,
insertion/removal and a native handle-paint callback. Its state is indexed;
the callback receives axis/active state, not panel identity. See the three
`base-resizable-*` catalog snapshots at the recorded pinned revision.
Simply sending a reordered OCaml child vector would associate old sizes with the
wrong panels. The adapter must preserve IDs and each panel's complete bounds.

## Public interface and ownership

Add `Split_group` alongside `Split_pane`; keep the two-pane API supported.
Public shape:

```ocaml
module Id : sig
  type t [@@deriving equal, compare, sexp_of]
  val of_string : string -> t Or_error.t
  val to_string : t -> string
end
module Panel : sig
  type t [@@deriving equal, sexp_of]
  val create : Id.t -> label:string -> ?initial_size:float
    -> ?minimum_size:float -> ?maximum_size:float -> ?visible:bool
    -> unit -> t Or_error.t
end
module Resize_request : sig
  type t [@@deriving equal, sexp_of]
  val create : Id.t -> size:float -> serial:int64 -> t Or_error.t
end
module Config : sig
  type t [@@deriving equal, sexp_of]
  val create : label:string -> ?axis:Split_pane.Axis.t
    -> ?keyboard_step:float -> ?reset_generation:int64
    -> ?resize:Resize_request.t -> Panel.t list -> t Or_error.t
end
val View.split_group : ?key:Key.t -> ?style:Style.t
  -> ?appearance:Split_group.Appearance.t
  -> ?handles:(Split_group.Id.t * 'action View.t) list
  -> ?on_resize:(Split_group.Snapshot.t -> 'action)
  -> config:Split_group.Config.t
  -> panels:(Split_group.Id.t * 'action View.t) list
  -> unit -> 'action View.t Or_error.t
```

IDs are nonempty UTF-8 without NUL, at most 256 bytes. Panel labels are nonblank,
at most 1024 bytes; the group label is nonblank and at most 4096 bytes. Groups
contain 0–64 unique panels; an empty or single-visible-panel group has no handle.
Sizes are finite logical pixels with `0 <= minimum <= maximum <= 16384`.
The default range is 80–16384. An explicit initial size must lie inside its range;
omission shares space left by explicit preferences (equally among unseeded panes). Zero-width ranges are legal.
Keyboard step is positive and at most 16384. Reset generations are nonnegative
and must not decrease for an existing owner. Ordinary updates preserve native
sizes; mount, new IDs, axis changes and a newer reset generation seed geometry.
Changing a surviving panel's initial size alone does not override user resizing.
New ranges are enforced even without a reset. Reordering retains keyed child
widgets, size preferences and surviving handle focus identities.

OCaml owns panel identity/order/visibility, constraints and content. Rust owns
measured sizes and transient gestures. A completed pointer, keyboard or
accessibility resize produces one keyed snapshot; pointer movement stays native.
Programmatic requests carry a positive monotonically increasing serial and a
finite target size in 0–16384. They execute once against usable geometry and emit
a final snapshot tagged with their serial. A missing panel cancels the request;
a hidden panel waits until visible. Clearing the request cancels pending work
without resetting the last accepted serial. Resetting geometry cannot replay an
old request. No synchronous callback crosses the bridge from layout or paint.

Snapshots contain every current panel ID and retained size in current order,
including hidden panes, plus their source (`Pointer`, `Keyboard`,
`Accessibility`, or `Request serial`). Event admission/OCaml dispatch must fence
owner/handler generations, current reset generation and structural identity;
stale callbacks must not apply a snapshot from a previous panel collection.

## Geometry and constraints

The mounted tree uses one structural container per panel, keyed by panel ID.
Each contains two fixed structural slots: exactly one ordinary content root and
zero or one passive grip root. Hidden panels and terminal grips stay mounted.
Configuration order determines panel order; a reorder moves existing wrappers.
The bridge operation carries configuration and resolved appearance together.
Both native admission and Core construction validate slot shape and passive grips;
native admission also rechecks descendants after nonstructural updates.
Resize observations carry node/handler generations, accepted revision and reset
generation. Any configuration change replaces the handler generation; appearance
alone does not. Native callbacks additionally check the current configuration
before enqueueing so old measured frames cannot publish against new panel state.

Use a bounded native geometry owner for this adapter, with explicit ID mapping.
The existing two-pane base adapter need not change. Reuse GPUI elements, focus,
accessibility and pointer capture; do not introduce an OCaml layout loop or a
second application state owner. First-party pure geometry is necessary to make
redistribution and full min/max guarantees explicit and testable, rather than
relying on index-based native state that the OCaml side cannot safely reorder.

Distribute usable container extent among visible panes in proportion to their
retained sizes (equal shares before preferences exist), clamping to every range
and redistributing residual space among eligible panes. An inserted or unhidden
panel participates with its seed/retained preference; hidden sizes are preserved.
If total minima exceed available extent, preserve minima and clip overflow.
If total maxima are smaller than available extent, preserve maxima and leave
trailing space. Do not silently violate constraints to fill an impossible parent.
Zero/unavailable layout does not destroy retained preferences.

A handle belongs to the preceding visible panel ID; there is no handle after the
last visible panel. Moving a boundary transfers the same amount between the
left and right partitions, nearest panels first, bounded by both partitions'
remaining capacity. This permits a flat group to continue resizing when an
adjacent pane reaches a limit. A programmatic panel resize transfers space with
siblings to its right first, then to its left, without exceeding any range.
Changing container geometry or structural/range/visibility/reset policy during
a drag cancels the gesture before applying the new layout. Pointer cancellation,
Escape, lost capture, deactivation, hidden/disabled ancestors, removal and window
close restore the committed geometry; they do not emit a completed resize.

Measure and lay out panel content and handles coherently in the same native
frame. Avoid a prepaint update that leaves the displayed divider waiting for a
later user event. No persistent frame clock is needed for a static group.

## Handles, styles and accessibility

`Appearance.create` supplies a checked default `handle_style` and ID-keyed
`item_styles`. States refine in base/hover/focused/pressed/disabled order, with
the per-ID style overriding shared paint within each state. The 1–16 pixel
painted `thickness` (default 1) is separate from the 8–32 pixel `hit_extent`
(default 8, at least thickness). Color, opacity, borders, radii, shadows and text
presentation are supported; layout, visibility, cursor and input-policy changes
are rejected. At most 64 unique overrides and 256 total declarations are allowed.
Absent IDs are ignored so a presentation can survive filtering. Styles apply to
the divider paint or grip wrapper; they cannot alter the native input rectangle.
Optional keyed `handles` supplies
passive decorative content (for example a grip or SVG); it never owns drag,
keyboard or accessibility behavior. New native elements are built per paint;
OCaml functions are not used as native paint callbacks. Decorations for hidden
or last panels stay mounted but unpainted; resources release on removal/close.

Each actual boundary has one focusable Splitter with an accessible name derived
from its adjacent panel labels, perpendicular orientation, current boundary
position and feasible range. Arrows move along the group axis; Home/End move to
the feasible extrema; accessibility increment/decrement/SetValue use the same
solver. Focus recording must distinguish multiple handles under one group node.
Native captured dragging works outside the handle/window area while keeping
child editor/input events independent. Reorder, visibility and removal must not
leave focus or capture on a retired boundary.

## Required verification before completion

Independent OCaml/Rust payload vectors and malformed/bounded admission; no-op,
reset and stale-event handling; numerical/reference/property tests for feasible
and infeasible bounds, zero sizes, multiple panes, insertion/reorder/hide,
container resize and all movement directions; production native layout, real
native event dispatch for drag/cancel/focus/keyboard/AX, per-handle styles and
passive decorations, child identity, idle and resource retirement. Add a public
multi-panel gallery example with reordering, hide/show, limits, programmatic
resize and custom handles, plus installed-consumer coverage. Physical macOS
GPU/input/VoiceOver and required Linux automation remain OCH-17 release gates;
TestPlatform checks alone do not satisfy them.


## Current foundation

The Core data model, standalone paired records/decoders, pure geometry and retained
state are implemented and locally validated; see [foundation evidence](../evidence/split-group-foundation-och41.md).
They are connected through `View.split_group`, paired Kind56/Op104/Event73 and
the production Host. The state model returns an observation when a programmatic
request is applied to a usable measurement. The native adapter queues it after
paint with current owner/handler/reset/collection checks. Drag previews do not overwrite
committed preferences; cancellation restores them before a changed layout applies.

The pinned GPUI `container_query` element supplies same-frame measured rendering:
its native callback runs after the assigned outer size is known, then lays out and
prepaints fresh child elements as roots inside that rectangle. Use this existing
primitive for panel/handle allocation instead of introducing a second-frame
layout loop or a new upstream hook. Ordinary View children are still described
before that native callback; no application callback runs there.

The standalone native widget now uses that primitive and the checked appearance
record. Same-frame layout, captured dragging, cancellation, keyed focus, keyboard/
accessibility actions, decorative grips and retirement have TestPlatform coverage;
see [native widget evidence](../evidence/split-group-widget-och41.md). The parent
must supply current visibility/enabled/pointer policy, begin/finish-frame sweeping,
explicit close, focus registration and an asynchronously fenced observer. The
production Host supplies these hooks, structural hidden-panel policy and measured
clipping for child focus/AX. The Navigation gallery's **A workspace that adapts to
you** card includes a retained editor, reorder/hide/show, outline insertion/removal,
both axes, limits, serialled resize, reset and custom grips. Broader integration,
installed-consumer and physical acceptance evidence remains distinct.
