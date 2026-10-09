open Core

type t =
  { revision : int64
  ; x : float
  ; y : float
  ; width : float
  ; height : float
  }
[@@deriving bin_io, equal, sexp_of]

let coordinate n = Float.is_finite n && Float.(n >= -1e9 && n <= 1e9)
let dimension n = Float.is_finite n && Float.(n >= 0. && n <= 1e9)

let is_valid t =
  Int64.(t.revision >= 0L)
  && coordinate t.x
  && coordinate t.y
  && dimension t.width
  && dimension t.height
  && Float.(t.height > 0.)
;;
