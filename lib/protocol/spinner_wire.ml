open Core

(** Private configuration bytes; field order is shared with Rust. The atomic
    node operation and capability are added only with the native adapter. *)
module Config = struct
  type t =
    { label : string
    ; animated : bool
    ; period_ms : int
    ; easing : Animation_wire.Easing.t
    ; source : Image_wire.Source.t option
    }
  [@@deriving bin_io, equal, sexp_of]
end
