open Core
module W = Gpuio_protocol.Otp_wire
module Alphabet = W.Alphabet
module Input_error = W.Input_error

let max_input_bytes = W.max_input_bytes

module Policy = struct
  type t = W.Policy.t [@@deriving equal, sexp_of]

  let create ~length ?(alphabet = Alphabet.Digits) () =
    let t = { W.Policy.length; alphabet } in
    if W.Policy.valid t
    then Ok t
    else Or_error.error_string "OTP length must be between 1 and 32"
  ;;

  let length t = t.W.Policy.length
  let alphabet t = t.W.Policy.alphabet
end

module Value = struct
  type t = string [@@deriving equal, sexp_of]

  let empty = ""
  let to_string t = t
  let length = String.length
  let fits t ~policy = W.canonical policy t
  let is_complete t ~policy = fits t ~policy && String.length t = Policy.length policy
  let of_string policy text = W.normalize policy ~paste:false text
  let of_paste policy text = W.normalize policy ~paste:true text

  let edit t ~policy ~selection ~text ~paste =
    W.replace
      policy
      ~value:t
      ~anchor:(Text_input.Selection.anchor selection)
      ~head:(Text_input.Selection.head selection)
      ~paste
      text
    |> Result.map ~f:(fun (value, anchor, head) ->
      value, Text_input.Selection.create ~anchor ~head |> Or_error.ok_exn)
  ;;

  let replace t ~policy ~selection ~text = edit t ~policy ~selection ~text ~paste:false
  let paste t ~policy ~selection ~text = edit t ~policy ~selection ~text ~paste:true
end

module Appearance = struct
  type t =
    { geometry : Gpuio_protocol.Otp_presentation_wire.t
    ; background : Color.t option
    ; border : Color.t option
    ; focus_border : Color.t option
    ; selection : Color.t option
    ; caret : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create
        ?(groups = 1)
        ?cell_width
        ?(cell_gap = 5.)
        ?(group_gap = 20.)
        ?(radius = 6.)
        ?(border_width = 1.)
        ?background
        ?border
        ?focus_border
        ?selection
        ?caret
        ()
    =
    let geometry =
      { Gpuio_protocol.Otp_presentation_wire.default with
        groups
      ; cell_width
      ; cell_gap
      ; group_gap
      ; radius
      ; border_width
      }
    in
    if Gpuio_protocol.Otp_presentation_wire.valid geometry
    then Ok { geometry; background; border; focus_border; selection; caret }
    else Or_error.error_string "invalid OTP cell presentation"
  ;;

  let default = create () |> Or_error.ok_exn
end

module Config = struct
  type t = W.Config.t [@@deriving equal, sexp_of]

  let create
        ~policy
        ~label
        ?(masked = false)
        ?(disabled = false)
        ?(read_only = false)
        ?(auto_focus = false)
        ()
    =
    let t = { W.Config.policy; label; masked; disabled; read_only; auto_focus } in
    if W.Config.valid t then Ok t else Or_error.error_string "invalid OTP configuration"
  ;;

  let policy t = t.W.Config.policy
  let is_masked t = t.W.Config.masked
  let is_disabled t = t.W.Config.disabled
  let is_read_only t = t.W.Config.read_only
end

module Revision = struct
  type t = int64 [@@deriving compare, equal, sexp_of]

  let of_int64 t =
    if Int64.(t >= 0L) then Ok t else Or_error.error_string "negative OTP revision"
  ;;

  let to_int64 t = t
end

let selection_of_wire (s : W.Selection.t) =
  Text_input.Selection.create
    ~anchor:(Int64.to_int_exn s.anchor)
    ~head:(Int64.to_int_exn s.head)
  |> Or_error.ok_exn
;;

module Snapshot = struct
  type t =
    { window : Gpuio_protocol.Window_id.t
    ; node : Gpuio_protocol.Node_id.t
    ; data : W.Snapshot.t
    }
  [@@deriving equal, sexp_of]

  let revision t = t.data.revision
  let policy t = t.data.policy
  let value t = t.data.value
  let draft t = t.data.draft
  let selection t = selection_of_wire t.data.selection
  let composition t = Option.map t.data.composition ~f:selection_of_wire
  let focused t = t.data.focused
  let can_undo t = t.data.can_undo
  let can_redo t = t.data.can_redo
  let is_complete t = W.Snapshot.is_complete t.data
end

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Complete of Snapshot.t
    | Rejected of Input_error.t * Snapshot.t
  [@@deriving equal, sexp_of]
end

module Command = struct
  type t =
    | Replace of
        { value : Value.t
        ; selection : Text_input.Selection_policy.t
        ; undo : Text_input.Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Clear of
        { undo : Text_input.Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Select of Text_input.Selection.t
    | Focus
    | Undo
    | Redo
    | Cancel_composition
    | Read_snapshot
  [@@deriving equal, sexp_of]
end

module Command_error = W.Error

module Expert = struct
  let appearance_to_wire (t : Appearance.t) ~theme =
    let open Or_error.Let_syntax in
    let resolve = function
      | None -> return None
      | Some color -> Theme.resolve theme color |> Or_error.map ~f:Option.some
    in
    let%bind background = resolve t.background in
    let%bind border = resolve t.border in
    let%bind focus_border = resolve t.focus_border in
    let%bind selection = resolve t.selection in
    let%map caret = resolve t.caret in
    { t.geometry with background; border; focus_border; selection; caret }
  ;;

  let config_to_wire t = t
  let value_to_wire t = t
  let policy_to_wire t = t

  let snapshot_of_wire ~window ~node data =
    if W.Snapshot.valid data
    then Ok { Snapshot.window; node; data }
    else Or_error.error_string "invalid OTP snapshot"
  ;;

  let event_of_wire ~window ~node event =
    if not (W.Event.valid event)
    then Or_error.error_string "invalid OTP event"
    else (
      let%map.Or_error snapshot =
        snapshot_of_wire ~window ~node (W.Event.snapshot event)
      in
      match event with
      | Observed _ -> Event.Observed snapshot
      | Changed _ -> Changed snapshot
      | Complete _ -> Complete snapshot
      | Rejected (reason, _) -> Rejected (reason, snapshot))
  ;;

  let selection (s : Text_input.Selection.t) =
    { W.Selection.anchor = Int64.of_int (Text_input.Selection.anchor s)
    ; head = Int64.of_int (Text_input.Selection.head s)
    }
  ;;

  let selection_policy : Text_input.Selection_policy.t -> W.Selection_policy.t = function
    | Start -> Start
    | End -> End
    | Preserve -> Preserve
    | Select s -> Select (selection s)
  ;;

  let undo : Text_input.Undo_policy.t -> W.Undo_policy.t = function
    | Record -> Record
    | Reset -> Reset
  ;;

  let command_to_wire : Command.t -> W.Command.t = function
    | Replace { value; selection; undo = history; if_revision } ->
      Replace
        { value
        ; selection = selection_policy selection
        ; undo = undo history
        ; if_revision
        }
    | Clear { undo = history; if_revision } -> Clear { undo = undo history; if_revision }
    | Select s -> Select (selection s)
    | Focus -> Focus
    | Undo -> Undo
    | Redo -> Redo
    | Cancel_composition -> Cancel_composition
    | Read_snapshot -> Read_snapshot
  ;;

  let error_of_wire t = t
  let window t = t.Snapshot.window
  let node t = t.Snapshot.node
end
