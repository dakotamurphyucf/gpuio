open Core

type t =
  { top : float
  ; right : float
  ; bottom : float
  ; left : float
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  List.for_all [ t.top; t.right; t.bottom; t.left ] ~f:(fun n ->
    Float.is_finite n && Float.(n >= 0. && n <= 16384.))
;;
