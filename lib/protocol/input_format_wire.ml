open Core

module Number = struct
  type t =
    { separator : string option
    ; fraction_digits : int64 option
    }
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  | Pattern of string
  | Number of Number.t
[@@deriving bin_io, equal, sexp_of]
