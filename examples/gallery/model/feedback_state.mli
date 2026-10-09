open Core

(** Preview actions are reduced against the latest state. Only one notification
    is retained; a delayed dismissal cannot dismiss its replacement. *)
type t

module Stage : sig
  type t =
    | Idle
    | Working
    | Halfway
    | Complete
  [@@deriving equal, sexp_of]

  val label : t -> string
end

module Action : sig
  type t =
    | Advance
    | Toggle_enabled
    | Notify
    | Dismiss of int
    | Leave
end

val initial : t
val stage : t -> Stage.t
val is_enabled : t -> bool
val notification : t -> int option
val apply : t -> Action.t -> t

(** Persistent demonstration cards. Restoring starts a fresh batch even while
    an earlier card is exiting; callbacks carry the batch identity. *)
module Samples : sig
  type t

  module Item : sig
    type t [@@deriving sexp_of]

    val number : t -> int
    val key : t -> string
  end

  module Action : sig
    type t =
      | Show
      | Dismiss of Item.t
  end

  val initial : t
  val items : t -> Item.t list
  val apply : t -> Action.t -> t
end
