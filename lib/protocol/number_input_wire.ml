open Core

let max_draft_bytes = 4096

let valid_text text =
  String.length text <= max_draft_bytes
  && Stdlib.String.is_valid_utf_8 text
  && not
       (String.exists text ~f:(function
          | '\000' | '\n' | '\r' -> true
          | _ -> false))
;;

module Value = struct
  type t =
    | Empty
    | Number of float
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Empty -> true
    | Number value -> Float.is_finite value
  ;;

  let normalize t domain =
    match t with
    | Empty -> Some Empty
    | Number value ->
      Option.map (Numeric_wire.Domain.normalize domain value) ~f:(fun v -> Number v)
  ;;
end

module Step_controls = struct
  type t =
    | Hidden
    | Sides
    | Stacked
  [@@deriving bin_io, equal, sexp_of]
end

module Step_mode = struct
  type t =
    | Native
    | Application
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { domain : Numeric_wire.Domain.t
    ; label : string
    ; placeholder : string
    ; increment_label : string
    ; decrement_label : string
    ; step_controls : Step_controls.t
    ; allow_empty : bool
    ; disabled : bool
    ; read_only : bool
    ; auto_focus : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid_label text =
    valid_text text
    && not (String.is_empty (String.strip text ~drop:Numeric_wire.Draft.whitespace))
  ;;

  let valid t =
    Numeric_wire.Domain.valid t.domain
    && valid_label t.label
    && valid_text t.placeholder
    && valid_label t.increment_label
    && valid_label t.decrement_label
  ;;
end

module Selection = struct
  type t =
    { anchor : int64
    ; head : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.anchor >= 0L && t.head >= 0L && t.anchor <= 4096L && t.head <= 4096L)
  ;;

  let within t text =
    let boundary offset =
      Int64.(offset <= of_int (String.length text))
      && (Int64.equal offset (Int64.of_int (String.length text))
          || Char.to_int text.[Int64.to_int_exn offset] land 0xc0 <> 0x80)
    in
    valid t && Stdlib.String.is_valid_utf_8 text && boundary t.anchor && boundary t.head
  ;;
end

module Selection_policy = struct
  type t =
    | Start
    | End
    | Preserve
    | Select of Selection.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Start | End | Preserve -> true
    | Select s -> Selection.valid s
  ;;

  let within t text =
    match t with
    | Select s -> Selection.within s text
    | Start | End | Preserve -> true
  ;;
end

module Undo_policy = struct
  type t =
    | Record
    | Reset
  [@@deriving bin_io, equal, sexp_of]
end

module Snapshot = struct
  type t =
    { revision : int64
    ; domain : Numeric_wire.Domain.t
    ; draft : string
    ; committed : Value.t
    ; selection : Selection.t
    ; composition : Selection.t option
    ; focused : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let classification t = Numeric_wire.Draft.parse t.domain t.draft

  let valid t =
    Int64.(t.revision >= 0L)
    && Numeric_wire.Domain.valid t.domain
    && valid_text t.draft
    && Selection.within t.selection t.draft
    && Option.value_map t.composition ~default:true ~f:(fun s ->
      Int64.(s.anchor <= s.head) && Selection.within s t.draft)
    && Option.equal Value.equal (Value.normalize t.committed t.domain) (Some t.committed)
  ;;

  let settled t =
    Option.is_none t.composition
    &&
    match t.committed, classification t with
    | Value.Empty, Numeric_wire.Draft.Empty -> true
    | Number value, Valid draft -> Float.equal value draft
    | _ -> false
  ;;
end

module Source = struct
  type t =
    | Keyboard
    | Stepper
    | Accessibility
    | Programmatic
  [@@deriving bin_io, equal, sexp_of]
end

module Rejection = struct
  type t =
    | Empty_required
    | Incomplete
    | Syntax
    | Non_finite
    | Composing
  [@@deriving bin_io, equal, sexp_of]

  let matches t snapshot =
    match t, Snapshot.classification snapshot with
    | Composing, _ -> Option.is_some snapshot.composition
    | Empty_required, Empty
    | Incomplete, Incomplete
    | Syntax, Invalid Syntax
    | Non_finite, Invalid Non_finite -> Option.is_none snapshot.composition
    | _ -> false
  ;;
end

module Cancel_reason = struct
  type t =
    | Escape
    | Programmatic
  [@@deriving bin_io, equal, sexp_of]
end

module Step_request = struct
  type t =
    { id : int64
    ; direction : Numeric_wire.Direction.t
    ; source : Source.t
    ; snapshot : Snapshot.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.id > 0L)
    && Snapshot.valid t.snapshot
    && Option.is_none t.snapshot.composition
    &&
    match Snapshot.classification t.snapshot with
    | Empty | Valid _ | Out_of_range _ -> true
    | Incomplete | Invalid _ -> false
  ;;
end

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Rejected of Rejection.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
    | Step_requested of Step_request.t
  [@@deriving bin_io, equal, sexp_of]

  let snapshot = function
    | Observed s | Changed s | Committed (_, s) | Rejected (_, s) | Cancelled (_, s) -> s
    | Step_requested request -> request.snapshot
  ;;

  let valid t =
    let s = snapshot t in
    Snapshot.valid s
    &&
    match t with
    | Observed _ -> true
    | Step_requested request -> Step_request.valid request
    | Changed _ -> Int64.(s.revision > 0L)
    | Committed _ | Cancelled _ -> Int64.(s.revision > 0L) && Snapshot.settled s
    | Rejected (reason, _) -> Int64.(s.revision > 0L) && Rejection.matches reason s
  ;;
end

module Command = struct
  type t =
    | Replace_draft of
        { text : string
        ; selection : Selection_policy.t
        ; undo : Undo_policy.t
        ; if_revision : int64 option
        }
    | Replace_value of
        { value : Value.t
        ; selection : Selection_policy.t
        ; undo : Undo_policy.t
        ; if_revision : int64 option
        }
    | Select of Selection.t
    | Focus
    | Undo
    | Redo
    | Commit
    | Cancel
    | Step of Numeric_wire.Direction.t
    | Read_snapshot
    | Resolve_step of
        { request_id : int64
        ; revision : int64
        ; value : Value.t option
        }
  [@@deriving bin_io, equal, sexp_of]

  let valid_revision = Option.value_map ~default:true ~f:(fun r -> Int64.(r >= 0L))

  let valid = function
    | Resolve_step { request_id; revision; value } ->
      Int64.(request_id > 0L && revision >= 0L) && Option.for_all value ~f:Value.valid
    | Replace_draft { text; selection; undo = _; if_revision } ->
      valid_text text
      && Selection_policy.within selection text
      && valid_revision if_revision
    | Replace_value { value; selection; undo = _; if_revision } ->
      Value.valid value && Selection_policy.valid selection && valid_revision if_revision
    | Select s -> Selection.valid s
    | Focus | Undo | Redo | Commit | Cancel | Step _ | Read_snapshot -> true
  ;;
end

module Error = struct
  type t =
    | Not_mounted
    | Closed
    | Stale_input
    | Stale_revision
    | Composing
    | Invalid_selection
    | Limit_exceeded
    | Busy
    | Native_failure
    | Invalid_text
    | Invalid_value
    | Focus_blocked
    | Disabled
    | Read_only
    | Invalid_config
    | Rejected of Rejection.t
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Applied of Snapshot.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Applied s -> Snapshot.valid s
    | Failed _ -> true
  ;;
end
