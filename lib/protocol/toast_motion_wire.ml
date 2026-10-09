open Core

type t =
  { spring : Animation_wire.Spring.t
  ; enter_ms : int64
  ; exit_ms : int64
  ; offset : float
  }
[@@deriving bin_io, equal, sexp_of]
