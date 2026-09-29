open Core

type t [@@deriving equal, sexp_of]

module Color_space : sig
  type t =
    | Srgb
    | Oklab
  [@@deriving equal, sexp_of]
end

val solid : Color.t -> t

(** GPUI's two-stop sRGB linear gradient. Angle is clockwise degrees from the top;
    stops use fractions in 0..1 and must be ordered. *)
val linear_gradient
  :  angle:float
  -> from:Color.t * float
  -> to_:Color.t * float
  -> t Or_error.t

(** Select the interpolation space. Colors are still supplied as ordinary RGBA
    values or theme tokens; interpolation happens natively during paint. [Srgb]
    is equivalent to [linear_gradient]. *)
val linear_gradient_in
  :  Color_space.t
  -> angle:float
  -> from:Color.t * float
  -> to_:Color.t * float
  -> t Or_error.t

module Expert : sig
  type description =
    | Solid of Color.t
    | Linear_gradient of Color_space.t * float * (Color.t * float) * (Color.t * float)

  val describe : t -> description
end
