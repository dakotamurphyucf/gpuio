open Core

module Viewport : sig
  type t [@@deriving equal, sexp_of]

  (** [origin] is the world point at the viewport's top-left. A world point maps
      to local logical pixels as [(point - origin) * zoom]. Device scale is
      applied by GPUI afterward. Zoom must be finite and in [0.05,64]. *)
  val create : origin:Canvas_geometry.Point.t -> zoom:float -> t Or_error.t

  val default : t
  val origin : t -> Canvas_geometry.Point.t
  val zoom : t -> float
end

module Command : sig
  module Action : sig
    type t =
      | Select of Canvas_scene.Item_id.t option
      | Set_viewport of Viewport.t
      | Reset_viewport
      | Reset_positions
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  (** Positive, monotonically increasing sequence per mounted node/source.
      Repeating the same command never replays it; changing an existing sequence
      is invalid. Scene publication/reset does not replay commands. Selecting a
      missing/noninteractive item fails natively. Explicit viewport commands are
      allowed when pointer pan/zoom is disabled, within configured zoom limits. *)
  val create : sequence:int64 -> Action.t -> t Or_error.t
end

module Error : sig
  type t =
    | Wrong_application
    | Unavailable_scene
    | Render_limit
    | Unavailable_image
    | Invalid_command
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Observation : sig
  type t =
    | Selection_changed of Canvas_scene.Item_id.t option
    | Activated of Canvas_scene.Item_id.t
    | Moved of Canvas_scene.Item_id.t * Canvas_geometry.Transform.t
    | Viewport_changed of Viewport.t
    | Command_completed of int64
    | Failed of Error.t
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** Native observations identify the published scene, not the view-tree
      revision. [Moved] reports the resulting local-to-world transform after
      native manipulation, rather than device-pixel deltas. Paint and intermediate
      drag frames do not synchronously call OCaml. Failure before acquiring a
      scene uses revision/generation zero; other observations require both >0. *)
  type t =
    { scene_revision : int64
    ; scene_generation : int64
    ; observation : Observation.t
    }
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Borrows an application-owned registration. The containing view's style
      determines size. Initial viewport applies on mount/source-generation reset;
      ordinary configuration updates preserve native viewport/selection.

      All input policies default to true; disabled defaults to false. Dragging
      additionally requires the item's [draggable] policy. Native position
      overrides persist across same-generation updates with unchanged item
      transforms; changing the source transform, removing the item, resetting the
      scene or [Reset_positions] clears the corresponding override. OCaml can
      accept [Moved]'s transform in a new scene without double-applying movement.

      Zoom limits are finite, ordered and within [0.05,64], and contain the initial
      viewport. Selection color resolves against [theme] during construction. *)
  val create
    :  scene:Canvas_scene.Handle.t
    -> ?label:string
    -> ?initial_viewport:Viewport.t
    -> ?minimum_zoom:float
    -> ?maximum_zoom:float
    -> ?selectable:bool
    -> ?draggable:bool
    -> ?pan_zoom:bool
    -> ?disabled:bool
    -> ?selection_color:Color.t
    -> ?theme:Theme.t
    -> ?command:Command.t
    -> unit
    -> t Or_error.t

  val scene : t -> Canvas_scene.Handle.t
end

module Expert : sig
  (** Preserve application ownership until view reconciliation. A foreign or
      missing owner emits an unavailable source, never an unchecked native ID. *)
  val to_wire
    :  Config.t
    -> owner:Canvas_scene.Expert.Owner.t option
    -> Gpuio_protocol.Canvas_view_wire.Config.t

  val event
    :  scene_revision:int64
    -> scene_generation:int64
    -> Gpuio_protocol.Canvas_view_wire.Observation.t
    -> Event.t Or_error.t
end
