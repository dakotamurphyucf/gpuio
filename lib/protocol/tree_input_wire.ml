open Core

module Navigation = struct
  type t =
    | Previous
    | Next
    | First
    | Last
    | Parent
    | Child
  [@@deriving bin_io, equal, sexp_of]
end

module Selection = struct
  type t =
    | Replace
    | Toggle
    | Range of { extend : bool }
  [@@deriving bin_io, equal, sexp_of]
end

module Request = struct
  type t =
    | Navigate of Navigation.t * Selection.t option
    | Select of int64 * Selection.t
    | Focus of int64
    | Set_expanded of int64 * bool
    | Activate of int64
    | Select_active of Selection.t
    | Activate_active
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Navigate _ | Select_active _ | Activate_active -> true
    | Select (id, _) | Focus id | Set_expanded (id, _) | Activate id -> Int64.(id > 0L)
  ;;
end
