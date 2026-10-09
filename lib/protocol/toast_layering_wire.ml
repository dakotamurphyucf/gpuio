open Core

type t =
  { peek : float
  ; gap : float
  ; width_step : float
  ; visible : int64
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  List.for_all [ t.peek; t.gap ] ~f:(fun n ->
    Float.is_finite n && Float.(n >= 0. && n <= 16384.))
  && Float.is_finite t.width_step
  && Float.(t.width_step >= 0. && t.width_step <= 0.1)
  && Int64.(t.visible >= 1L && t.visible <= 8L)
;;
