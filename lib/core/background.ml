open Core

type t =
  | Solid of Color.t
  | Linear_gradient of float * (Color.t * float) * (Color.t * float)
[@@deriving equal, sexp_of]

let solid color = Solid color

let linear_gradient ~angle ~from:((_, start) as from) ~to_:((_, stop) as to_) =
  if
    Float.is_finite angle
    && Float.(angle >= 0. && angle <= 360. && start >= 0. && stop <= 1. && start <= stop)
  then Ok (Linear_gradient (angle, from, to_))
  else Or_error.error_string "invalid linear gradient angle or stops"
;;

module Expert = struct
  type description = t =
    | Solid of Color.t
    | Linear_gradient of float * (Color.t * float) * (Color.t * float)

  let describe t = t
end
