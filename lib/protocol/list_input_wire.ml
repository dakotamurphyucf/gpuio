open Core

module Navigation = struct
  type t =
    | Previous
    | Next
    | First
    | Last
  [@@deriving bin_io, equal, sexp_of]
end

module Gesture = struct
  type t =
    | Replace
    | Toggle
    | Range of { extend : bool }
  [@@deriving bin_io, equal, sexp_of]
end

module Confirmation = struct
  type t =
    | Primary
    | Secondary
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { generation : int64
    ; cursor : int64 option
    ; query : Node_id.t option
    ; selection_on_navigation : bool
    ; disabled : bool
    ; busy : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.generation > 0L) && Option.for_all t.cursor ~f:(fun id -> Int64.(id > 0L))
  ;;
end

module Request = struct
  type t =
    | Navigate of Navigation.t * Gesture.t option
    | Select of int64 * Gesture.t
    | Focus of int64
    | Select_active of Gesture.t
    | Confirm of int64 * Confirmation.t
    | Confirm_active of Confirmation.t
    | Context of int64
    | Context_active
    | Set_selected of int64 * bool
    | Cancel
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Navigate _ | Select_active _ | Confirm_active _ | Context_active | Cancel -> true
    | Select (id, _) | Focus id | Confirm (id, _) | Context id | Set_selected (id, _) ->
      Int64.(id > 0L)
  ;;
end
