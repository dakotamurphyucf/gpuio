open Core
module Wire = Gpuio_protocol.Notification_wire

module Tag = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string value =
    if Wire.valid_tag value
    then Ok value
    else Or_error.error_string "invalid notification tag"
  ;;

  let to_string t = t
end

module Action_id = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string value =
    if Wire.valid_action_id value
    then Ok value
    else Or_error.error_string "invalid notification action ID"
  ;;

  let to_string t = t
end

module Action = struct
  type t = Wire.Action.t [@@deriving equal, sexp_of]

  let create id ~label =
    let t = { Wire.Action.id; label } in
    if Wire.Action.valid t
    then Ok t
    else Or_error.error_string "invalid notification action label"
  ;;

  let id t = t.Wire.Action.id
  let label t = t.Wire.Action.label
end

module Sound = Wire.Sound

type t = Wire.Content.t [@@deriving equal, sexp_of]

let create ~title ?(body = "") ?(actions = []) ?(sound = Sound.Silent) () =
  let t = { Wire.Content.title; body; actions; sound } in
  if Wire.Content.valid t
  then Ok t
  else Or_error.error_string "invalid notification content"
;;

let title t = t.Wire.Content.title
let body t = t.Wire.Content.body
let actions t = t.Wire.Content.actions
let sound t = t.Wire.Content.sound

module Receipt = struct
  type t = Wire.Receipt.t [@@deriving equal, compare, sexp_of]

  let tag t = t.Wire.Receipt.tag
end

module Authorization = Wire.Authorization
module Capabilities = Wire.Capabilities
module Error = Wire.Error
module Closed_reason = Wire.Closed_reason
module Event = Wire.Event

module Expert = struct
  let error_of_wire t = t
  let authorization_of_wire t = t
  let capabilities_of_wire t = t
  let to_wire t = t

  let of_wire t =
    if Wire.Content.valid t
    then Ok t
    else Or_error.error_string "invalid notification wire content"
  ;;

  let receipt_to_wire t = t

  let receipt_of_wire t =
    if Wire.Receipt.valid t
    then Ok t
    else Or_error.error_string "invalid notification receipt"
  ;;

  let event_of_wire t =
    if Wire.Event.valid t
    then Ok t
    else Or_error.error_string "invalid notification event"
  ;;
end
