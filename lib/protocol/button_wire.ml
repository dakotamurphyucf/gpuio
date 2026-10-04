open Core

module Focus = struct
  type t =
    | Focusable of Checkable_wire.Tab_order.t
    | Preserve
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Focusable order -> Checkable_wire.Tab_order.valid order
    | Preserve -> true
  ;;
end

module Policy = struct
  type t =
    { loading : bool
    ; focus : Focus.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Focus.valid t.focus
end

module Content = struct
  type t =
    | Icon_slots
    | Rich
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { policy : Policy.t
    ; content : Content.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Policy.valid t.policy
end
