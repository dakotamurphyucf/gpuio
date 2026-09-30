open Core

type t =
  { value : int
  ; step : int
  ; generation : int64
  ; sequence : int64
  ; command : (int64 * int) option
  ; disabled : bool
  ; visible : bool
  ; notice : string
  }

module Action = struct
  type t =
    | Observe of int64 * int Gpuio.Extension.Event.t
    | Set_property
    | Send_command
    | Toggle_step
    | Toggle_disabled
    | Toggle_visible
    | Reset
    | Depart
end

let initial =
  { value = 7
  ; step = 1
  ; generation = 1L
  ; sequence = 0L
  ; command = None
  ; disabled = false
  ; visible = true
  ; notice = "Waiting for the native component"
  }
;;

let apply t = function
  | Action.Observe (generation, _) when not (Int64.equal generation t.generation) -> t
  | Observe (_, Data value) ->
    if
      t.disabled
      || (not t.visible)
      || Option.is_some t.command
      || value < 0
      || value > 100
    then t
    else { t with value; notice = "Native input received" }
  | Observe (_, Mounted) -> { t with notice = "Native component mounted" }
  | Observe (_, Command_completed sequence) ->
    (match t.command with
     | Some (pending, value) when Int64.equal sequence pending ->
       { t with
         value
       ; command = None
       ; notice = sprintf "Native command %Ld completed" sequence
       }
     | None | Some _ -> t)
  | Observe (_, Failed error) ->
    { t with
      command = None
    ; notice =
        "Native failure: "
        ^ Sexp.to_string_hum ([%sexp_of: Gpuio.Extension.Error.t] error)
    }
  | Set_property ->
    if Option.is_some t.command
    then t
    else { t with value = 12; notice = "Property set to 12" }
  | Send_command ->
    if Option.is_some t.command || Int64.equal t.sequence Int64.max_value
    then t
    else (
      let sequence = Int64.succ t.sequence in
      { t with
        sequence
      ; command = Some (sequence, 42)
      ; notice = "Waiting for command acknowledgement"
      })
  | Toggle_step ->
    if Option.is_some t.command
    then t
    else { t with step = (if t.step = 1 then 5 else 1) }
  | Toggle_disabled -> { t with disabled = not t.disabled }
  | Toggle_visible -> { t with visible = not t.visible }
  | Reset ->
    if Int64.equal t.generation Int64.max_value
    then t
    else
      { initial with
        generation = Int64.succ t.generation
      ; disabled = t.disabled
      ; visible = t.visible
      ; step = t.step
      }
  | Depart ->
    { t with
      generation =
        (if Int64.equal t.generation Int64.max_value
         then t.generation
         else Int64.succ t.generation)
    ; command = None
    ; notice = "Waiting for the native component"
    }
;;

let value t = t.value
let step t = t.step
let generation t = t.generation
let command t = t.command
let disabled t = t.disabled
let visible t = t.visible
let notice t = t.notice
