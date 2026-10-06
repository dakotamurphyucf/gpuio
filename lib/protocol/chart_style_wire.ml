open Core

module Key = struct
  type t =
    | Series of int64
    | Slice of int64
    | Node of int64
    | Rising
    | Falling
  [@@deriving bin_io, equal, compare, sexp_of]

  let valid = function
    | Series id | Slice id | Node id -> Int64.(id > 0L)
    | Rising | Falling -> true
  ;;
end

module Ordinal = struct
  type t =
    { domain : Key.t list
    ; range : int64 list
    ; unknown : int64 option
    }
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { version : int64
  ; palette : int64 list
  ; axis_color : int64
  ; grid_color : int64
  ; label_color : int64
  ; selection_color : int64
  ; gradient_end : int64 option
  ; stroke_width : float
  ; point_radius : float
  ; bar_radius : float
  ; area_opacity : float
  ; ordinal : Ordinal.t option
  ; inspection : Chart_inspection_wire.t
  ; node_labels : Chart_node_labels_wire.t
  }
[@@deriving bin_io, equal, sexp_of]

let color value = Int64.(value >= 0L && value <= 0xffff_ffffL)

let within value minimum maximum =
  Float.is_finite value && Float.(value >= minimum && value <= maximum)
;;

let valid_ordinal (t : Ordinal.t) =
  List.length t.domain <= 1024
  && List.for_all t.domain ~f:Key.valid
  && (not (List.contains_dup t.domain ~compare:Key.compare))
  && List.length t.range >= 1
  && List.length t.range <= 32
  && List.for_all t.range ~f:color
  && Option.for_all t.unknown ~f:color
;;

let valid t =
  Int64.equal t.version (-2L)
  && List.length t.palette >= 1
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
  && Option.for_all t.ordinal ~f:valid_ordinal
  && Chart_inspection_wire.valid t.inspection
  && Chart_node_labels_wire.valid t.node_labels
;;
