open Core

module Unavailability = struct
  type t =
    | Backend
    | Mapping
    | Native_view
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  | Inactive of Input_content_hint_wire.t option
  | Exposed of Input_content_hint_wire.t
  | Unavailable of Input_content_hint_wire.t * Unavailability.t
[@@deriving bin_io, equal, sexp_of]
