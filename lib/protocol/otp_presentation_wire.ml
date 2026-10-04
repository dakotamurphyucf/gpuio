open Core

type t =
  { groups : int
  ; cell_width : float option
  ; cell_gap : float
  ; group_gap : float
  ; radius : float
  ; border_width : float
  ; background : int64 option
  ; border : int64 option
  ; focus_border : int64 option
  ; selection : int64 option
  ; caret : int64 option
  }
[@@deriving bin_io, equal, sexp_of]

let default =
  { groups = 1
  ; cell_width = None
  ; cell_gap = 5.
  ; group_gap = 20.
  ; radius = 6.
  ; border_width = 1.
  ; background = None
  ; border = None
  ; focus_border = None
  ; selection = None
  ; caret = None
  }
;;

let valid t =
  let dimension n = Float.is_finite n && Float.(n >= 0. && n <= 4096.) in
  let color = Option.for_all ~f:(fun n -> Int64.(n >= 0L && n <= 0xffffffffL)) in
  t.groups >= 1
  && t.groups <= 32
  && Option.for_all t.cell_width ~f:(fun n -> dimension n && Float.(n >= 1.))
  && List.for_all [ t.cell_gap; t.group_gap; t.radius ] ~f:dimension
  && Float.is_finite t.border_width
  && Float.(t.border_width >= 0. && t.border_width <= 64.)
  && List.for_all
       [ t.background; t.border; t.focus_border; t.selection; t.caret ]
       ~f:color
;;
