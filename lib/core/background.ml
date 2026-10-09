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
  | Pattern_slash of Color.t * float * float
  | Checkerboard of Color.t * float
[@@deriving equal, sexp_of]

let solid color = Solid color
let pattern_dimension n = Float.is_finite n && Float.(n >= 0.5 && n <= 64.)

let pattern_slash color ~width ~interval =
  if pattern_dimension width && pattern_dimension interval
  then Ok (Pattern_slash (color, width, interval))
  else Or_error.error_string "pattern width and interval must be finite in [0.5,64]"
;;

let checkerboard color ~size =
  if pattern_dimension size
  then Ok (Checkerboard (color, size))
  else Or_error.error_string "checkerboard size must be finite in [0.5,64]"
;;

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
    | Pattern_slash of Color.t * float * float
    | Checkerboard of Color.t * float

  let describe t = t
end
