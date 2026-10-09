open Core

type t =
  { expanded : bool
  ; retain : bool
  ; spring : Animation_wire.Spring.t
  }
[@@deriving bin_io, equal, sexp_of]
