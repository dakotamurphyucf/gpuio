open Core

type t =
  { months : int64
  ; cell_height : float
  ; cell_gap : float
  ; month_gap : float
  ; padding : float
  ; cell_radius : float
  ; outline_width : float
  ; selected_background : int64 option
  ; selected_foreground : int64 option
  ; hover_background : int64 option
  ; today_border : int64 option
  ; focus_border : int64 option
  ; muted_foreground : int64 option
  }
[@@deriving bin_io, equal, sexp_of]

let default =
  { months = 1L
  ; cell_height = 32.0
  ; cell_gap = 4.0
  ; month_gap = 16.0
  ; padding = 8.0
  ; cell_radius = 6.0
  ; outline_width = 1.0
  ; selected_background = None
  ; selected_foreground = None
  ; hover_background = None
  ; today_border = None
  ; focus_border = None
  ; muted_foreground = None
  }
;;

let valid t =
  let bounded value low high =
    Float.is_finite value && Float.(value >= low && value <= high)
  in
  Int64.(t.months >= 1L && t.months <= 12L)
  && bounded t.cell_height 16. 128.
  && List.for_all [ t.cell_gap; t.month_gap; t.padding; t.cell_radius ] ~f:(fun n ->
    bounded n 0. 64.)
  && bounded t.outline_width 0. (t.cell_height /. 2.)
  && List.for_all
       [ t.selected_background
       ; t.selected_foreground
       ; t.hover_background
       ; t.today_border
       ; t.focus_border
       ; t.muted_foreground
       ]
       ~f:(Option.for_all ~f:(fun n -> Int64.(n >= 0L && n <= 0xffffffffL)))
;;
