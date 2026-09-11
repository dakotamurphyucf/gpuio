open Core

type t [@@deriving equal, sexp_of]

(** Names are unique, and definitions must be concrete colors. This avoids cycles
    and makes theme resolution deterministic. *)
val create : (string * Color.t) list -> t Or_error.t

val default : t
val resolve : t -> Color.t -> int64 Or_error.t
