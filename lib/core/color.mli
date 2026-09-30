open Core

(** Colors are concrete RGBA values or named theme references. Theme resolution
    happens before submission; missing names produce a recoverable error. *)
type t [@@deriving equal, sexp_of]

val rgba : red:int -> green:int -> blue:int -> alpha:int -> t Or_error.t
val rgb_exn : int -> t
val token : string -> t Or_error.t
val token_exn : string -> t

(** Multiply alpha by a finite factor in [0,1]. Repeated factors are combined
    without growing an expression tree. Tokens resolve against the submission
    theme; alpha is rounded to the nearest 8-bit value only during resolution.
    A transparent token still requires a definition. *)
val with_opacity : t -> float -> t Or_error.t

module Expert : sig
  type value =
    | Rgba of int64
    | Token of string
    | Opacity of t * float

  val value : t -> value
end
