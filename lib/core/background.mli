open Core

type t [@@deriving equal, sexp_of]

module Color_space : sig
  type t =
    | Srgb
    | Oklab
  [@@deriving equal, sexp_of]
end

val solid : Color.t -> t

(** Native diagonal slash pattern with transparent gaps. Width and interval are
    finite physical pixel dimensions in [0.5,64], passed verbatim to GPUI's
    packed pattern brush. They do not rotate with chart orientation or scale
    with logical layout. The native brush quantizes dimensions; this is not a
    precise vector hatch. Theme colors resolve before rendering. *)
val pattern_slash : Color.t -> width:float -> interval:float -> t Or_error.t

(** Native checkerboard with alternating colored/transparent squares, size in
    physical pixels [0.5,64]. The pattern starts at the painted shape's bounds. *)
val checkerboard : Color.t -> size:float -> t Or_error.t

(** GPUI's two-stop sRGB linear gradient. Angle is finite, in 0..360 clockwise
    degrees from the top;
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
    | Pattern_slash of Color.t * float * float
    | Checkerboard of Color.t * float

  val describe : t -> description
end
