open Core

let max_input_bytes = 4096

module Alphabet = struct
  type t =
    | Digits
    | Ascii_alphanumeric
  [@@deriving bin_io, equal, sexp_of]
end

module Policy = struct
  type t =
    { length : int
    ; alphabet : Alphabet.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = t.length >= 1 && t.length <= 32
end

module Input_error = struct
  type t =
    | Invalid_policy
    | Input_too_large
    | Invalid_utf8
    | Unexpected_character of { byte_offset : int }
    | Too_long
    | Invalid_value
    | Invalid_selection
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Unexpected_character { byte_offset } ->
      byte_offset >= 0 && byte_offset < max_input_bytes
    | Invalid_policy
    | Input_too_large
    | Invalid_utf8
    | Too_long
    | Invalid_value
    | Invalid_selection -> true
  ;;
end

let allowed alphabet code =
  (code >= 48 && code <= 57)
  ||
  match alphabet with
  | Alphabet.Digits -> false
  | Ascii_alphanumeric -> (code >= 65 && code <= 90) || (code >= 97 && code <= 122)
;;

let canonical policy text =
  Policy.valid policy
  && String.length text <= policy.length
  && String.for_all text ~f:(fun ch -> allowed policy.alphabet (Char.to_int ch))
;;

let full_width code =
  if
    (code >= 0xff10 && code <= 0xff19)
    || (code >= 0xff21 && code <= 0xff3a)
    || (code >= 0xff41 && code <= 0xff5a)
  then code - 0xfee0
  else code
;;

let separator = function
  | 9 | 10 | 11 | 12 | 13 | 32 | 45 -> true
  | _ -> false
;;

let normalize policy ~paste text =
  let open Input_error in
  if not (Policy.valid policy)
  then Error Invalid_policy
  else if String.length text > max_input_bytes
  then Error Input_too_large
  else if not (Stdlib.String.is_valid_utf_8 text)
  then Error Invalid_utf8
  else (
    let buffer = Buffer.create policy.length in
    let rec loop offset =
      if offset = String.length text
      then Ok (Buffer.contents buffer)
      else (
        let decoded = Stdlib.String.get_utf_8_uchar text offset in
        let code = Stdlib.Uchar.to_int (Stdlib.Uchar.utf_decode_uchar decoded) in
        let next = offset + Stdlib.Uchar.utf_decode_length decoded in
        if paste && separator code
        then loop next
        else if not (allowed policy.alphabet (full_width code))
        then Error (Unexpected_character { byte_offset = offset })
        else if Buffer.length buffer = policy.length
        then Error Too_long
        else (
          Buffer.add_char buffer (Char.of_int_exn (full_width code));
          loop next))
    in
    loop 0)
;;

let replace policy ~value ~anchor ~head ~paste text =
  let open Input_error in
  if not (Policy.valid policy)
  then Error Invalid_policy
  else if not (canonical policy value)
  then Error Invalid_value
  else if
    anchor < 0 || head < 0 || anchor > String.length value || head > String.length value
  then Error Invalid_selection
  else
    let open Result.Let_syntax in
    let%bind inserted = normalize policy ~paste text in
    let start = Int.min anchor head
    and stop = Int.max anchor head in
    if paste && String.is_empty inserted
    then Ok (value, anchor, head)
    else if String.length value - (stop - start) + String.length inserted > policy.length
    then Error Too_long
    else
      Ok
        ( String.prefix value start ^ inserted ^ String.drop_prefix value stop
        , start + String.length inserted
        , start + String.length inserted )
;;

let valid_draft text =
  String.length text <= max_input_bytes
  && Stdlib.String.is_valid_utf_8 text
  && not
       (String.exists text ~f:(function
          | '\000' | '\r' | '\n' -> true
          | _ -> false))
;;

module Config = struct
  type t =
    { policy : Policy.t
    ; label : string
    ; masked : bool
    ; disabled : bool
    ; read_only : bool
    ; auto_focus : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Policy.valid t.policy
    && valid_draft t.label
    && not
         (String.is_empty
            (String.strip t.label ~drop:(function
               | ' ' | '\t' | '\r' | '\n' | '\011' | '\012' -> true
               | _ -> false)))
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

  let within t text =
    match t with
    | Start | End | Preserve -> true
    | Select selection -> Selection.within selection text
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
    ; policy : Policy.t
    ; value : string
    ; draft : string
    ; selection : Selection.t
    ; composition : Selection.t option
    ; focused : bool
    ; can_undo : bool
    ; can_redo : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.revision >= 0L)
    && canonical t.policy t.value
    && valid_draft t.draft
    && Selection.within t.selection t.draft
    &&
    match t.composition with
    | None -> String.equal t.value t.draft
    | Some marked ->
      Int64.(marked.anchor < marked.head) && Selection.within marked t.draft
  ;;

  let is_complete t =
    Option.is_none t.composition && String.length t.value = t.policy.length
  ;;
end

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Complete of Snapshot.t
    | Rejected of Input_error.t * Snapshot.t
  [@@deriving bin_io, equal, sexp_of]

  let snapshot = function
    | Observed s | Changed s | Complete s | Rejected (_, s) -> s
  ;;

  let valid t =
    let s = snapshot t in
    Snapshot.valid s
    &&
    match t with
    | Observed _ -> true
    | Changed _ -> Int64.(s.revision > 0L)
    | Complete _ -> Int64.(s.revision > 0L) && Snapshot.is_complete s
    | Rejected (reason, _) ->
      Int64.(s.revision > 0L) && Input_error.valid reason && Option.is_none s.composition
  ;;
end

module Command = struct
  type t =
    | Replace of
        { value : string
        ; selection : Selection_policy.t
        ; undo : Undo_policy.t
        ; if_revision : int64 option
        }
    | Clear of
        { undo : Undo_policy.t
        ; if_revision : int64 option
        }
    | Select of Selection.t
    | Focus
    | Undo
    | Redo
    | Cancel_composition
    | Read_snapshot
  [@@deriving bin_io, equal, sexp_of]

  let valid_revision =
    Option.value_map ~default:true ~f:(fun revision -> Int64.(revision >= 0L))
  ;;

  let valid = function
    | Replace { value; selection; undo = _; if_revision } ->
      canonical { Policy.length = 32; alphabet = Ascii_alphanumeric } value
      && Selection_policy.within selection value
      && valid_revision if_revision
    | Clear { undo = _; if_revision } -> valid_revision if_revision
    | Select selection -> Selection.valid selection
    | Focus | Undo | Redo | Cancel_composition | Read_snapshot -> true
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
    | Invalid_value
    | Focus_blocked
    | Disabled
    | Read_only
    | Invalid_config
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
