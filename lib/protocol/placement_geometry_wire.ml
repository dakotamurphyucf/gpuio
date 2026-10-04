open Core

module Corner = struct
  type t =
    | Top_left
    | Top_right
    | Bottom_left
    | Bottom_right
  [@@deriving bin_io, equal, sexp_of]
end

module Point = struct
  type t =
    { corner : Corner.t
    ; x : float
    ; y : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { viewport_margin : float
  ; point : Point.t option
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  let bounded x low high = Float.is_finite x && Float.(x >= low && x <= high) in
  bounded t.viewport_margin 0. 16384.
  && Option.for_all t.point ~f:(fun p ->
    bounded p.x (-1_000_000.) 1_000_000. && bounded p.y (-1_000_000.) 1_000_000.)
;;
