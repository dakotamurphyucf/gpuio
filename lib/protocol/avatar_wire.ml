open Core

module Config = struct
  type t =
    { source : Image_wire.Source.t option
    ; fit : Image_wire.Fit.t
    ; label : string option
    ; fallback : string
    }
  [@@deriving bin_io, equal, sexp_of]
end
