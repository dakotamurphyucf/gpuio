open Core

(** Inclusive integer percentages, bounded to 0..100 with lower <= upper. *)
type t [@@deriving equal, sexp_of]

val create : lower:int -> upper:int -> t Or_error.t
val all : t
val lower : t -> int
val upper : t -> int
val contains : t -> int -> bool
val describe : t -> string
