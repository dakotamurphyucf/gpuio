open Core
module W = Gpuio_protocol.Number_input_wire

let max_draft_bytes = W.max_draft_bytes

let validate_draft text =
  if W.valid_text text
  then Ok ()
  else
    Or_error.error_string
      "numeric draft must be single-line UTF-8 without NUL, at most 4096 bytes"
;;

module Value = struct
  include W.Value

  let empty = Empty

  let of_float value =
    if Float.is_finite value
    then Ok (Number (if Float.equal value 0. then 0. else value))
    else Or_error.error_string "numeric value must be finite"
  ;;
end

module Step_controls = W.Step_controls

module Config = struct
  type t = W.Config.t [@@deriving equal, sexp_of]

  let create
        ~domain
        ~label
        ?(placeholder = "")
        ?(increment_label = label ^ " increase")
        ?(decrement_label = label ^ " decrease")
        ?(step_controls = Step_controls.Sides)
        ?(allow_empty = false)
        ?(disabled = false)
        ?(read_only = false)
        ?(auto_focus = false)
        ()
    =
    let t =
      { W.Config.domain = Numeric.Expert.to_wire domain
      ; label
      ; placeholder
      ; increment_label
      ; decrement_label
      ; step_controls
      ; allow_empty
      ; disabled
      ; read_only
      ; auto_focus
      }
    in
    if W.Config.valid t
    then Ok t
    else Or_error.error_string "invalid numeric input configuration"
  ;;

  let domain t = Numeric.Expert.of_wire t.W.Config.domain |> Or_error.ok_exn
  let allows_empty t = t.W.Config.allow_empty
  let is_disabled t = t.W.Config.disabled
  let is_read_only t = t.W.Config.read_only
end

module Revision = struct
  type t = int64 [@@deriving compare, equal, sexp_of]

  let of_int64 t =
    if Int64.(t >= 0L) then Ok t else Or_error.error_string "negative numeric revision"
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
  let domain t = Numeric.Expert.of_wire t.data.domain |> Or_error.ok_exn
  let draft t = t.data.draft
  let classification t = Numeric.Draft.parse (domain t) (draft t)
  let committed t = t.data.committed
  let selection t = selection_of_wire t.data.selection
  let composition t = Option.map t.data.composition ~f:selection_of_wire
  let focused t = t.data.focused
end

module Source = W.Source
module Rejection = W.Rejection
module Cancel_reason = W.Cancel_reason

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Rejected of Rejection.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
  [@@deriving equal, sexp_of]
end

module Command = struct
  type t =
    | Replace_draft of
        { text : string
        ; selection : Text_input.Selection_policy.t
        ; undo : Text_input.Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Replace_value of
        { value : Value.t
        ; selection : Text_input.Selection_policy.t
        ; undo : Text_input.Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Select of Text_input.Selection.t
    | Focus
    | Undo
    | Redo
    | Commit
    | Cancel
    | Step of Numeric.Direction.t
    | Read_snapshot
  [@@deriving equal, sexp_of]
end

module Command_error = W.Error

module Expert = struct
  let config_to_wire t = t
  let value_to_wire t = t

  let snapshot_of_wire ~window ~node data =
    if W.Snapshot.valid data
    then Ok { Snapshot.window; node; data }
    else Or_error.error_string "invalid numeric snapshot"
  ;;

  let event_of_wire ~window ~node event =
    if not (W.Event.valid event)
    then Or_error.error_string "invalid numeric event"
    else (
      let%map.Or_error s = snapshot_of_wire ~window ~node (W.Event.snapshot event) in
      match event with
      | Observed _ -> Event.Observed s
      | Changed _ -> Changed s
      | Committed (source, _) -> Committed (source, s)
      | Rejected (reason, _) -> Rejected (reason, s)
      | Cancelled (reason, _) -> Cancelled (reason, s))
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
    | Replace_draft { text; selection; undo = policy; if_revision } ->
      Replace_draft
        { text; selection = selection_policy selection; undo = undo policy; if_revision }
    | Replace_value { value; selection; undo = policy; if_revision } ->
      Replace_value
        { value; selection = selection_policy selection; undo = undo policy; if_revision }
    | Select s -> Select (selection s)
    | Focus -> Focus
    | Undo -> Undo
    | Redo -> Redo
    | Commit -> Commit
    | Cancel -> Cancel
    | Step Increase -> Step Increase
    | Step Decrease -> Step Decrease
    | Read_snapshot -> Read_snapshot
  ;;

  let error_of_wire t = t
  let window t = t.Snapshot.window
  let node t = t.Snapshot.node
end
