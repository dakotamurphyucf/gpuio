open Core

type t =
  { palette : int64 list
  ; axis_color : int64
  ; grid_color : int64
  ; label_color : int64
  ; selection_color : int64
  ; gradient_end : int64 option
  ; stroke_width : float
  ; point_radius : float
  ; bar_radius : float
  ; area_opacity : float
  }
[@@deriving bin_io, equal, sexp_of]

let color value = Int64.(value >= 0L && value <= 0xffff_ffffL)

let within value minimum maximum =
  Float.is_finite value && Float.(value >= minimum && value <= maximum)
;;

let valid t =
  List.length t.palette >= 1
  && List.length t.palette <= 32
  && List.for_all t.palette ~f:color
  && List.for_all
       [ t.axis_color; t.grid_color; t.label_color; t.selection_color ]
       ~f:color
  && Option.for_all t.gradient_end ~f:color
  && within t.stroke_width 0.5 8.
  && within t.point_radius 1. 12.
  && within t.bar_radius 0. 32.
  && within t.area_opacity 0. 1.
;;
