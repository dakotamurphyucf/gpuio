open Core

(** Declarative native axis presentation, resolved by [Chart_style.create].
    Existing [Chart_options.Axes.x/y] remain the master visibility gates.
    Dimensions are logical pixels; presentation never changes source data. *)
module Label_side : sig
  type t =
    | Auto
    | Before
    | After
  [@@deriving equal, sexp_of]
end

module Label_align : sig
  type t =
    | Auto
    | Left
    | Center
    | Right
  [@@deriving equal, sexp_of]
end

module Tick_position : sig
  type t [@@deriving equal, sexp_of]

  (** Finite numeric coordinate in [-1e100,1e100]. Out-of-domain values and
      values on categorical x-axes are ignored. *)
  val value : float -> t Or_error.t

  (** Stable category center. Unknown IDs and numeric/y targets are ignored. *)
  val category : Chart_data.Category_id.t -> t

  (** [0,1] along the physical axis: left-to-right or top-to-bottom, independent
      of numeric reversal. *)
  val fraction : float -> t Or_error.t
end

module Tick : sig
  type t [@@deriving equal, sexp_of]

  (** Text is at most 256 UTF-8 bytes without ASCII controls. Empty text hides
      its caption but retains its grid position. Font size is [8,32]; omitted
      color/font and Auto alignment inherit the axis settings. *)
  val create
    :  position:Tick_position.t
    -> text:string
    -> ?color:Color.t
    -> ?font_size:float
    -> ?align:Label_align.t
    -> unit
    -> t Or_error.t
end

type t [@@deriving equal, sexp_of]

(** Omitted position keeps the bottom/left axis; explicit [0,1] is perpendicular
    to the line (top-to-bottom for horizontal axes, left-to-right for vertical).
    Before labels lie above/left; After below/right; Auto chooses the nearer edge.
    Ticks default to automatic; an explicit list (at most 64) replaces them,
    including an empty list. [tick_count] is [2,64], used only for automatic ticks.
    Label gap [0,64], width [8,256], font size [8,32], line width [0.5,8].
    Defaults preserve existing layout. Labels may ellipsize/clip/overlap in dense
    or tiny plots; this is not a collision-avoidance API. *)
val create
  :  ?line:bool
  -> ?labels:bool
  -> ?position:float
  -> ?ticks:Tick.t list
  -> ?tick_count:int
  -> ?label_side:Label_side.t
  -> ?label_align:Label_align.t
  -> ?label_gap:float
  -> ?label_width:float
  -> ?font_size:float
  -> ?line_width:float
  -> ?line_color:Color.t
  -> ?label_color:Color.t
  -> unit
  -> t Or_error.t

val default : t

module Expert : sig
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Chart_axis_wire.t Or_error.t
  val position_to_wire : Tick_position.t -> Gpuio_protocol.Chart_axis_wire.Tick_position.t
end
