open Core

(** Ordered list intents. Row IDs are monotonic managed-list identities, never
    application keys or node handles. Config/requests are wire values; use the
    checked component API at the application boundary. *)
module Navigation : sig
  type t =
    | Previous
    | Next
    | First
    | Last
  [@@deriving bin_io, equal, sexp_of]
end

module Gesture : sig
  type t =
    | Replace
    | Toggle
    | Range of { extend : bool }
  [@@deriving bin_io, equal, sexp_of]
end

module Confirmation : sig
  type t =
    | Primary
    | Secondary
  [@@deriving bin_io, equal, sexp_of]
end

module Config : sig
  (** Positive interaction epoch. Query ownership, disabled state and navigation
      policy changes advance it; cursor/busy updates may retain it so queued
      relative navigation survives a normal render. [query] is an Input sibling,
      with at most one owning list. Native admission validates relationships.
      Removing/reinstalling input must advance the last admitted generation. *)
  type t =
    { generation : int64
    ; cursor : int64 option
    ; query : Node_id.t option
    ; selection_on_navigation : bool
    ; disabled : bool
    ; busy : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  val valid : t -> bool
end

module Request : sig
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

  val valid : t -> bool
end
