open Core

type t =
  { clear_label : string option
  ; loading : bool
  ; gap : float
  }
[@@deriving bin_io, equal, sexp_of]
