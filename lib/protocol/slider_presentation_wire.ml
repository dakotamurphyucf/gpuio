open Core

module Fill = struct
  type t =
    | Selected
    | Remaining
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { fill : Fill.t
  ; track_thickness : float
  ; track_radius : float
  ; thumb_size : float
  ; target_size : float
  ; ring_width : float
  ; track_color : int64 option
  ; fill_color : int64 option
  ; thumb_color : int64 option
  ; ring_color : int64 option
  }
[@@deriving bin_io, equal, sexp_of]

let default =
  { fill = Selected
  ; track_thickness = 4.
  ; track_radius = 2.
  ; thumb_size = 12.
  ; target_size = 20.
  ; ring_width = 2.
  ; track_color = None
  ; fill_color = None
  ; thumb_color = None
  ; ring_color = None
  }
;;

let valid t =
  let dimension n = Float.is_finite n && Float.(n >= 1. && n <= 256.) in
  List.for_all [ t.track_thickness; t.thumb_size; t.target_size ] ~f:dimension
  && Float.(t.track_thickness <= t.target_size && t.thumb_size <= t.target_size)
  && Float.is_finite t.track_radius
  && Float.(t.track_radius >= 0. && t.track_radius <= 256.)
  && Float.is_finite t.ring_width
  && Float.(t.ring_width >= 0. && t.ring_width <= t.target_size /. 2.)
  && List.for_all
       [ t.track_color; t.fill_color; t.thumb_color; t.ring_color ]
       ~f:(Option.for_all ~f:(fun n -> Int64.(n >= 0L && n <= 0xffffffffL)))
;;
