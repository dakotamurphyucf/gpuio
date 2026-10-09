open Core
module W = Gpuio_protocol.Rating_wire

module Request = struct
  include W.Request

  let validate request =
    if valid request
    then Ok request
    else Or_error.error_string "rating request is outside its bounded star range"
  ;;

  let set value = validate (Set value)
  let toggle value = validate (Toggle value)
  let clear = Set 0
  let increase = Increase
  let decrease = Decrease
end

module Config = struct
  type t = W.Config.t [@@deriving equal, sexp_of]

  let create
        ~label
        ~value
        ?(maximum = 5)
        ?(star_size = 24.)
        ?(disabled = false)
        ?(read_only = false)
        ()
    =
    let t = { W.Config.label; value; maximum; star_size; disabled; read_only } in
    if W.Config.valid t
    then Ok t
    else
      Or_error.error_string
        "rating requires a bounded label, value, maximum and star size"
  ;;

  let value t = t.W.Config.value
  let maximum t = t.W.Config.maximum
  let star_size t = t.W.Config.star_size
  let is_disabled t = t.W.Config.disabled
  let is_read_only t = t.W.Config.read_only

  let apply_request t request =
    if not (W.Config.can_apply t request)
    then t
    else (
      let value =
        match request with
        | Request.Set value -> value
        | Toggle value -> if t.value = value then 0 else value
        | Increase -> Int.min t.maximum (t.value + 1)
        | Decrease -> Int.max 0 (t.value - 1)
      in
      { t with value })
  ;;
end

module Appearance = struct
  type t =
    { active : Color.t option
    ; inactive : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create ?active ?inactive () = { active; inactive }
  let default = create ()
end

module Expert = struct
  let to_wire t = t
  let request_of_wire request = Option.some_if (W.Request.valid request) request
  let can_apply = W.Config.can_apply

  let appearance_to_wire (t : Appearance.t) ~theme =
    let open Or_error.Let_syntax in
    let resolve = function
      | None -> return None
      | Some color ->
        let%map color = Theme.resolve theme color in
        Some color
    in
    let%bind active = resolve t.active in
    let%map inactive = resolve t.inactive in
    if Option.is_none active && Option.is_none inactive
    then None
    else Some { W.Appearance.active; inactive }
  ;;
end
