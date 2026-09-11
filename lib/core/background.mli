open Core

type t [@@deriving equal, sexp_of]

val solid : Color.t -> t

(** GPUI's two-stop linear gradient. Angle is clockwise degrees from the top;
    stops use fractions in 0..1 and must be ordered. *)
val linear_gradient
  :  angle:float
  -> from:Color.t * float
  -> to_:Color.t * float
  -> t Or_error.t

module Expert : sig
  type description =
    | Solid of Color.t
    | Linear_gradient of float * (Color.t * float) * (Color.t * float)

  val describe : t -> description
end
