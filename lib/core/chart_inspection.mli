open Core

(** Native chart inspection presentation. All dimensions are logical pixels.
    Source values, IDs and selection remain unchanged; no OCaml callback runs
    during hover, layout or paint. Optional colors inherit chart defaults and
    are resolved, including tokens, by [Chart_style.create]. *)
module Placement : sig
  type t =
    | Corner
    | Anchor
    | Cursor
  [@@deriving equal, sexp_of]
end

module Axis : sig
  type t =
    | Off
    | Vertical
    | Horizontal
    | Both
  [@@deriving equal, sexp_of]
end

module Pattern : sig
  type t =
    | Dashed
    | Solid
  [@@deriving equal, sexp_of]
end

module Card : sig
  type t [@@deriving equal, sexp_of]

  (** Width 96..480, gap 0..64, padding/radius 0..24, font size 8..32,
      line height 8..48, border width 0..8. Line height must be at least font
      size. Corner keeps the top-right card; Anchor places it beside the
      inspected mark with bounded flipping/clipping. Cursor follows native pointer
      motion, falling back to Anchor for keyboard inspection or after pointer
      departure/cancellation. Marker and crosshair remain on the inspected mark.
      Hiding the card does not
      hide the original-data control. Title/value flags hide visual children;
      a visible card retains its full accessible summary. *)
  val create
    :  ?visible:bool
    -> ?title:bool
    -> ?values:bool
    -> ?placement:Placement.t
    -> ?width:float
    -> ?gap:float
    -> ?padding:float
    -> ?radius:float
    -> ?font_size:float
    -> ?line_height:float
    -> ?border_width:float
    -> ?text_color:Color.t
    -> ?background:Color.t
    -> ?border_color:Color.t
    -> unit
    -> t Or_error.t

  val default : t
end

module Crosshair : sig
  type t [@@deriving equal, sexp_of]

  (** Thickness 0.5..64. Full prepared plot extents only. Solid thickness
      supports highlight bands; dashed uses native dashed borders. Endpoint
      guides stay inside the plot; narrow plots clip the configured thickness. *)
  val create
    :  ?axis:Axis.t
    -> ?pattern:Pattern.t
    -> ?thickness:float
    -> ?color:Color.t
    -> unit
    -> t Or_error.t

  val default : t
end

module Marker : sig
  type t [@@deriving equal, sexp_of]

  (** Size 2..48, stroke width 0..8 and at most half the size. Status retains
      the selection check/preview circle independently of fill and stroke. *)
  val create
    :  ?visible:bool
    -> ?status:bool
    -> ?size:float
    -> ?stroke_width:float
    -> ?fill:Color.t
    -> ?stroke:Color.t
    -> unit
    -> t Or_error.t

  val default : t
end

type t [@@deriving equal, sexp_of]

val create : ?card:Card.t -> ?crosshair:Crosshair.t -> ?marker:Marker.t -> unit -> t
val default : t

module Expert : sig
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Chart_inspection_wire.t Or_error.t
end
