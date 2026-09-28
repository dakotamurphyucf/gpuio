open Core

module Stage = struct
  type t =
    | Idle
    | Working
    | Halfway
    | Complete
  [@@deriving equal, sexp_of]

  let label = function
    | Idle -> "Ready to begin"
    | Working -> "Finding the right pieces"
    | Halfway -> "Halfway there"
    | Complete -> "Everything is in place"
  ;;

  let next = function
    | Idle -> Working
    | Working -> Halfway
    | Halfway -> Complete
    | Complete -> Idle
  ;;
end

type t =
  { stage : Stage.t
  ; enabled : bool
  ; notification : int option
  ; serial : int
  }

module Action = struct
  type t =
    | Advance
    | Toggle_enabled
    | Notify
    | Dismiss of int
    | Leave
end

let initial = { stage = Stage.Idle; enabled = true; notification = None; serial = 0 }
let stage t = t.stage
let is_enabled t = t.enabled
let notification t = t.notification

let apply t = function
  | Action.Advance -> if t.enabled then { t with stage = Stage.next t.stage } else t
  | Toggle_enabled -> { t with enabled = not t.enabled }
  | Notify ->
    (* Never reuse an identity, even if the finite preview counter is exhausted. *)
    if t.serial = Int.max_value
    then t
    else { t with serial = t.serial + 1; notification = Some (t.serial + 1) }
  | Dismiss serial ->
    if Option.equal Int.equal t.notification (Some serial)
    then { t with notification = None }
    else t
  | Leave -> { t with notification = None }
;;
