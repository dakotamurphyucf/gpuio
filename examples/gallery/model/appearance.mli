open Core

type t =
  | Light
  | Dark
[@@deriving equal, sexp_of]

val toggle : t -> t
val label : t -> string

module Scale : sig
  type t =
    | Compact
    | Comfortable
    | Large
  [@@deriving equal, sexp_of]

  val all : t list
  val next : t -> t
  val label : t -> string
  val factor : t -> float
end
