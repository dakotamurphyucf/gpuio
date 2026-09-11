open Core

(** Stable identity scoped to siblings. Changing a key or the node kind replaces
    its native identity. Unkeyed children use their position among siblings. *)
type t [@@deriving compare, equal, sexp_of]

val of_string : string -> t Or_error.t
val of_string_exn : string -> t
val of_int : int -> t
val to_string : t -> string
