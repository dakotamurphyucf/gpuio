open Core

(** Resolved chart styling. The palette follows dataset series/slice/node order
    and repeats when needed. Call [create] again when the application theme
    changes; construction resolves all color tokens without native callbacks. *)
type t [@@deriving equal, sexp_of]

(** Palette length 1..32; stroke width 0.5..8, point radius 1..12 and bar corner
    radius 0..32 logical pixels. Area opacity is 0..1. Optional [gradient_end]
    makes bars fade from their palette color to that color along the value axis.
    Corner radii clamp to each bar's size. Other filled families use palette
    colors; area/radar fills additionally apply [area_opacity]. Native labels
    inherit the view's font and use [label_color]. *)
val create
  :  ?palette:Color.t list
  -> ?axis_color:Color.t
  -> ?grid_color:Color.t
  -> ?label_color:Color.t
  -> ?selection_color:Color.t
  -> ?gradient_end:Color.t
  -> ?stroke_width:float
  -> ?point_radius:float
  -> ?bar_radius:float
  -> ?area_opacity:float
  -> ?theme:Theme.t
  -> unit
  -> t Or_error.t

val default : t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Chart_style_wire.t
  val of_wire : Gpuio_protocol.Chart_style_wire.t -> t Or_error.t
end
