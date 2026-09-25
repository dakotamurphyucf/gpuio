open Core

let max_text_bytes = 262_144
let history_budget_bytes = 2 * 1024 * 1024

module Mode = struct
  type t =
    | Single_line
    | Multiline
  [@@deriving equal, sexp_of]
end

let valid_utf8 text =
  Stdlib.String.is_valid_utf_8 text && not (String.contains text '\000')
;;

let validate_text ~mode text =
  if String.length text > max_text_bytes
  then Or_error.error_string "input text exceeds 256 KiB"
  else if not (valid_utf8 text)
  then Or_error.error_string "input text must be UTF-8 without NUL"
  else (
    match mode with
    | Mode.Single_line
      when String.exists text ~f:(fun char ->
             Char.equal char '\n' || Char.equal char '\r') ->
      Or_error.error_string "single-line input cannot contain newlines"
    | Single_line | Multiline -> Ok ())
;;

module Config = struct
  type t =
    { mode : Mode.t
    ; label : string
    ; placeholder : string
    ; read_only : bool
    ; disabled : bool
    ; submit_on_enter : bool
    ; auto_focus : bool
    ; min_rows : int
    ; max_rows : int
    }
  [@@deriving equal, sexp_of]

  let create
        ?(placeholder = "")
        ?(read_only = false)
        ?(disabled = false)
        ?(submit_on_enter = true)
        ?(auto_focus = false)
        ?(min_rows = 1)
        ?max_rows
        ~mode
        ~label
        ()
    =
    let max_rows =
      Option.value
        max_rows
        ~default:
          (match mode with
           | Mode.Single_line -> 1
           | Multiline -> 8)
    in
    if String.is_empty label || String.length label > 1024 || not (valid_utf8 label)
    then Or_error.error_string "input label must contain 1..1024 UTF-8 bytes without NUL"
    else if String.length placeholder > 4096 || not (valid_utf8 placeholder)
    then
      Or_error.error_string
        "input placeholder must contain at most 4096 UTF-8 bytes without NUL"
    else if min_rows < 1 || max_rows < min_rows || max_rows > 256
    then Or_error.error_string "input rows must satisfy 1 <= min_rows <= max_rows <= 256"
    else if Mode.equal mode Single_line && (min_rows <> 1 || max_rows <> 1)
    then Or_error.error_string "single-line input requires one row"
    else
      Ok
        { mode
        ; label
        ; placeholder
        ; read_only
        ; disabled
        ; submit_on_enter
        ; auto_focus
        ; min_rows
        ; max_rows
        }
  ;;

  let mode t = t.mode
end

module Revision = struct
  type t = int64 [@@deriving compare, equal, sexp_of]

  let of_int64 value =
    if Int64.(value < 0L)
    then Or_error.error_string "negative editor revision"
    else Ok value
  ;;

  let to_int64 t = t
end

module Selection = struct
  type t =
    { anchor : int
    ; head : int
    }
  [@@deriving equal, sexp_of]

  let create ~anchor ~head =
    if anchor < 0 || head < 0 || anchor > max_text_bytes || head > max_text_bytes
    then Or_error.error_string "selection offsets must be UTF-8 byte offsets in 0..262144"
    else Ok { anchor; head }
  ;;

  let anchor t = t.anchor
  let head t = t.head

  let validate t ~text =
    let boundary offset =
      offset <= String.length text
      && (offset = String.length text || Char.to_int text.[offset] land 0xc0 <> 0x80)
    in
    if Stdlib.String.is_valid_utf_8 text && boundary t.anchor && boundary t.head
    then Ok ()
    else Or_error.error_string "selection is outside text or splits a UTF-8 character"
  ;;
end

module Selection_policy = struct
  type t =
    | Start
    | End
    | Preserve
    | Select of Selection.t
  [@@deriving equal, sexp_of]
end

module Undo_policy = struct
  type t =
    | Record
    | Reset
  [@@deriving equal, sexp_of]
end

module Snapshot = struct
  type t =
    { window : Gpuio_protocol.Window_id.t
    ; node : Gpuio_protocol.Node_id.t
    ; revision : Revision.t
    ; text : string
    ; selection : Selection.t
    ; composition : Selection.t option
    ; focused : bool
    }
  [@@deriving equal, sexp_of]

  let text t = t.text
  let revision t = t.revision
  let selection t = t.selection
  let composition t = t.composition
  let focused t = t.focused
end

module Submission = struct
  type t = Snapshot.t [@@deriving equal, sexp_of]

  let text = Snapshot.text
  let revision = Snapshot.revision
end

module Command = struct
  type t =
    | Replace of
        { text : string
        ; selection : Selection_policy.t
        ; undo : Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Select of Selection.t
    | Focus
    | Undo
    | Redo
    | Submit
  [@@deriving equal, sexp_of]
end

module Command_error = struct
  type t =
    | Not_mounted
    | Closed
    | Stale_editor
    | Stale_revision
    | Composing
    | Invalid_selection
    | Limit_exceeded
    | Busy
    | Native_failure
    | Invalid_text
    | Focus_blocked
  [@@deriving equal, sexp_of]
end

module Event = struct
  type t =
    | Changed of Snapshot.t
    | Submitted of Submission.t
  [@@deriving equal, sexp_of]
end

module Expert = struct
  type config = Config.t =
    { mode : Mode.t
    ; label : string
    ; placeholder : string
    ; read_only : bool
    ; disabled : bool
    ; submit_on_enter : bool
    ; auto_focus : bool
    ; min_rows : int
    ; max_rows : int
    }

  let config t = t

  let snapshot ~window ~node ~revision ~text ~selection ~composition ~focused =
    let open Or_error.Let_syntax in
    let%bind () = validate_text ~mode:Multiline text in
    let%bind () = Selection.validate selection ~text in
    let%map () =
      match composition with
      | None -> Ok ()
      | Some range ->
        if Selection.anchor range > Selection.head range
        then Or_error.error_string "composition range must be ordered"
        else Selection.validate range ~text
    in
    { Snapshot.window; node; revision; text; selection; composition; focused }
  ;;

  let submission snapshot =
    if Option.is_some (Snapshot.composition snapshot)
    then Or_error.error_string "cannot submit during composition"
    else Ok snapshot
  ;;

  let submission_snapshot t = t
  let window (t : Snapshot.t) = t.window
  let node (t : Snapshot.t) = t.node

  let config_to_wire (t : Config.t) : Gpuio_protocol.Wire.Editor.Config.t =
    { label = t.label
    ; placeholder = t.placeholder
    ; read_only = t.read_only
    ; disabled = t.disabled
    ; submit_on_enter = t.submit_on_enter
    ; auto_focus = t.auto_focus
    ; min_rows = Int64.of_int t.min_rows
    ; max_rows = Int64.of_int t.max_rows
    }
  ;;

  let selection_to_wire (t : Selection.t) : Gpuio_protocol.Wire.Editor.Selection.t =
    { anchor = Int64.of_int t.anchor; head = Int64.of_int t.head }
  ;;

  let selection_of_wire (t : Gpuio_protocol.Wire.Editor.Selection.t) =
    if Int64.(t.anchor < 0L || t.head < 0L || t.anchor > 262_144L || t.head > 262_144L)
    then Or_error.error_string "invalid native selection offset"
    else
      Selection.create ~anchor:(Int64.to_int_exn t.anchor) ~head:(Int64.to_int_exn t.head)
  ;;

  let snapshot_of_wire ~window ~node (t : Gpuio_protocol.Wire.Editor.Snapshot.t) =
    let open Or_error.Let_syntax in
    let%bind revision = Revision.of_int64 t.revision in
    let%bind selection = selection_of_wire t.selection in
    let%bind composition =
      match t.composition with
      | None -> Ok None
      | Some range -> selection_of_wire range |> Or_error.map ~f:Option.some
    in
    snapshot
      ~window
      ~node
      ~revision
      ~text:t.text
      ~selection
      ~composition
      ~focused:t.focused
  ;;

  let command_to_wire : Command.t -> Gpuio_protocol.Wire.Editor.Command.t = function
    | Replace { text; selection; undo; if_revision } ->
      let selection : Gpuio_protocol.Wire.Editor.Selection_policy.t =
        match selection with
        | Start -> Start
        | End -> End
        | Preserve -> Preserve
        | Select range -> Select (selection_to_wire range)
      in
      let undo : Gpuio_protocol.Wire.Editor.Undo_policy.t =
        match undo with
        | Record -> Record
        | Reset -> Reset
      in
      Replace (text, selection, undo, Option.map if_revision ~f:Revision.to_int64)
    | Select range -> Select (selection_to_wire range)
    | Focus -> Focus
    | Undo -> Undo
    | Redo -> Redo
    | Submit -> Submit
  ;;

  let error_of_wire : Gpuio_protocol.Wire.Editor.Error.t -> Command_error.t = function
    | Not_mounted -> Not_mounted
    | Closed -> Closed
    | Stale_editor -> Stale_editor
    | Stale_revision -> Stale_revision
    | Composing -> Composing
    | Invalid_selection -> Invalid_selection
    | Limit_exceeded -> Limit_exceeded
    | Busy -> Busy
    | Native_failure -> Native_failure
    | Invalid_text -> Invalid_text
    | Focus_blocked -> Focus_blocked
  ;;
end
