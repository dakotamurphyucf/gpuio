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

module Samples = struct
  module Item = struct
    type t =
      { batch : int
      ; number : int
      }
    [@@deriving equal, sexp_of]

    let number t = t.number
    let key t = sprintf "layered-sample-%d-%d" t.batch t.number
  end

  type t =
    { batch : int
    ; items : Item.t list
    }

  module Action = struct
    type t =
      | Show
      | Dismiss of Item.t
  end

  let create batch =
    { batch; items = List.map [ 1; 2; 3 ] ~f:(fun number -> { Item.batch; number }) }
  ;;

  let initial = create 0
  let items t = t.items

  let apply t = function
    | Action.Show -> if t.batch = Int.max_value then t else create (t.batch + 1)
    | Dismiss item ->
      { t with
        items = List.filter t.items ~f:(fun current -> not (Item.equal current item))
      }
  ;;
end
