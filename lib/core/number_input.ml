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

module Draft = struct
  type t = string [@@deriving equal, sexp_of]

  let of_string text =
    let%map.Or_error () = validate_draft text in
    text
  ;;

  let to_string t = t
end

module Appearance = struct
  type t =
    { gap : float
    ; button_width : float
    ; button_min_height : float
    ; stacked_button_min_height : float
    ; editor_padding : float
    ; border_width : float option
    ; frame_style : Style.t
    ; editor_style : Style.t
    ; decrement_style : Style.t
    ; increment_style : Style.t
    }
  [@@deriving equal, sexp_of]

  let properties : Style.Property.Name.t list =
    [ Background
    ; Foreground
    ; Opacity
    ; Border_color
    ; Shadows
    ; Top_left_radius
    ; Top_right_radius
    ; Bottom_left_radius
    ; Bottom_right_radius
    ; Font_size
    ; Font_family
    ; Font_weight
    ; Text_align
    ; Line_height
    ; White_space
    ; Text_overflow
    ; Line_clamp
    ; Text_decoration
    ]
  ;;

  let create
        ?(gap = 4.)
        ?(button_width = 24.)
        ?(button_min_height = 20.)
        ?(stacked_button_min_height = 16.)
        ?(editor_padding = 0.)
        ?border_width
        ?(frame_style = Style.empty)
        ?(editor_style = Style.empty)
        ?(decrement_style = Style.empty)
        ?(increment_style = Style.empty)
        ()
    =
    let bounded n low high = Float.is_finite n && Float.(n >= low && n <= high) in
    let open Or_error.Let_syntax in
    let%bind () =
      if
        List.for_all [ gap; editor_padding ] ~f:(fun n -> bounded n 0. 256.)
        && List.for_all
             [ button_width; button_min_height; stacked_button_min_height ]
             ~f:(fun n -> bounded n 1. 256.)
        && Option.for_all border_width ~f:(fun n -> bounded n 0. 64.)
      then Ok ()
      else Or_error.error_string "invalid numeric presentation geometry"
    in
    let%bind () =
      List.fold_result [ frame_style; editor_style ] ~init:() ~f:(fun () style ->
        Style.Expert.validate_scope style ~states:[ Base; Focused; Disabled ] ~properties)
    in
    let%bind () =
      List.fold_result [ decrement_style; increment_style ] ~init:() ~f:(fun () style ->
        Style.Expert.validate_scope
          style
          ~states:[ Base; Hovered; Pressed; Disabled ]
          ~properties)
    in
    let%map () =
      if
        List.sum
          (module Int)
          [ frame_style; editor_style; decrement_style; increment_style ]
          ~f:Style.Expert.declaration_count
        <= 128
      then Ok ()
      else Or_error.error_string "numeric appearance exceeds 128 declarations"
    in
    { gap
    ; button_width
    ; button_min_height
    ; stacked_button_min_height
    ; editor_padding
    ; border_width
    ; frame_style
    ; editor_style
    ; decrement_style
    ; increment_style
    }
  ;;

  let default = create () |> Or_error.ok_exn
end

module Step_mode = W.Step_mode

module Config = struct
  type t =
    { editing : W.Config.t
    ; step_mode : Step_mode.t
    }
  [@@deriving equal, sexp_of]

  let create
        ~domain
        ~label
        ?(placeholder = "")
        ?(increment_label = label ^ " increase")
        ?(decrement_label = label ^ " decrease")
        ?(step_mode = Step_mode.Native)
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
    then Ok { editing = t; step_mode }
    else Or_error.error_string "invalid numeric input configuration"
  ;;

  let step_mode t = t.step_mode
  let domain t = Numeric.Expert.of_wire t.editing.W.Config.domain |> Or_error.ok_exn
  let allows_empty t = t.editing.W.Config.allow_empty
  let is_disabled t = t.editing.W.Config.disabled
  let is_read_only t = t.editing.W.Config.read_only
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

module Step_request = struct
  type t =
    { id : int64
    ; direction : Numeric.Direction.t
    ; source : Source.t
    ; snapshot : Snapshot.t
    }
  [@@deriving equal, sexp_of]

  let snapshot t = t.snapshot
  let direction t = t.direction
  let source t = t.source
end

module Step_resolution = struct
  type t =
    | Apply of Value.t
    | Decline
  [@@deriving equal, sexp_of]
end

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Rejected of Rejection.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
    | Step_requested of Step_request.t
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
    | Resolve_step of Step_request.t * Step_resolution.t
  [@@deriving equal, sexp_of]
end

module Command_error = W.Error

module Expert = struct
  let appearance_to_wire (t : Appearance.t) ~theme =
    let open Or_error.Let_syntax in
    let%bind frame_style = Style.Expert.to_wire t.frame_style ~theme in
    let%bind editor_style = Style.Expert.to_wire t.editor_style ~theme in
    let%bind decrement_style = Style.Expert.to_wire t.decrement_style ~theme in
    let%map increment_style = Style.Expert.to_wire t.increment_style ~theme in
    ({ gap = t.gap
     ; button_width = t.button_width
     ; button_min_height = t.button_min_height
     ; stacked_button_min_height = t.stacked_button_min_height
     ; editor_padding = t.editor_padding
     ; border_width = t.border_width
     ; frame_style
     ; editor_style
     ; decrement_style
     ; increment_style
     }
     : Gpuio_protocol.Wire.Number_presentation.t)
  ;;

  let config_to_wire t = t.Config.editing
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
      | Cancelled (reason, _) -> Cancelled (reason, s)
      | Step_requested request ->
        Step_requested
          { Step_request.id = request.id
          ; direction =
              (match request.direction with
               | Increase -> Numeric.Direction.Increase
               | Decrease -> Decrease)
          ; source = request.source
          ; snapshot = s
          })
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
    | Resolve_step (request, decision) ->
      Resolve_step
        { request_id = request.id
        ; revision = Snapshot.revision request.snapshot
        ; value =
            (match decision with
             | Apply value -> Some value
             | Decline -> None)
        }
  ;;

  let command_snapshot = function
    | Command.Resolve_step (request, _) -> Some (Step_request.snapshot request)
    | Replace_draft _
    | Replace_value _
    | Select _
    | Focus
    | Undo
    | Redo
    | Commit
    | Cancel
    | Step _
    | Read_snapshot -> None
  ;;

  let error_of_wire t = t
  let window t = t.Snapshot.window
  let node t = t.Snapshot.node
end
