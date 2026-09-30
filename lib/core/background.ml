open Core

module Color_space = struct
  type t =
    | Srgb
    | Oklab
  [@@deriving equal, sexp_of]
end

type t =
  | Solid of Color.t
  | Linear_gradient of Color_space.t * float * (Color.t * float) * (Color.t * float)
[@@deriving equal, sexp_of]

let solid color = Solid color

let linear_gradient_in space ~angle ~from:((_, start) as from) ~to_:((_, stop) as to_) =
  if
    Float.is_finite angle
    && Float.(angle >= 0. && angle <= 360. && start >= 0. && stop <= 1. && start <= stop)
  then Ok (Linear_gradient (space, angle, from, to_))
  else Or_error.error_string "invalid linear gradient angle or stops"
;;

let linear_gradient = linear_gradient_in Color_space.Srgb

module Expert = struct
  type description = t =
    | Solid of Color.t
    | Linear_gradient of Color_space.t * float * (Color.t * float) * (Color.t * float)

  let describe t = t
end
