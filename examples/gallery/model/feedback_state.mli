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
